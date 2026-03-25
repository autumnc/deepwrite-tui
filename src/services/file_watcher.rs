use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSignature {
    exists: bool,
    modified: Option<SystemTime>,
    len: u64,
    digest: u64,
}

impl FileSignature {
    fn read(path: &Path) -> anyhow::Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => {
                let modified = std::fs::metadata(path)
                    .ok()
                    .and_then(|meta| meta.modified().ok());
                let len = bytes.len() as u64;

                let mut hasher = DefaultHasher::new();
                bytes.hash(&mut hasher);

                Ok(Self {
                    exists: true,
                    modified,
                    len,
                    digest: hasher.finish(),
                })
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                exists: false,
                modified: None,
                len: 0,
                digest: 0,
            }),
            Err(err) => Err(err.into()),
        }
    }
}

pub struct FileWatcher {
    path: PathBuf,
    last_signature: FileSignature,
    last_polled_at: Instant,
    poll_interval: Duration,
}

impl FileWatcher {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        Self::with_poll_interval(path, Duration::from_millis(250))
    }

    fn with_poll_interval(path: &Path, poll_interval: Duration) -> anyhow::Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            last_signature: FileSignature::read(path)?,
            last_polled_at: Instant::now(),
            poll_interval,
        })
    }

    pub fn poll_changed(&mut self) -> anyhow::Result<Option<PathBuf>> {
        if self.last_polled_at.elapsed() < self.poll_interval {
            return Ok(None);
        }
        self.last_polled_at = Instant::now();

        let current = FileSignature::read(&self.path)?;
        if current != self.last_signature {
            self.last_signature = current;
            return Ok(Some(self.path.clone()));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn poll_changed_detects_plain_write() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("note.md");
        fs::write(&path, "before").unwrap();

        let mut watcher = FileWatcher::with_poll_interval(&path, Duration::ZERO).unwrap();
        assert!(watcher.poll_changed().unwrap().is_none());

        fs::write(&path, "after").unwrap();
        assert_eq!(watcher.poll_changed().unwrap(), Some(path));
    }

    #[test]
    fn poll_changed_detects_atomic_replace() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("note.md");
        fs::write(&path, "before").unwrap();

        let mut watcher = FileWatcher::with_poll_interval(&path, Duration::ZERO).unwrap();
        assert!(watcher.poll_changed().unwrap().is_none());

        let temp = tmp.path().join("temp");
        fs::write(&temp, "after atomic replace").unwrap();
        fs::rename(&temp, &path).unwrap();

        assert_eq!(watcher.poll_changed().unwrap(), Some(path));
    }
}
