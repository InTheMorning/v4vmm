use std::path::PathBuf;

use v4vmm::cli::{self, Command};

#[test]
fn parse_show_config() {
    let cmd = cli::parse_args_from(["show-config"]).unwrap();
    assert_eq!(cmd, Command::ShowConfig);
}

#[test]
fn parse_id3_dump_path() {
    let cmd = cli::parse_args_from(["id3-dump", "/tmp/song.mp3"]).unwrap();
    assert_eq!(
        cmd,
        Command::Id3Dump {
            path: PathBuf::from("/tmp/song.mp3"),
        }
    );
}

#[test]
fn parse_subscribe_url() {
    let cmd = cli::parse_args_from(["subscribe", "https://example.com/feed.xml"]).unwrap();
    assert_eq!(
        cmd,
        Command::Subscribe {
            feed_url: "https://example.com/feed.xml".into(),
        }
    );
}

#[test]
fn parse_help_when_no_args() {
    let cmd = cli::parse_args_from(Vec::<String>::new()).unwrap();
    assert_eq!(cmd, Command::Help);
}

#[test]
fn reject_unknown_command() {
    let err = cli::parse_args_from(["wat"]).unwrap_err();
    assert!(err.to_string().contains("unknown command"));
}
