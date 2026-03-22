use notify::{EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;

pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    pub rx: mpsc::Receiver<PathBuf>,
}

impl FileWatcher {
    pub fn new(path: &PathBuf) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let watched_path = path.clone();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        // Send the watched path when an external modification is detected.
                        let _ = tx.send(watched_path.clone());
                    }
                }
            })?;

        watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }
}
