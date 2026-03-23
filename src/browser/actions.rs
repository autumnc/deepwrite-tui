use std::fs;
use std::path::{Component, Path};

use anyhow::{Context, Result};

fn validate_entry_name<'a>(name: &'a str, label: &str) -> Result<&'a str> {
    let name = name.trim();
    anyhow::ensure!(!name.is_empty(), "{label} cannot be empty");

    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(name),
        _ => anyhow::bail!("{label} cannot contain path separators"),
    }
}

/// Create a new file in the given directory.
///
/// If `name` does not end with `.md` or `.txt`, `.md` is appended automatically.
pub fn create_file(dir: &Path, name: &str) -> Result<()> {
    let name = validate_entry_name(name, "File name")?;

    let file_name = if name.ends_with(".md") || name.ends_with(".txt") {
        name.to_string()
    } else {
        format!("{}.md", name)
    };

    let path = dir.join(&file_name);
    anyhow::ensure!(!path.exists(), "\"{}\" already exists", file_name);

    fs::write(&path, "").with_context(|| format!("Failed to create file: {}", path.display()))?;
    Ok(())
}

/// Create a new directory inside the given parent directory.
pub fn create_directory(dir: &Path, name: &str) -> Result<()> {
    let name = validate_entry_name(name, "Directory name")?;

    let path = dir.join(name);
    anyhow::ensure!(!path.exists(), "\"{}\" already exists", name);

    fs::create_dir(&path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    Ok(())
}

/// Rename a file or directory.
pub fn rename_entry(dir: &Path, old_name: &str, new_name: &str) -> Result<()> {
    let new_name = validate_entry_name(new_name, "New name")?;

    let old_path = dir.join(old_name);
    let new_path = dir.join(new_name);

    anyhow::ensure!(old_path.exists(), "\"{}\" does not exist", old_name);
    anyhow::ensure!(!new_path.exists(), "\"{}\" already exists", new_name);

    fs::rename(&old_path, &new_path)
        .with_context(|| format!("Failed to rename \"{}\" to \"{}\"", old_name, new_name))?;
    Ok(())
}

/// Delete a file or directory (recursively for directories).
pub fn delete_entry(dir: &Path, name: &str) -> Result<()> {
    let path = dir.join(name);
    anyhow::ensure!(path.exists(), "\"{}\" does not exist", name);

    if path.is_dir() {
        fs::remove_dir_all(&path)
            .with_context(|| format!("Failed to delete directory: {}", path.display()))?;
    } else {
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete file: {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_file_adds_md_extension() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "notes").unwrap();
        assert!(tmp.path().join("notes.md").exists());
    }

    #[test]
    fn test_create_file_keeps_md_extension() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "notes.md").unwrap();
        assert!(tmp.path().join("notes.md").exists());
    }

    #[test]
    fn test_create_file_keeps_txt_extension() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "notes.txt").unwrap();
        assert!(tmp.path().join("notes.txt").exists());
    }

    #[test]
    fn test_create_file_duplicate_fails() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "notes").unwrap();
        assert!(create_file(tmp.path(), "notes").is_err());
    }

    #[test]
    fn test_create_file_empty_name_fails() {
        let tmp = TempDir::new().unwrap();
        assert!(create_file(tmp.path(), "").is_err());
        assert!(create_file(tmp.path(), "  ").is_err());
    }

    #[test]
    fn test_create_file_rejects_nested_paths() {
        let tmp = TempDir::new().unwrap();
        assert!(create_file(tmp.path(), "../notes").is_err());
        assert!(create_file(tmp.path(), "nested/notes").is_err());
    }

    #[test]
    fn test_create_directory() {
        let tmp = TempDir::new().unwrap();
        create_directory(tmp.path(), "subdir").unwrap();
        assert!(tmp.path().join("subdir").is_dir());
    }

    #[test]
    fn test_create_directory_duplicate_fails() {
        let tmp = TempDir::new().unwrap();
        create_directory(tmp.path(), "subdir").unwrap();
        assert!(create_directory(tmp.path(), "subdir").is_err());
    }

    #[test]
    fn test_create_directory_rejects_nested_paths() {
        let tmp = TempDir::new().unwrap();
        assert!(create_directory(tmp.path(), "../subdir").is_err());
        assert!(create_directory(tmp.path(), "nested/subdir").is_err());
    }

    #[test]
    fn test_rename_entry() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "old.md").unwrap();
        rename_entry(tmp.path(), "old.md", "new.md").unwrap();
        assert!(!tmp.path().join("old.md").exists());
        assert!(tmp.path().join("new.md").exists());
    }

    #[test]
    fn test_rename_to_existing_fails() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "a.md").unwrap();
        create_file(tmp.path(), "b.md").unwrap();
        assert!(rename_entry(tmp.path(), "a.md", "b.md").is_err());
    }

    #[test]
    fn test_rename_rejects_nested_paths() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "old.md").unwrap();
        assert!(rename_entry(tmp.path(), "old.md", "../new.md").is_err());
        assert!(rename_entry(tmp.path(), "old.md", "nested/new.md").is_err());
    }

    #[test]
    fn test_delete_file() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "delete_me.md").unwrap();
        delete_entry(tmp.path(), "delete_me.md").unwrap();
        assert!(!tmp.path().join("delete_me.md").exists());
    }

    #[test]
    fn test_delete_directory() {
        let tmp = TempDir::new().unwrap();
        create_directory(tmp.path(), "delete_dir").unwrap();
        // Put a file inside
        create_file(&tmp.path().join("delete_dir"), "inner.md").unwrap();
        delete_entry(tmp.path(), "delete_dir").unwrap();
        assert!(!tmp.path().join("delete_dir").exists());
    }

    #[test]
    fn test_delete_nonexistent_fails() {
        let tmp = TempDir::new().unwrap();
        assert!(delete_entry(tmp.path(), "nope.md").is_err());
    }
}
