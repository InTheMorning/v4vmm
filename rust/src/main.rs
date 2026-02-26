use anyhow::{Context, Result};
use id3::{Tag, TagLike, Version};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
struct Id3Info {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    other: HashMap<String, Vec<String>>, // frame_id -> stringified contents
}

fn write_basic_id3(path: &Path, title: &str, artist: &str, album: &str) -> Result<()> {
    // Read existing tag if present; otherwise create a new one
    let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());

    tag.set_title(title);
    tag.set_artist(artist);
    tag.set_album(album);

    // Writes/updates an ID3 tag in the file (creates one if missing)
    tag.write_to_path(path, Version::Id3v24)
        .with_context(|| format!("failed to write id3 tag to {}", path.display()))?;

    Ok(())
}

fn read_basic_id3(path: &Path) -> Result<Id3Info> {
    let tag = Tag::read_from_path(path)
        .with_context(|| format!("failed to read id3 tag from {}", path.display()))?;

    let mut info = Id3Info {
        title: tag.title().map(|s| s.to_string()),
        artist: tag.artist().map(|s| s.to_string()),
        album: tag.album().map(|s| s.to_string()),
        other: HashMap::new(),
    };

    for frame in tag.frames() {
        let id = frame.id().to_string();

        // Skip the “basic” frames we already surface separately
        if id == "TIT2" || id == "TPE1" || id == "TALB" {
            continue;
        }

        // Dump the frame content in a generic way (good for "everything else")
        // Later we can decode specific frames (APIC artwork, TXXX custom fields, etc.)
        info.other
            .entry(id)
            .or_insert_with(Vec::new)
            .push(format!("{:?}", frame.content()));
    }

    Ok(info)
}

fn main() -> Result<()> {
    let path = Path::new("test.mp3");
    if !path.exists() {
        anyhow::bail!("Put a copy of an MP3 in this folder named test.mp3");
    }

    
    let info = read_basic_id3(path)?;
    println!("TITLE : {:?}", info.title);
    println!("ARTIST: {:?}", info.artist);
    println!("ALBUM : {:?}", info.album);
    println!("OTHER frames: {}", info.other.len());
    
    // Optional: print other frames
    for (frame_id, values) in &info.other {
        println!("  {} -> {}", frame_id, values.len());
    }
/**    write_basic_id3(path, "Test Title from Rust", "Test Artist", "Test Album")?;
    println!("✅ Wrote ID3 tags to {}", path.display());
*/
    Ok(())
}