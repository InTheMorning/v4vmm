//! Image type classification.
//!
//! ADR 0056. One owner for "what image is this?". Previously this lived private
//! inside `audio_tags`, so the thumbnail cache and cover-art lookup could not
//! reach it and fell back to trusting the response `Content-Type`. A real JPEG
//! served as `application/octet-stream` produced no artwork at all.
//!
//! Precedence for remote responses is always: bytes first, declared `image/*`
//! second, otherwise no image. A declared type is a hint; the bytes are the
//! fact.

use std::path::Path;

/// Image type from magic bytes.
pub fn from_bytes(data: &[u8]) -> Option<String> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png".into())
    } else if data.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg".into())
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some("image/gif".into())
    } else if data.len() >= 12 && data.starts_with(b"RIFF") && data[8..12] == *b"WEBP" {
        Some("image/webp".into())
    } else {
        None
    }
}

/// Image type from a declared content type, e.g. a `Content-Type` header.
///
/// Returns `None` for any non-image type, so a `text/html` redirect landing page
/// cannot pass itself off as artwork.
pub fn from_declared(value: &str) -> Option<String> {
    let mime_type = value.split(';').next()?.trim().to_ascii_lowercase();
    mime_type.starts_with("image/").then_some(mime_type)
}

/// Image type for a local file path.
///
/// Extensions are trusted here and only here: the operator controls local file
/// paths. Remote URL suffixes are never trusted (ADR 0056).
pub fn from_path(path: &Path) -> Option<String> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg".into()),
        "png" => Some("image/png".into()),
        "gif" => Some("image/gif".into()),
        "webp" => Some("image/webp".into()),
        _ => None,
    }
}

/// The precedence rule for remote image responses: bytes, then declared type.
pub fn classify(data: &[u8], declared: Option<&str>) -> Option<String> {
    from_bytes(data).or_else(|| declared.and_then(from_declared))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_supported_magic_bytes() {
        assert_eq!(
            from_bytes(b"\x89PNG\r\n\x1a\nimage bytes"),
            Some("image/png".into())
        );
        assert_eq!(from_bytes(b"\xff\xd8\xffjpeg"), Some("image/jpeg".into()));
        assert_eq!(from_bytes(b"GIF89a gif"), Some("image/gif".into()));
        assert_eq!(from_bytes(b"RIFF1234WEBP"), Some("image/webp".into()));
        assert_eq!(from_bytes(b"<html>moved</html>"), None);
    }

    #[test]
    fn declared_type_accepts_only_image_types() {
        assert_eq!(
            from_declared("image/jpeg; charset=binary"),
            Some("image/jpeg".into())
        );
        assert_eq!(from_declared("text/html"), None);
        assert_eq!(from_declared("application/octet-stream"), None);
    }

    #[test]
    fn bytes_win_over_a_wrong_declared_type() {
        assert_eq!(
            classify(b"\x89PNG\r\n\x1a\nbytes", Some("image/jpeg")),
            Some("image/png".into())
        );
    }

    #[test]
    fn declared_type_is_the_fallback_for_unrecognized_bytes() {
        assert_eq!(
            classify(b"unrecognized", Some("image/jpeg")),
            Some("image/jpeg".into())
        );
    }

    /// The pre-download artwork fix: a valid image served under a non-image
    /// content type is still an image.
    #[test]
    fn sniffs_images_served_under_a_non_image_content_type() {
        assert_eq!(
            classify(b"\xff\xd8\xffjpeg bytes", Some("application/octet-stream")),
            Some("image/jpeg".into())
        );
    }

    #[test]
    fn markup_body_is_not_an_image() {
        assert_eq!(classify(b"<html>moved</html>", Some("text/html")), None);
    }
}
