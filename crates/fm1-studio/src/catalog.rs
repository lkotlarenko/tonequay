//! Hub metadata and creator downloads. All network/disk work stays on a worker.
use crate::persistence;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub const HUB: &str = "https://thefirmwarehub.com/catalog.json";
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    #[serde(default, deserialize_with = "kinds")]
    pub kind: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub what: String,
    pub lic: Option<String>,
    pub pkg: Option<String>,
    pub sha256: Option<String>,
    pub size: Option<u64>,
    pub release: Option<String>,
    pub tag: Option<String>,
}
fn kinds<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Kinds {
        One(String),
        Many(Vec<String>),
    }
    Ok(match Option::<Kinds>::deserialize(deserializer)? {
        Some(Kinds::One(s)) => s,
        Some(Kinds::Many(v)) => v.join(" "),
        None => String::new(),
    })
}
impl Entry {
    pub fn project(&self) -> String {
        if self.repo.starts_with("https://") {
            self.repo.clone()
        } else {
            format!("https://github.com/{}", self.repo)
        }
    }
    pub fn cached(&self, root: &std::path::Path) -> Option<PathBuf> {
        let hash = self.sha256.as_ref()?;
        if !valid_hash(hash) {
            return None;
        }
        Some(root.join("packages").join(format!("{hash}.fwsc")))
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    pub firmwares: Vec<Entry>,
}
pub fn parse(bytes: &[u8]) -> Result<Catalog, String> {
    let c: Catalog = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if c.firmwares.len() > 4096 {
        return Err("too many catalog entries".into());
    }
    let mut ids = std::collections::HashSet::new();
    for e in &c.firmwares {
        if e.id.is_empty() || e.id.len() > 128 || !ids.insert(&e.id) {
            return Err("invalid or duplicate firmware ID".into());
        }
        if e.sha256.as_ref().is_some_and(|h| !valid_hash(h)) {
            return Err("invalid catalog SHA-256".into());
        }
    }
    Ok(c)
}
fn valid_hash(h: &str) -> bool {
    h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn verify(bytes: &[u8], hash: &str) -> Result<(), String> {
    if !valid_hash(hash) || !persistence::identity(bytes).eq_ignore_ascii_case(hash) {
        return Err("SHA-256 mismatch; package rejected".into());
    }
    Ok(())
}
pub enum Update {
    Progress(String),
    Catalog(Catalog),
    Downloaded(PathBuf),
    Error(String),
}
pub struct Job {
    pub rx: Receiver<Update>,
    pub cancel: Arc<AtomicBool>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("FM1-Studio/0.4.0 (+https://github.com/joakimeriksson/fm1-emu)")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())
}
fn fetch(
    client: &reqwest::blocking::Client,
    url: &str,
    limit: u64,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<Vec<u8>, String> {
    let mut response = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("download exceeds size limit".into());
    }
    let mut bytes = Vec::new();
    let mut chunk = [0; 16384];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("download cancelled".into());
        }
        let n = response.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if bytes.len() as u64 + n as u64 > limit {
            return Err("download exceeds size limit".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
        progress(bytes.len() as u64);
    }
    Ok(bytes)
}
pub fn refresh(root: PathBuf) -> Job {
    job(move |tx, cancel| {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let stamp = root.join("catalog-attempt");
        if let Ok(t) = std::fs::read_to_string(&stamp) {
            if t.parse::<u64>()
                .is_ok_and(|t| now.saturating_sub(t) < 86400)
            {
                return Err("Hub refresh limited to once per 24 hours; using offline cache".into());
            }
        }
        persistence::atomic(&stamp, now.to_string().as_bytes())?;
        let bytes = fetch(&client()?, HUB, 4 * 1024 * 1024, cancel, |_| {})?;
        let catalog = parse(&bytes)?;
        persistence::atomic(&root.join("catalog.json"), &bytes)?;
        let _ = tx.send(Update::Catalog(catalog));
        Ok(())
    })
}
pub fn download(root: PathBuf, entry: Entry) -> Job {
    job(move |tx, cancel| {
        let hash = entry
            .sha256
            .as_ref()
            .ok_or("no trusted catalog SHA-256; import locally")?;
        let path = entry.cached(&root).ok_or("invalid firmware identity")?;
        if path.exists() {
            let b = persistence::read_bounded(&path, persistence::MAX_PACKAGE)?;
            verify(&b, hash)?;
            let _ = tx.send(Update::Downloaded(path));
            return Ok(());
        }
        let client = client()?;
        // A release page is not a package URL. Resolve its author's GitHub assets.
        let release = entry
            .release
            .as_deref()
            .ok_or("no original release URL; use local import")?;
        let url = if release.ends_with(".fwsc") && !release.contains("thefirmwarehub.com") {
            release.to_owned()
        } else {
            let tag = entry
                .tag
                .as_deref()
                .ok_or("creator release has no exact tag; use local import")?;
            if entry.repo.contains("..") || entry.repo.split('/').count() != 2 {
                return Err("no supported creator release endpoint; use local import".into());
            }
            let api = format!(
                "https://api.github.com/repos/{}/releases/tags/{}",
                entry.repo, tag
            );
            let data = fetch(&client, &api, 2 * 1024 * 1024, cancel, |_| {})?;
            let value: serde_json::Value =
                serde_json::from_slice(&data).map_err(|e| e.to_string())?;
            let assets = value["assets"].as_array().ok_or("release has no assets")?;
            let candidates: Vec<_> = assets
                .iter()
                .filter(|a| a["name"].as_str().is_some_and(|n| n.ends_with(".fwsc")))
                .collect();
            if candidates.len() != 1 {
                return Err(
                    "creator release has multiple/no FWSC assets; select and import locally".into(),
                );
            }
            candidates[0]["browser_download_url"]
                .as_str()
                .ok_or("missing original asset URL")?
                .to_owned()
        };
        let bytes = fetch(&client, &url, persistence::MAX_PACKAGE, cancel, |n| {
            let _ = tx.try_send(Update::Progress(format!("Downloading {} KiB", n / 1024)));
        })?;
        if entry.size.is_some_and(|n| n != bytes.len() as u64) {
            return Err("catalog size mismatch".into());
        }
        verify(&bytes, hash)?;
        // Validate before installing in cache. Never execute archive or installer scripts.
        let _ = fm1_soc::stock_from_package(&bytes, None)?;
        persistence::atomic(&path, &bytes)?;
        let _ = tx.send(Update::Downloaded(path));
        Ok(())
    })
}
fn job(
    f: impl FnOnce(&mpsc::SyncSender<Update>, &AtomicBool) -> Result<(), String> + Send + 'static,
) -> Job {
    let (tx, rx) = mpsc::sync_channel(16);
    let cancel = Arc::new(AtomicBool::new(false));
    let c = cancel.clone();
    std::thread::spawn(move || {
        if let Err(e) = f(&tx, &c) {
            let _ = tx.send(Update::Error(e));
        }
    });
    Job { rx, cancel }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_schema_and_invalid_hash() {
        assert!(parse(br#"{"firmwares":[{"id":"test","name":"Test","sha256":"bad"}]}"#).is_err());
        let c=parse(br#"{"license":{},"updated":"today","firmwares":[{"id":"test","name":"Test","kind":"groove","lic":"MIT","pkg":"fw/t.fwsc"}]}"#).unwrap();
        assert_eq!(c.firmwares[0].kind, "groove");
        let c =
            parse(br#"{"firmwares":[{"id":"multi","name":"Multiple","kind":["synth","drums"]}]}"#)
                .unwrap();
        assert_eq!(c.firmwares[0].kind, "synth drums");
    }
    #[test]
    fn mismatched_download_rejected() {
        assert!(verify(b"wrong", &persistence::identity(b"right")).is_err());
        assert!(verify(b"right", &persistence::identity(b"right")).is_ok());
    }
    #[test]
    fn refresh_failures_are_rate_limited() {
        let d = tempfile::tempdir().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        persistence::atomic(
            &d.path().join("catalog-attempt"),
            now.to_string().as_bytes(),
        )
        .unwrap();
        let job = refresh(d.path().into());
        assert!(matches!(job.rx.recv().unwrap(),Update::Error(s) if s.contains("24 hours")));
    }
}
