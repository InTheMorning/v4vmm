mod common;

use v4vmm::config;

#[test]
fn load_config_creates_default_file_and_parses_it() {
    let (_cfg, dir) = common::test_config();
    let cfg_path = dir.path().join("config.toml");

    let loaded = config::load_config(&cfg_path).unwrap();

    assert_eq!(loaded.music_dir, std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("V4VMusic"));
    assert!(cfg_path.exists(), "expected config file to be created");
}

#[test]
fn ensure_dirs_creates_music_and_db_parent() {
    let (cfg, _dir) = common::test_config();

    assert!(!cfg.music_dir.exists());
    assert!(!cfg.db_path.parent().unwrap().exists());

    config::ensure_dirs(&cfg).unwrap();

    assert!(cfg.music_dir.exists(), "music_dir should exist");
    assert!(cfg.db_path.parent().unwrap().exists(), "db parent should exist");
}
