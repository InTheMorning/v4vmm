mod common;

use std::fs;

use v4vmm::config;

#[test]
fn load_config_creates_default_file_and_parses_it() {
    let (_cfg, dir) = common::test_config();
    let cfg_path = dir.path().join("config.toml");

    let loaded = config::load_config(&cfg_path).unwrap();

    assert_eq!(
        loaded.music_dir,
        std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("V4VMusic")
    );
    assert!(cfg_path.exists(), "expected config file to be created");
    assert_eq!(
        config::load_musicindex_endpoint(&cfg_path).unwrap(),
        "https://api.musicindex.org"
    );
}

#[test]
fn ensure_dirs_creates_music_and_db_parent() {
    let (cfg, _dir) = common::test_config();

    assert!(!cfg.music_dir.exists());
    assert!(!cfg.db_path.parent().unwrap().exists());

    config::ensure_dirs(&cfg).unwrap();

    assert!(cfg.music_dir.exists(), "music_dir should exist");
    assert!(
        cfg.db_path.parent().unwrap().exists(),
        "db parent should exist"
    );
}

#[test]
fn musicindex_endpoint_defaults_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    fs::write(
        &cfg_path,
        r#"
music_dir = "/tmp/v4vmm-test/music"
db_path = "/tmp/v4vmm-test/db.sqlite"
"#,
    )
    .unwrap();

    assert_eq!(
        config::load_musicindex_endpoint(&cfg_path).unwrap(),
        "https://api.musicindex.org"
    );
}

#[test]
fn musicindex_endpoint_save_normalizes_bare_host() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    fs::write(
        &cfg_path,
        r#"
music_dir = "/tmp/v4vmm-test/music"
db_path = "/tmp/v4vmm-test/db.sqlite"
"#,
    )
    .unwrap();

    let saved = config::save_musicindex_endpoint(&cfg_path, "api.musicindex.org/").unwrap();

    assert_eq!(saved, "https://api.musicindex.org");
    assert_eq!(
        config::load_musicindex_endpoint(&cfg_path).unwrap(),
        "https://api.musicindex.org"
    );
}
