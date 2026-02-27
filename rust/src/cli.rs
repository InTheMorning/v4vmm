// src/cli.rs
use anyhow::{anyhow, Result};
use std::env;
use std::path::{Path, PathBuf};

use crate::config::Config;

#[derive(Debug)]
pub enum Command {
    ShowConfig,
    Id3Dump { path: PathBuf },
    Subscribe { feed_url: String },
    RssDump { feed_url: String },
    Help,
}

pub fn parse_args() -> Result<Command> {
    let mut args = env::args().skip(1); // skip program name

    match args.next().as_deref() {
        Some("show-config") => Ok(Command::ShowConfig),

        Some("id3-dump") => {
            let p = args
                .next()
                .ok_or_else(|| anyhow!("id3-dump requires a path argument"))?;
            Ok(Command::Id3Dump {
                path: PathBuf::from(p),
            })
        }

        Some("subscribe") => {
            let u = args
                .next()
                .ok_or_else(|| anyhow!("subscribe requires a feed URL"))?;
            Ok(Command::Subscribe { feed_url: u })
        }

        Some("rss-dump") => {
            let u = args
                .next()
                .ok_or_else(|| anyhow!("rss-dump requires a feed URL"))?;
            Ok(Command::RssDump { feed_url: u })
        }

        Some("help") | Some("-h") | Some("--help") | None => Ok(Command::Help),

        Some(other) => Err(anyhow!("unknown command: {other} (try: v4vmm help)")),
    }
}

pub fn print_help() {
    println!(
        r#"v4vmm (early prototype)

Usage:
  v4vmm show-config
  v4vmm id3-dump <path-to-mp3>
  v4vmm subscribe <feed-url>
  v4vmm rss-dump <feed-url>
"#
    );
}

pub fn cmd_show_config(cfg: &Config, cfg_path: &Path) -> Result<()> {
    println!("Config path : {}", cfg_path.display());
    println!("music_dir   : {}", cfg.music_dir.display());
    println!("db_path     : {}", cfg.db_path.display());
    Ok(())
}
