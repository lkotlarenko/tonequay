//! A persistent local library, scanned off the UI/audio threads. Directory changes
//! only refresh the list; they never reload a running firmware automatically.
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime},
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalFirmware {
    pub path: PathBuf,
    pub name: String,
    pub search: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub root: PathBuf,
    pub files: Vec<LocalFirmware>,
    pub error: Option<String>,
}
impl Snapshot {
    pub fn empty(root: PathBuf) -> Self {
        Self {
            root,
            files: vec![],
            error: None,
        }
    }
}
fn scan(root: &Path) -> Result<Vec<LocalFirmware>, String> {
    let entries =
        std::fs::read_dir(root).map_err(|e| format!("Firmware folder {}: {e}", root.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("fwsc"))
        {
            continue;
        }
        // Non-recursive and no symlinks: do not wander outside a selected folder.
        if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        files.push(LocalFirmware {
            path,
            name: name.clone(),
            search: name.to_lowercase(),
            size: meta.len(),
            modified: meta.modified().ok(),
        });
    }
    files.sort_by(|a, b| a.search.cmp(&b.search).then(a.path.cmp(&b.path)));
    Ok(files)
}
pub struct Folder {
    root: Arc<Mutex<PathBuf>>,
    latest: Arc<Mutex<Option<Snapshot>>>,
    quit: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Folder {
    pub fn new(root: PathBuf, ctx: eframe::egui::Context) -> Self {
        Self::start(root, ctx, Duration::from_secs(2))
    }
    fn start(root: PathBuf, ctx: eframe::egui::Context, interval: Duration) -> Self {
        let root = Arc::new(Mutex::new(root));
        let latest = Arc::new(Mutex::new(None));
        let quit = Arc::new(AtomicBool::new(false));
        let (r, l, q) = (root.clone(), latest.clone(), quit.clone());
        let thread = std::thread::spawn(move || {
            let mut previous: Option<Snapshot> = None;
            while !q.load(Ordering::Relaxed) {
                let Ok(path) = r.lock().map(|p| p.clone()) else {
                    break;
                };
                let new_root = previous
                    .as_ref()
                    .is_none_or(|p| p.root != path || p.error.is_some());
                let result = if new_root {
                    std::fs::create_dir_all(&path)
                        .map_err(|e| {
                            format!("Cannot create firmware folder {}: {e}", path.display())
                        })
                        .and_then(|()| scan(&path))
                } else {
                    scan(&path)
                };
                let snapshot = match result {
                    Ok(files) => Snapshot {
                        root: path.clone(),
                        files,
                        error: None,
                    },
                    Err(error) => Snapshot {
                        root: path.clone(),
                        files: vec![],
                        error: Some(error),
                    },
                };
                if previous.as_ref() != Some(&snapshot) {
                    if let Ok(current) = r.lock() {
                        if *current == path {
                            if let Ok(mut slot) = l.lock() {
                                *slot = Some(snapshot.clone());
                            }
                            ctx.request_repaint();
                        }
                    }
                    previous = Some(snapshot);
                }
                if !q.load(Ordering::Relaxed) {
                    std::thread::park_timeout(interval);
                }
            }
        });
        Self {
            root,
            latest,
            quit,
            thread: Some(thread),
        }
    }
    pub fn set_path(&self, path: PathBuf) {
        if let Ok(mut root) = self.root.lock() {
            *root = path;
        }
        if let Some(t) = &self.thread {
            t.thread().unpark();
        }
    }
    pub fn take(&self) -> Option<Snapshot> {
        self.latest.try_lock().ok().and_then(|mut s| s.take())
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        self.quit.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            t.thread().unpark();
            let _ = t.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scan_is_sorted_nonrecursive_and_only_fwsc() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Zebra.FWSC"), b"z").unwrap();
        std::fs::write(d.path().join("alpha.fwsc"), b"a").unwrap();
        std::fs::write(d.path().join("installer.sh"), b"never run").unwrap();
        std::fs::create_dir(d.path().join("nested")).unwrap();
        std::fs::write(d.path().join("nested/hidden.fwsc"), b"x").unwrap();
        let f = scan(d.path()).unwrap();
        assert_eq!(
            f.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            vec!["alpha", "Zebra"]
        );
        assert!(scan(&d.path().join("missing")).is_err());
    }
    fn wait(folder: &Folder, predicate: impl Fn(&Snapshot) -> bool) -> Snapshot {
        let start = std::time::Instant::now();
        loop {
            if let Some(s) = folder.take() {
                if predicate(&s) {
                    return s;
                }
            }
            assert!(
                start.elapsed() < Duration::from_secs(3),
                "folder watcher did not update"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn background_scan_detects_new_files_removal_and_folder_changes() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("firmware");
        let f = Folder::start(root.clone(), Default::default(), Duration::from_millis(15));
        wait(&f, |s| s.root == root);
        assert!(root.is_dir());
        std::fs::write(root.join("new.fwsc"), b"x").unwrap();
        wait(&f, |s| s.files.len() == 1);
        std::fs::remove_file(root.join("new.fwsc")).unwrap();
        wait(&f, |s| s.files.is_empty());
        let other = d.path().join("other");
        f.set_path(other.clone());
        wait(&f, |s| s.root == other);
        assert!(other.is_dir());
    }
}
