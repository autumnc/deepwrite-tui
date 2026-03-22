use std::fs;

use deepwrite::browser::entries::{list_entries, EntryKind};
use tempfile::TempDir;

#[test]
fn test_filters_non_markdown_files() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("readme.md"), "").unwrap();
    fs::write(tmp.path().join("notes.txt"), "").unwrap();
    fs::write(tmp.path().join("image.png"), "").unwrap();
    fs::write(tmp.path().join("code.rs"), "").unwrap();

    let entries = list_entries(tmp.path(), false).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

    assert!(names.contains(&"readme.md"));
    assert!(names.contains(&"notes.txt"));
    assert!(!names.contains(&"image.png"));
    assert!(!names.contains(&"code.rs"));
}

#[test]
fn test_directories_first_then_alphabetical() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join("zebra")).unwrap();
    fs::create_dir(tmp.path().join("alpha")).unwrap();
    fs::write(tmp.path().join("beta.md"), "").unwrap();
    fs::write(tmp.path().join("aardvark.txt"), "").unwrap();

    let entries = list_entries(tmp.path(), false).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

    // Directories first (alpha, zebra), then files (aardvark.txt, beta.md)
    assert_eq!(names, vec!["alpha", "zebra", "aardvark.txt", "beta.md"]);

    // Verify kinds
    assert_eq!(entries[0].kind, EntryKind::Directory);
    assert_eq!(entries[1].kind, EntryKind::Directory);
    assert_eq!(entries[2].kind, EntryKind::File);
    assert_eq!(entries[3].kind, EntryKind::File);
}

#[test]
fn test_hides_dotfiles_by_default() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join(".git")).unwrap();
    fs::write(tmp.path().join(".env"), "").unwrap();
    fs::write(tmp.path().join("visible.md"), "").unwrap();

    let entries = list_entries(tmp.path(), false).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

    assert!(!names.contains(&".git"));
    assert!(!names.contains(&".env"));
    assert!(names.contains(&"visible.md"));
}

#[test]
fn test_shows_dotfiles_when_enabled() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join(".hidden_dir")).unwrap();
    fs::write(tmp.path().join(".secret.md"), "").unwrap();
    fs::write(tmp.path().join("visible.md"), "").unwrap();

    let entries = list_entries(tmp.path(), true).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

    assert!(names.contains(&".hidden_dir"));
    assert!(names.contains(&".secret.md"));
    assert!(names.contains(&"visible.md"));
}
