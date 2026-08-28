use std::fs;
use std::io::Cursor;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use gpui::{Image, ImageFormat};
use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use lru::LruCache;
use sha2::{Digest, Sha256};

use crate::media::image_type;
use crate::remote_media;

const DEFAULT_HOT_CAPACITY: usize = 128;
const DEFAULT_MAX_DIM: u32 = 512;
const DEFAULT_MAX_DISK_BYTES: u64 = 500 * 1024 * 1024;

/// Shared image cache. Clone the `Arc` cheaply across entities.
pub struct ImageCache {
    cache_dir: PathBuf,
    hot: Mutex<LruCache<String, Arc<Image>>>,
    /// Static first-frame cache for animated formats (GIFs). Keyed by URL.
    /// For non-animated images, `fetch_static_blocking` falls through to `hot`.
    static_hot: Mutex<LruCache<String, Arc<Image>>>,
    max_dimension: u32,
    max_disk_bytes: u64,
    writes_since_eviction: Mutex<u32>,
}

impl ImageCache {
    pub fn new(cache_dir: PathBuf) -> Arc<Self> {
        Self::with_capacity(
            cache_dir,
            DEFAULT_HOT_CAPACITY,
            DEFAULT_MAX_DIM,
            DEFAULT_MAX_DISK_BYTES,
        )
    }

    pub fn with_capacity(
        cache_dir: PathBuf,
        hot_capacity: usize,
        max_dimension: u32,
        max_disk_bytes: u64,
    ) -> Arc<Self> {
        let capacity = NonZeroUsize::new(hot_capacity.max(1)).unwrap();
        let cache = Arc::new(Self {
            cache_dir: cache_dir.clone(),
            hot: Mutex::new(LruCache::new(capacity)),
            static_hot: Mutex::new(LruCache::new(capacity)),
            max_dimension,
            max_disk_bytes,
            writes_since_eviction: Mutex::new(0),
        });
        // Prune on startup without blocking UI.
        std::thread::spawn(move || {
            let _ = evict_oldest(&cache_dir, max_disk_bytes);
        });
        cache
    }

    /// Fast path — hot in-memory hit for the animated variant. UI-thread safe.
    pub fn peek(&self, url: &str) -> Option<Arc<Image>> {
        let mut hot = self.hot.lock().ok()?;
        hot.get(url).cloned()
    }

    /// Fast path — hot in-memory hit for the static variant. For non-animated
    /// formats this falls through to the regular `peek` (same image).
    pub fn peek_static(&self, url: &str) -> Option<Arc<Image>> {
        if let Ok(mut s) = self.static_hot.lock() {
            if let Some(img) = s.get(url).cloned() {
                return Some(img);
            }
        }
        self.peek(url)
    }

    /// Blocking fetch: memory → disk → network. Returns the animated variant
    /// for GIFs. Call from a background executor.
    pub fn fetch_blocking(&self, url: &str) -> Option<Arc<Image>> {
        if let Some(img) = self.peek(url) {
            return Some(img);
        }

        let (bytes, mime) = self.read_or_download(url).ok().flatten()?;
        let format = ImageFormat::from_mime_type(&mime)?;
        let image = Arc::new(Image::from_bytes(format, bytes));

        if let Ok(mut hot) = self.hot.lock() {
            hot.put(url.to_string(), image.clone());
        }
        Some(image)
    }

    /// Blocking fetch for the static preview. For GIFs, returns the first frame
    /// re-encoded as JPEG. For other formats, identical to `fetch_blocking`.
    pub fn fetch_static_blocking(&self, url: &str) -> Option<Arc<Image>> {
        if let Some(img) = self.peek_static(url) {
            return Some(img);
        }

        let (bytes, mime) = self.read_or_download(url).ok().flatten()?;

        if mime == "image/gif" {
            let preview = match gif_first_frame_jpeg(&bytes, self.max_dimension) {
                Ok(p) => p,
                Err(_) => {
                    // Fall back to animated if we can't decode a preview.
                    let image = Arc::new(Image::from_bytes(ImageFormat::Gif, bytes));
                    if let Ok(mut hot) = self.hot.lock() {
                        hot.put(url.to_string(), image.clone());
                    }
                    return Some(image);
                }
            };
            let image = Arc::new(Image::from_bytes(ImageFormat::Jpeg, preview));
            if let Ok(mut s) = self.static_hot.lock() {
                s.put(url.to_string(), image.clone());
            }
            Some(image)
        } else {
            let format = ImageFormat::from_mime_type(&mime)?;
            let image = Arc::new(Image::from_bytes(format, bytes));
            if let Ok(mut hot) = self.hot.lock() {
                hot.put(url.to_string(), image.clone());
            }
            Some(image)
        }
    }

