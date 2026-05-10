use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::SigilError;
use crate::toml_schema::compute_logic_hash;

#[derive(Debug)]
pub struct HotReloadEvent {
    pub path: PathBuf,
    pub content_hash: String,
}

pub struct HotReloadWatcher {
    pub rx: Receiver<HotReloadEvent>,
    _watcher: RecommendedWatcher,
}

impl HotReloadWatcher {
    pub fn spawn_listener<F>(self, mut on_event: F) -> std::thread::JoinHandle<()>
    where
        F: FnMut(HotReloadEvent) + Send + 'static,
    {
        std::thread::spawn(move || {
            let HotReloadWatcher { rx, _watcher } = self;
            for event in rx.iter() {
                on_event(event);
            }
            drop(_watcher);
        })
    }
}

pub fn start_hot_reload(paths: Vec<PathBuf>) -> Result<HotReloadWatcher, SigilError> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            for path in event.paths {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let hash = compute_logic_hash(&content);
                    let _ = tx.send(HotReloadEvent {
                        path: path.clone(),
                        content_hash: hash,
                    });
                }
            }
        }
    })
    .map_err(|e| SigilError::io(e.to_string()))?;

    for path in paths {
        let mode = if path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        watcher
            .watch(&path, mode)
            .map_err(|e| SigilError::io(e.to_string()))?;
    }

    Ok(HotReloadWatcher {
        rx,
        _watcher: watcher,
    })
}
