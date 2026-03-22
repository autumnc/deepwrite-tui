use std::path::{Path, PathBuf};

use super::entries::{list_entries, Entry, EntryKind};

/// Navigates a directory tree, tracking the current directory, its entries,
/// and the currently selected entry.
#[derive(Debug)]
pub struct Navigator {
    pub current_dir: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub show_hidden: bool,
}

impl Navigator {
    /// Create a new Navigator rooted at `dir`.
    pub fn new(dir: &Path, show_hidden: bool) -> Self {
        let current_dir = dir.to_path_buf();
        let entries = list_entries(&current_dir, show_hidden).unwrap_or_default();
        Self {
            current_dir,
            entries,
            selected: 0,
            show_hidden,
        }
    }

    /// Refresh the entries list from the current directory.
    pub fn refresh(&mut self) {
        self.entries = list_entries(&self.current_dir, self.show_hidden).unwrap_or_default();
        // Clamp selected index
        if self.entries.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.entries.len() {
            self.selected = self.entries.len() - 1;
        }
    }

    /// Move the selection up by one.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move the selection down by one.
    pub fn move_down(&mut self) {
        if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    /// Return a reference to the currently selected entry, if any.
    pub fn selected_entry(&self) -> Option<&Entry> {
        self.entries.get(self.selected)
    }

    /// Toggle visibility of hidden files and refresh.
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh();
    }

    /// Enter the selected directory (if it is a directory).
    /// Returns true if the directory was entered.
    pub fn enter_selected(&mut self) -> bool {
        if let Some(entry) = self.selected_entry() {
            if entry.kind == EntryKind::Directory {
                let new_dir = self.current_dir.join(&entry.name);
                self.current_dir = new_dir;
                self.selected = 0;
                self.refresh();
                return true;
            }
        }
        false
    }

    /// Go up to the parent directory.
    /// Returns true if we moved up.
    pub fn go_up(&mut self) -> bool {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.selected = 0;
            self.refresh();
            true
        } else {
            false
        }
    }

    /// Return indices of entries whose names fuzzy-match the query.
    ///
    /// Fuzzy match: all query characters appear in order in the entry name
    /// (case-insensitive).
    pub fn filter_entries(&self, query: &str) -> Vec<usize> {
        let query_lower: Vec<char> = query.to_lowercase().chars().collect();
        if query_lower.is_empty() {
            return (0..self.entries.len()).collect();
        }

        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                let name_lower = entry.name.to_lowercase();
                let mut chars = name_lower.chars();
                for qc in &query_lower {
                    if !chars.any(|c| c == *qc) {
                        return false;
                    }
                }
                true
            })
            .map(|(i, _)| i)
            .collect()
    }
}
