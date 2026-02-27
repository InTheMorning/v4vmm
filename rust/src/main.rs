mod cli;
mod config;
mod db;
mod rss;
mod id3;

use anyhow::Result;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cfg_path = config::config_path()?;
    let cfg = config::load_config(&cfg_path)?;
    config::ensure_dirs(&cfg)?;
    let mut db = db::open_db(&cfg)?;

    let cmd = cli::parse_args()?;
    match cmd {
        cli::Command::ShowConfig => cli::cmd_show_config(&cfg, &cfg_path),

        cli::Command::Id3Dump { path } => id3::cmd_id3_dump(&cfg, &path),

        cli::Command::Subscribe { feed_url } => rss::cmd_subscribe(&cfg, &mut db, &feed_url),

        cli::Command::RssDump { feed_url } => rss::cmd_rss_dump(&feed_url),

        cli::Command::Help => {
            cli::print_help();
            Ok(())
        }
    }
}
