//! Exact-package save isolation. Full flash is a Studio format, not .fm1save.
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
pub const MAX_PACKAGE: u64 = 16 * 1024 * 1024;
pub fn identity(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn root() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        })
        .join("fm1-studio")
}
pub fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("file exceeds size limit".into());
    }
    Ok(bytes)
}
pub fn atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().ok_or("missing parent directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("pending");
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
pub fn save_path(root: &Path, hash: &str) -> Result<PathBuf, String> {
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid firmware identity".into());
    }
    Ok(root.join("saves").join(format!("{hash}.flash")))
}
pub fn save(path: &Path, flash: &[u8]) -> Result<(), String> {
    if flash.len() != fm1_soc::FLASH_SIZE {
        return Err("wrong flash size".into());
    }
    if path.exists() {
        std::fs::copy(path, path.with_extension("flash.bak")).map_err(|e| e.to_string())?;
    }
    atomic(path, flash)
}
/// Portable Studio backup: magic/version, exact package digest, flash bytes.
/// Rejecting a different digest prevents silently migrating incompatible layouts.
pub fn export(path: &Path, hash: &str, flash: &[u8]) -> Result<(), String> {
    let _ = save_path(Path::new("."), hash)?;
    if flash.len() != fm1_soc::FLASH_SIZE {
        return Err("wrong flash size".into());
    }
    let mut bytes = b"FM1STUDIO1\n".to_vec();
    bytes.extend_from_slice(hash.as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(flash);
    atomic(path, &bytes)
}
pub fn import(path: &Path, hash: &str) -> Result<Vec<u8>, String> {
    let bytes = read_bounded(path, fm1_soc::FLASH_SIZE as u64 + 76)?;
    let header = format!("FM1STUDIO1\n{hash}\n");
    if bytes.len() != header.len() + fm1_soc::FLASH_SIZE || !bytes.starts_with(header.as_bytes()) {
        return Err("Backup is not a Studio backup for this exact firmware. Browser .fm1save and raw flash are not portable Studio backups.".into());
    }
    Ok(bytes[header.len()..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn isolation_and_traversal() {
        let r = Path::new("/tmp/test");
        assert_ne!(
            save_path(r, &identity(b"v1")),
            save_path(r, &identity(b"v2"))
        );
        assert!(save_path(r, "../bad").is_err());
        assert_eq!(
            identity(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn portable_backup_rejects_other_firmware() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("test.fm1flash");
        let hash = identity(b"firmware A");
        export(&p, &hash, &vec![42; fm1_soc::FLASH_SIZE]).unwrap();
        assert_eq!(import(&p, &hash).unwrap()[0], 42);
        assert!(import(&p, &identity(b"firmware B")).is_err());
        std::fs::write(&p, b"FM1STUDIO1\n").unwrap();
        assert!(import(&p, &hash).is_err());
    }
    #[test]
    fn backups_and_bounds() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("save.flash");
        save(&p, &vec![1; fm1_soc::FLASH_SIZE]).unwrap();
        save(&p, &vec![2; fm1_soc::FLASH_SIZE]).unwrap();
        assert_eq!(std::fs::read(p.with_extension("flash.bak")).unwrap()[0], 1);
        assert!(read_bounded(&p, 32).is_err());
    }
}
