#!/usr/bin/env python3
"""Check identity examples in ADR 0075 documents.

Situational guard for ADR 0075. Remove it when that ADR is superseded.
Check Bech32 checksums, key length, padding, and nprofile TLV structure.
This check makes no network request and changes no file.
"""

import argparse
from pathlib import Path
import re
import sys


CHARSET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
TOKENS = re.compile(r"\b(?:npub|nprofile)1[a-z0-9]+\b", re.I)
EXPECTED_KEY = re.compile(r"\|\s*`([0-9a-f]{64})`\s*\|", re.I)
INVALID_EXAMPLE = "npub1notavalidkey"
NEGATIVE_SECTIONS = {"C09", "Prefix Validity Without Content Validity"}
DEFAULT_FILES = (
    "docs/schema/adr-0075-metadata-example-corpus.md",
    "docs/schema/adr-0075-identity-syntax-contract.md",
)


def decode(value):
    """Return the prefix and decoded bytes, or reject an invalid encoding."""
    if value.lower() != value and value.upper() != value:
        raise ValueError("Mixed letter case")
    prefix, encoded = value.lower().rsplit("1", 1)
    if len(encoded) < 6:
        raise ValueError("Missing checksum")
    try:
        words = [CHARSET.index(character) for character in encoded]
    except ValueError as error:
        raise ValueError("Invalid Bech32 character") from error
    expanded = [ord(c) >> 5 for c in prefix] + [0] + [ord(c) & 31 for c in prefix]
    checksum = 1
    generators = (0x3B6A57B2, 0x26508E6D, 0x1EA119FA, 0x3D4233DD, 0x2A1462B3)
    for word in expanded + words:
        top = checksum >> 25
        checksum = ((checksum & 0x1FFFFFF) << 5) ^ word
        for bit, generator in enumerate(generators):
            if (top >> bit) & 1:
                checksum ^= generator
    if checksum != 1:
        raise ValueError("Invalid Bech32 checksum")
    accumulator = 0
    bits = 0
    payload = bytearray()
    for word in words[:-6]:
        accumulator = ((accumulator << 5) | word) & 0xFFF
        bits += 5
        while bits >= 8:
            bits -= 8
            payload.append((accumulator >> bits) & 0xFF)
    if bits >= 5 or (accumulator << (8 - bits)) & 0xFF:
        raise ValueError("Invalid padding")
    return prefix, bytes(payload)


def public_key(value):
    """Return the public key after checking the NIP-19 payload shape."""
    prefix, payload = decode(value)
    if prefix == "npub":
        if len(payload) != 32:
            raise ValueError("The npub key must contain 32 bytes")
        return payload
    if prefix != "nprofile":
        raise ValueError("Unsupported identity prefix")
    offset = 0
    keys = []
    while offset < len(payload):
        if offset + 2 > len(payload):
            raise ValueError("Incomplete TLV header")
        kind, length = payload[offset:offset + 2]
        offset += 2
        end = offset + length
        if end > len(payload):
            raise ValueError("Incomplete TLV value")
        if kind == 0:
            keys.append(payload[offset:end])
        elif kind == 1:
            try:
                payload[offset:end].decode("ascii")
            except UnicodeDecodeError as error:
                raise ValueError("Invalid relay text") from error
        offset = end
    if len(keys) != 1 or len(keys[0]) != 32:
        raise ValueError("The nprofile must contain one 32-byte public key")
    return keys[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="*", type=Path)
    args = parser.parse_args()
    paths = args.files or [Path(name) for name in DEFAULT_FILES]
    errors = []
    positive = 0
    negative = 0
    vectors = 0
    for path in paths:
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeError) as error:
            errors.append(f"{path}: Cannot read input: {error}")
            continue
        section = ""
        for number, line in enumerate(lines, 1):
            heading = re.match(r"^#{1,6} (.+)$", line)
            if heading:
                section = heading[1]
            for value in TOKENS.findall(line):
                is_negative = value == INVALID_EXAMPLE and section in NEGATIVE_SECTIONS
                try:
                    key = public_key(value)
                except ValueError as error:
                    if is_negative:
                        negative += 1
                    else:
                        errors.append(f"{path}:{number}: {error}: {value}")
                else:
                    if is_negative:
                        errors.append(f"{path}:{number}: The invalid example passed")
                    else:
                        positive += 1
                        expected = EXPECTED_KEY.search(line)
                        if expected:
                            vectors += 1
                            if key.hex() != expected[1].lower():
                                errors.append(f"{path}:{number}: The decoded key differs from the expected key")
    if not positive:
        errors.append("No valid identity example was checked")
    if not args.files and negative != 2:
        errors.append("Expected the corpus and syntax contract to retain one invalid example each")
    if not args.files and vectors < 3:
        errors.append("Expected at least three vectors with decoded public keys")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Green: {positive} valid examples, {negative} intentional invalid examples, and {vectors} decoded vectors")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
