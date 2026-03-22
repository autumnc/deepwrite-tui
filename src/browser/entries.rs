use std::fs;
use std::path::Path;

/// The kind of a directory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    File,
}

/// A single entry in a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
}

/// List entries in `dir`, filtered to only show:
/// - Directories
/// - Files with `.md` or `.txt` extensions
///
/// Dotfiles (names starting with `.`) are hidden unless `show_hidden` is true.
///
/// Results are sorted: directories first, then alphabetical (case-insensitive).
pub fn list_entries(dir: &Path, show_hidden: bool) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();

    for dir_entry in fs::read_dir(dir)? {
        let dir_entry = dir_entry?;
        let file_name = dir_entry.file_name();
        let name = file_name.to_string_lossy().to_string();

        // Hide dotfiles unless show_hidden is true
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let file_type = dir_entry.file_type()?;

        if file_type.is_dir() {
            entries.push(Entry {
                name,
                kind: EntryKind::Directory,
            });
        } else if file_type.is_file() {
            // Only include .md and .txt files
            let lower = name.to_lowercase();
            if lower.ends_with(".md") || lower.ends_with(".txt") {
                entries.push(Entry {
                    name,
                    kind: EntryKind::File,
                });
            }
        }
    }

    // Sort: directories first, then alphabetical (case-insensitive)
    entries.sort_by(|a, b| {
        let kind_order = |k: &EntryKind| match k {
            EntryKind::Directory => 0,
            EntryKind::File => 1,
        };
        kind_order(&a.kind)
            .cmp(&kind_order(&b.kind))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}
