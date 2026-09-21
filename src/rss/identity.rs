//! ADR 0075 typed RSS identity evidence and NIP-19 validation.

use std::fmt;

#[derive(Clone, PartialEq, Eq)]
pub enum NostrIdentity {
    PublicKey {
        original: String,
        public_key: [u8; 32],
    },
    Profile {
        original: String,
        public_key: [u8; 32],
        relay_hints: Vec<String>,
        tlvs: Vec<NostrTlv>,
    },
}

impl NostrIdentity {
    pub fn scheme(&self) -> &'static str {
        match self {
            Self::PublicKey { .. } => "nostr_npub",
            Self::Profile { .. } => "nostr_nprofile",
        }
    }

    pub fn original(&self) -> &str {
        match self {
            Self::PublicKey { original, .. } | Self::Profile { original, .. } => original,
        }
    }
}

impl fmt::Debug for NostrIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NostrIdentity")
            .field("scheme", &self.scheme())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NostrTlv {
    pub field_type: u8,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityDecodeError {
    InvalidCharacter,
    MixedCase,
    MissingSeparator,
    InvalidChecksum,
    InvalidDataCharacter,
    InvalidPadding,
    InvalidPublicKeyLength,
    TruncatedTlvHeader,
    TruncatedTlvValue,
    MissingProfilePublicKey,
    DuplicateProfilePublicKey,
    InvalidProfilePublicKeyLength,
    InvalidRelayText,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityValidation {
    Valid(NostrIdentity),
    UnsupportedEncoding,
    Malformed(IdentityDecodeError),
}

pub fn validate_nostr_identity(candidate: &str) -> IdentityValidation {
    match decode_bech32(candidate) {
        Ok((hrp, payload)) if hrp == "npub" => decode_public_key(candidate, &payload),
        Ok((hrp, payload)) if hrp == "nprofile" => decode_profile(candidate, &payload),
        Ok(_) => IdentityValidation::UnsupportedEncoding,
        Err(error) => IdentityValidation::Malformed(error),
    }
}

fn decode_public_key(original: &str, payload: &[u8]) -> IdentityValidation {
    let Ok(public_key) = <[u8; 32]>::try_from(payload) else {
        return IdentityValidation::Malformed(IdentityDecodeError::InvalidPublicKeyLength);
    };
    IdentityValidation::Valid(NostrIdentity::PublicKey {
        original: original.to_owned(),
        public_key,
    })
}

fn decode_profile(original: &str, payload: &[u8]) -> IdentityValidation {
    let mut offset = 0;
    let mut public_key = None;
    let mut relay_hints = Vec::new();
    let mut tlvs = Vec::new();
    while offset < payload.len() {
        let Some(&field_type) = payload.get(offset) else {
            return IdentityValidation::Malformed(IdentityDecodeError::TruncatedTlvHeader);
        };
        let Some(&length) = payload.get(offset + 1) else {
            return IdentityValidation::Malformed(IdentityDecodeError::TruncatedTlvHeader);
        };
        offset += 2;
        let end = match offset.checked_add(usize::from(length)) {
            Some(end) if end <= payload.len() => end,
            _ => return IdentityValidation::Malformed(IdentityDecodeError::TruncatedTlvValue),
        };
        let value = &payload[offset..end];
        match field_type {
            0 => {
                if public_key.is_some() {
                    return IdentityValidation::Malformed(
                        IdentityDecodeError::DuplicateProfilePublicKey,
                    );
                }
                let Ok(key) = <[u8; 32]>::try_from(value) else {
                    return IdentityValidation::Malformed(
                        IdentityDecodeError::InvalidProfilePublicKeyLength,
                    );
                };
                public_key = Some(key);
            }
            1 => match std::str::from_utf8(value) {
                Ok(relay) if relay.is_ascii() => relay_hints.push(relay.to_owned()),
                _ => return IdentityValidation::Malformed(IdentityDecodeError::InvalidRelayText),
            },
            _ => {}
        }
        tlvs.push(NostrTlv {
            field_type,
            value: value.to_vec(),
        });
        offset = end;
    }
    let Some(public_key) = public_key else {
        return IdentityValidation::Malformed(IdentityDecodeError::MissingProfilePublicKey);
    };
    IdentityValidation::Valid(NostrIdentity::Profile {
        original: original.to_owned(),
        public_key,
        relay_hints,
        tlvs,
    })
}

fn decode_bech32(value: &str) -> Result<(String, Vec<u8>), IdentityDecodeError> {
    if !value.bytes().all(|byte| (33..=126).contains(&byte)) {
        return Err(IdentityDecodeError::InvalidCharacter);
    }
    let has_lower = value.bytes().any(|byte| byte.is_ascii_lowercase());
    let has_upper = value.bytes().any(|byte| byte.is_ascii_uppercase());
    if has_lower && has_upper {
        return Err(IdentityDecodeError::MixedCase);
    }
    let value = value.to_ascii_lowercase();
    let Some(separator) = value.rfind('1') else {
        return Err(IdentityDecodeError::MissingSeparator);
    };
    if separator == 0 || value.len().saturating_sub(separator + 1) < 6 {
        return Err(IdentityDecodeError::MissingSeparator);
    }
    let hrp = &value[..separator];
    let data = value[separator + 1..]
        .bytes()
        .map(bech32_value)
        .collect::<Result<Vec<_>, _>>()?;
    if bech32_polymod(
        hrp.bytes()
            .map(|byte| byte >> 5)
            .chain(std::iter::once(0))
            .chain(hrp.bytes().map(|byte| byte & 0x1f))
            .chain(data.iter().copied()),
    ) != 1
    {
        return Err(IdentityDecodeError::InvalidChecksum);
    }
    let payload_length = data.len() - 6;
    let payload = convert_bits(&data[..payload_length])?;
    Ok((hrp.to_owned(), payload))
}

fn bech32_value(byte: u8) -> Result<u8, IdentityDecodeError> {
    const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    CHARSET
        .iter()
        .position(|candidate| *candidate == byte)
        .and_then(|index| u8::try_from(index).ok())
        .ok_or(IdentityDecodeError::InvalidDataCharacter)
}

fn bech32_polymod(values: impl Iterator<Item = u8>) -> u32 {
    let mut checksum = 1_u32;
    for value in values {
        let top = checksum >> 25;
        checksum = (checksum & 0x1ff_ffff) << 5 ^ u32::from(value);
        for (index, generator) in [
            0x3b6a_57b2,
            0x2650_8e6d,
            0x1ea1_19fa,
            0x3d42_33dd,
            0x2a14_62b3,
        ]
        .iter()
        .enumerate()
        {
            if (top >> index) & 1 == 1 {
                checksum ^= generator;
            }
        }
    }
    checksum
}

fn convert_bits(values: &[u8]) -> Result<Vec<u8>, IdentityDecodeError> {
    let mut accumulator = 0_u32;
    let mut bits = 0_u32;
    let mut output = Vec::with_capacity(values.len().saturating_mul(5) / 8);
    for value in values {
        accumulator = (accumulator << 5) | u32::from(*value);
        bits += 5;
        while bits >= 8 {
            bits -= 8;
            output.push(((accumulator >> bits) & 0xff) as u8);
        }
    }
    if bits >= 5 || ((accumulator << (8 - bits)) & 0xff) != 0 {
        return Err(IdentityDecodeError::InvalidPadding);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0075_rss_decodes_checked_public_key_vector() {
        let value = "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6";
        let IdentityValidation::Valid(NostrIdentity::PublicKey { public_key, .. }) =
            validate_nostr_identity(value)
        else {
            panic!("the checked npub vector must validate");
        };
        assert_eq!(
            hex(&public_key),
            "3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d"
        );
    }

    #[test]
    fn adr_0075_rss_decodes_checked_profile_vector() {
        let value = "nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gpp4mhxue69uhhytnc9e3k7mgpz4mhxue69uhkg6nzv9ejuumpv34kytnrdaksjlyr9p";
        let IdentityValidation::Valid(NostrIdentity::Profile {
            public_key,
            relay_hints,
            ..
        }) = validate_nostr_identity(value)
        else {
            panic!("the checked nprofile vector must validate");
        };
        assert_eq!(
            hex(&public_key),
            "3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d"
        );
        assert_eq!(relay_hints, ["wss://r.x.com", "wss://djbas.sadkb.com"]);
    }

    #[test]
    fn adr_0075_rss_rejects_malformed_candidate() {
        assert!(matches!(
            validate_nostr_identity("npub1notavalidkey"),
            IdentityValidation::Malformed(_)
        ));
    }

    #[test]
    fn adr_0075_rss_rejects_checksum_case_and_unsupported_encodings() {
        let value = "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6";
        let mut wrong_checksum = value.to_owned();
        wrong_checksum.pop();
        wrong_checksum.push('q');
        assert!(matches!(
            validate_nostr_identity(&wrong_checksum),
            IdentityValidation::Malformed(IdentityDecodeError::InvalidChecksum)
        ));
        let mut mixed_case = value.to_owned();
        mixed_case.replace_range(..1, "N");
        assert!(matches!(
            validate_nostr_identity(&mixed_case),
            IdentityValidation::Malformed(IdentityDecodeError::MixedCase)
        ));
        assert!(matches!(
            validate_nostr_identity("nsec1notavalidkey"),
            IdentityValidation::Malformed(_) | IdentityValidation::UnsupportedEncoding
        ));
        assert!(matches!(
            validate_nostr_identity(&value.to_ascii_uppercase()),
            IdentityValidation::Valid(NostrIdentity::PublicKey { .. })
        ));
    }

    fn hex(value: &[u8]) -> String {
        value.iter().map(|byte| format!("{byte:02x}")).collect()
    }
    #[test]
    fn adr_0075_rss_rejects_checksum_valid_padding_lengths_and_profile_boundaries() {
        // Fixtures use an independent encoder and the ADR 0075 Python reference decoder.
        for (label, candidate, expected) in [
            ("short npub", "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6g5f6fxy8", IdentityDecodeError::InvalidPublicKeyLength),
            ("long npub", "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsqaacg5m", IdentityDecodeError::InvalidPublicKeyLength),
            ("empty profile", "nprofile1p2zuer", IdentityDecodeError::MissingProfilePublicKey),
            ("unknown TLV without key", "nprofile1qgp24wcvx9pqk", IdentityDecodeError::MissingProfilePublicKey),
            ("truncated initial header", "nprofile1qqqsnhxh", IdentityDecodeError::TruncatedTlvHeader),
            ("truncated trailing header", "nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gzpvqklu", IdentityDecodeError::TruncatedTlvHeader),
            ("truncated value", "nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gzqv4qmcpscd", IdentityDecodeError::TruncatedTlvValue),
            ("profile key length 0", "nprofile1qqqqxkww4j", IdentityDecodeError::InvalidProfilePublicKeyLength),
            ("profile key length 31", "nprofile1qq0sqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqrny624", IdentityDecodeError::InvalidProfilePublicKeyLength),
            ("profile key length 33", "nprofile1qqssqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqmgczld", IdentityDecodeError::InvalidProfilePublicKeyLength),
            ("duplicate required key", "nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gqyqalp33lewf5vdq847t6te0wvnags0gs0mu72kz8938tn24wlfze68e3ux3", IdentityDecodeError::DuplicateProfilePublicKey),
            ("non ASCII relay", "nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gpqtp6jlr7r4p", IdentityDecodeError::InvalidRelayText),
            ("invalid UTF8 relay", "nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gpq8lsfplutk", IdentityDecodeError::InvalidRelayText),
            ("excess padding", "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsqqy3jnr8", IdentityDecodeError::InvalidPadding),
            ("nonzero padding", "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkw3eyr0ng", IdentityDecodeError::InvalidPadding),
            ("Bech32m", "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkws3w8ktc", IdentityDecodeError::InvalidChecksum),
        ] {
            assert_eq!(validate_nostr_identity(candidate), IdentityValidation::Malformed(expected), "{label}");
        }
    }

    #[test]
    fn adr_0075_rss_rejects_invalid_ascii_alphabet_and_separator() {
        for (candidate, error) in [
            ("npub1é", IdentityDecodeError::InvalidCharacter),
            ("npub1 qqqqqq", IdentityDecodeError::InvalidCharacter),
            ("", IdentityDecodeError::MissingSeparator),
            ("npub", IdentityDecodeError::MissingSeparator),
            ("1qqqqqq", IdentityDecodeError::MissingSeparator),
            ("npub1qq", IdentityDecodeError::MissingSeparator),
            ("npub1bbbbbb", IdentityDecodeError::InvalidDataCharacter),
        ] {
            assert_eq!(
                validate_nostr_identity(candidate),
                IdentityValidation::Malformed(error)
            );
        }
    }

    #[test]
    fn adr_0075_rss_preserves_unknown_tlvs_repeated_relays_and_original_profile() {
        let candidate = "nprofile1pypszqsrqqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gpqamhxue69uhkzqg8waehxw309asusqqyc4n7y";
        let IdentityValidation::Valid(NostrIdentity::Profile {
            original,
            public_key,
            relay_hints,
            tlvs,
        }) = validate_nostr_identity(candidate)
        else {
            panic!("the complete profile must validate");
        };
        assert_eq!(original, candidate);
        assert_eq!(
            hex(&public_key),
            "3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d"
        );
        assert_eq!(relay_hints, ["wss://a", "wss://a"]);
        assert_eq!(
            tlvs,
            vec![
                NostrTlv {
                    field_type: 9,
                    value: vec![1, 2, 3]
                },
                NostrTlv {
                    field_type: 0,
                    value: public_key.to_vec()
                },
                NostrTlv {
                    field_type: 1,
                    value: b"wss://a".to_vec()
                },
                NostrTlv {
                    field_type: 1,
                    value: b"wss://a".to_vec()
                },
                NostrTlv {
                    field_type: 200,
                    value: vec![]
                },
            ]
        );
    }

    #[test]
    fn adr_0075_rss_returns_unsupported_for_a_valid_other_encoding() {
        assert_eq!(
            validate_nostr_identity(
                "nsec180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsgyumg0"
            ),
            IdentityValidation::UnsupportedEncoding
        );
    }

    #[test]
    fn adr_0075_rss_decoder_rejects_truncation_without_panics() {
        let value = "nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gpp4mhxue69uhhytnc9e3k7mgpz4mhxue69uhkg6nzv9ejuumpv34kytnrdaksjlyr9p";
        for end in 0..value.len() {
            assert!(!matches!(
                validate_nostr_identity(&value[..end]),
                IdentityValidation::Valid(_)
            ));
        }
        let uppercase = value.to_ascii_uppercase();
        let IdentityValidation::Valid(identity) = validate_nostr_identity(&uppercase) else {
            panic!("uppercase profile must validate");
        };
        assert_eq!(identity.original(), uppercase);
        assert_eq!(identity.scheme(), "nostr_nprofile");
    }
}
