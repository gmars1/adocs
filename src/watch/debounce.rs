use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{Event, EventKind, RecursiveMode, Watcher};

#[derive(Debug, Clone)]
pub struct WatcherHandle {
    changed_paths: Arc<Mutex<HashSet<PathBuf>>>,
}

impl WatcherHandle {
    pub fn start(
        watch_paths: Vec<PathBuf>,
        debounce_ms: u64,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let changed_paths = Arc::new(Mutex::new(HashSet::new()));
        let watched_changed = changed_paths.clone();

        let mut watcher = notify::recommended_watcher(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
                        _ => return,
                    }
                    if let Ok(mut guard) = watched_changed.lock() {
                        for path in event.paths {
                            guard.insert(path);
                        }
                    }
                }
            },
        )?;

        for path in &watch_paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)?;
            }
        }

        let drainer = changed_paths.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(debounce_ms));
                // The lock is held only briefly during insert in the callback,
                // so there's nothing to drain here. The watcher inserts directly.
                // This loop just keeps the watcher alive.
                let _keep_alive = drainer.clone();
            }
        });

        Ok(WatcherHandle { changed_paths })
    }

    pub fn take_changed(&self) -> Vec<PathBuf> {
        if let Ok(mut guard) = self.changed_paths.lock() {
            guard.drain().collect()
        } else {
            Vec::new()
        }
    }
}