    fn read_or_download(&self, url: &str) -> Result<Option<(Vec<u8>, String)>> {
        fs::create_dir_all(&self.cache_dir)?;
        let path = self.cache_dir.join(cache_key(url));

        if let Some(entry) = read_cache_entry(&path)? {
            return Ok(Some(entry));
        }

        let response = remote_media::fetch(url, "thumbnail")?;
        let declared = remote_media::declared_content_type(&response);
        let bytes = response.bytes()?.to_vec();
        if bytes.is_empty() {
            return Ok(None);
        }
        // Bytes first, declared type second. A thumbnail served as
        // application/octet-stream is still a thumbnail.
        let Some(mime_type) = image_type::classify(&bytes, declared.as_deref()) else {
            return Ok(None);
        };

        // Pre-scale static images. Leave GIFs alone to preserve animation.
        let (final_bytes, final_mime) = if mime_type == "image/gif" {
            (bytes, mime_type)
        } else {
            match downscale(&bytes, self.max_dimension) {
                Ok(Some(scaled)) => (scaled, "image/jpeg".to_string()),
                _ => (bytes, mime_type),
            }
        };

        write_cache_entry(&path, &final_mime, &final_bytes)?;
        self.maybe_evict();
        Ok(Some((final_bytes, final_mime)))
    }

    /// Trigger disk eviction every ~64 writes to amortize the scan cost.
    fn maybe_evict(&self) {
        let Ok(mut count) = self.writes_since_eviction.lock() else {
            return;
        };
        *count += 1;
        if *count < 64 {
            return;
        }
        *count = 0;
        drop(count);
        let dir = self.cache_dir.clone();
        let max_bytes = self.max_disk_bytes;
        std::thread::spawn(move || {
            let _ = evict_oldest(&dir, max_bytes);
        });
    }
}

fn cache_key(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hasher.finalize();
    let mut out = String::with_capacity(32);
    for b in hash.iter().take(16) {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn read_cache_entry(path: &Path) -> Result<Option<(Vec<u8>, String)>> {
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if data.is_empty() {
        return Ok(None);
    }
    let mime_len = data[0] as usize;
    if data.len() < 1 + mime_len {
        return Ok(None);
    }
    let mime = std::str::from_utf8(&data[1..1 + mime_len])?.to_string();
    if !mime.starts_with("image/") {
        return Ok(None);
    }
    let bytes = data[1 + mime_len..].to_vec();
    if bytes.is_empty() {
        return Ok(None);
    }
    Ok(Some((bytes, mime)))
}

fn write_cache_entry(path: &Path, mime: &str, bytes: &[u8]) -> Result<()> {
    if mime.len() > 255 {
        return Err(anyhow!("mime type too long: {} bytes", mime.len()));
    }
    let mut out = Vec::with_capacity(1 + mime.len() + bytes.len());
    out.push(mime.len() as u8);
    out.extend_from_slice(mime.as_bytes());
    out.extend_from_slice(bytes);
    fs::write(path, out)?;
    Ok(())
}

/// Walk the cache directory; if total size exceeds `max_bytes`, delete oldest
/// entries by mtime until under the limit. Best-effort — errors are swallowed.
fn evict_oldest(dir: &Path, max_bytes: u64) -> Result<()> {
    if max_bytes == 0 || !dir.exists() {
        return Ok(());
    }
    let mut entries: Vec<(PathBuf, std::time::SystemTime, u64)> = Vec::new();
    let mut total: u64 = 0;
    for entry in fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let size = meta.len();
        total = total.saturating_add(size);
        entries.push((entry.path(), mtime, size));
    }
    if total <= max_bytes {
        return Ok(());
    }
    // Sort oldest-first, delete until under limit.
    entries.sort_by_key(|(_, mtime, _)| *mtime);
    for (path, _, size) in entries {
        if total <= max_bytes {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(size);
        }
    }
    Ok(())
}

/// Decode the first frame of an animated GIF and re-encode as JPEG, scaled to
/// fit within `max_dim` on the longer side.
fn gif_first_frame_jpeg(bytes: &[u8], max_dim: u32) -> Result<Vec<u8>> {
    let decoder = GifDecoder::new(Cursor::new(bytes))?;
    let mut frames = decoder.into_frames();
    let frame = frames
        .next()
        .ok_or_else(|| anyhow!("gif has no frames"))??;
    let buf = frame.into_buffer();
    let dyn_img = image::DynamicImage::ImageRgba8(buf);
    let scaled = if dyn_img.width() > max_dim || dyn_img.height() > max_dim {
        dyn_img.resize(max_dim, max_dim, image::imageops::FilterType::Triangle)
    } else {
        dyn_img
    };
    // JPEG doesn't support alpha — flatten by converting to RGB8.
    let rgb = scaled.to_rgb8();
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(rgb)
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Jpeg)?;
    Ok(out)
}

