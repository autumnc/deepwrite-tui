use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::file_io;

pub struct AutoSave {
    pub path: Option<PathBuf>,
    pub delay: Duration,
    pub last_edit: Option<Instant>,
    pub last_save_content: String,
    pub dirty: bool,
}

impl AutoSave {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            path: None,
            delay: Duration::from_millis(delay_ms),
            last_edit: None,
            last_save_content: String::new(),
            dirty: false,
        }
    }

    pub fn mark_edited(&mut self) {
        self.last_edit = Some(Instant::now());
        self.dirty = true;
    }

    pub fn should_save(&self) -> bool {
        if !self.dirty {
            return false;
        }
        match self.last_edit {
            Some(t) => t.elapsed() >= self.delay,
            None => false,
        }
    }

    /// Save if content has actually changed since the last save.
    pub fn save(&mut self, content: &str) -> anyhow::Result<()> {
        if content == self.last_save_content {
            self.dirty = false;
            return Ok(());
        }
        if let Some(ref path) = self.path {
            file_io::save_file(path, content)?;
            self.last_save_content = content.to_string();
            self.dirty = false;
            self.last_edit = None;
        }
        Ok(())
    }

    /// Force-save, bypassing the debounce timer.
    pub fn force_save(&mut self, content: &str) -> anyhow::Result<()> {
        if let Some(ref path) = self.path {
            file_io::save_file(path, content)?;
            self.last_save_content = content.to_string();
            self.dirty = false;
            self.last_edit = None;
        }
        Ok(())
    }
}