fn downscale(bytes: &[u8], max_dim: u32) -> Result<Option<Vec<u8>>> {
    let img = image::load_from_memory(bytes)?;
    if img.width() <= max_dim && img.height() <= max_dim {
        return Ok(None);
    }
    let resized = img.resize(max_dim, max_dim, image::imageops::FilterType::Triangle);
    let mut buf = Vec::new();
    resized.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Jpeg)?;
    Ok(Some(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    const TEST_IMAGE_BYTES: &[u8] = include_bytes!("../assets/music_network_logo.png");

    fn serve_once(content_type: &'static str, body: &'static [u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(headers.as_bytes()).expect("write headers");
            stream.write_all(body).expect("write body");
        });
        format!("http://{addr}/cover.png")
    }

    fn serve_redirect_then_image() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("listener addr");
        std::thread::spawn(move || {
            let (mut redirect_stream, _) = listener.accept().expect("accept redirect request");
            let mut request = [0_u8; 1024];
            let _ = redirect_stream.read(&mut request);
            // The real feed case: a Location containing raw spaces.
            let location = format!("http://{addr}/Assets/front cover.png");
            let response = format!(
                "HTTP/1.1 301 Moved Permanently\r\nLocation: {location}\r\nContent-Type: text/html\r\nContent-Length: 8\r\nConnection: close\r\n\r\nredirect"
            );
            redirect_stream
                .write_all(response.as_bytes())
                .expect("write redirect response");

            let (mut image_stream, _) = listener.accept().expect("accept image request");
            let mut request = [0_u8; 1024];
            let _ = image_stream.read(&mut request);
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                TEST_IMAGE_BYTES.len()
            );
            image_stream
                .write_all(headers.as_bytes())
                .expect("write headers");
            image_stream
                .write_all(TEST_IMAGE_BYTES)
                .expect("write body");
        });
        format!("http://{addr}/cover.png")
    }

    fn cache(dir: &Path) -> Arc<ImageCache> {
        ImageCache::with_capacity(dir.join("thumbnails"), 2, 512, 1024 * 1024)
    }

    /// Pre-download artwork for a redirecting feed. This is the White Triangles
    /// case: without redirect resolution the thumbnail silently never appears.
    #[test]
    fn thumbnail_survives_a_redirect_with_spaces_in_location() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache = cache(temp.path());

        let image = cache.fetch_blocking(&serve_redirect_then_image());

        assert!(image.is_some(), "redirected artwork should still resolve");
    }

    /// Independent of redirects: plenty of hosts serve real images under a
    /// generic content type. The bytes decide, not the header.
    #[test]
    fn thumbnail_resolves_image_served_under_a_non_image_content_type() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache = cache(temp.path());

        let image = cache.fetch_blocking(&serve_once("application/octet-stream", TEST_IMAGE_BYTES));

        assert!(
            image.is_some(),
            "a real PNG is a PNG regardless of its declared type"
        );
    }

    #[test]
    fn markup_body_yields_no_image_and_no_cache_entry() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache_dir = temp.path().join("thumbnails");
        let cache = ImageCache::with_capacity(cache_dir.clone(), 2, 512, 1024 * 1024);

        let image = cache.fetch_blocking(&serve_once("text/html", b"<html>moved</html>"));

        assert!(image.is_none(), "markup must not decode as artwork");
        let cached_entries = fs::read_dir(&cache_dir)
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(cached_entries, 0, "a failed fetch must not be cached");
    }
}
