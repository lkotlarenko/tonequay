//! Explicit desktop integration lets KDE resolve the Wayland application ID/icon.
use crate::persistence;
use std::path::PathBuf;
pub fn install() -> Result<String, String> {
    let executable = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_exe)
        .map_err(|e| e.to_string())?;
    let root = persistence::root()
        .parent()
        .ok_or("missing XDG data directory")?
        .to_path_buf();
    persistence::atomic(
        &root.join("icons/hicolor/256x256/apps/tonequay.png"),
        include_bytes!("../../../packaging/studio/tonequay.png"),
    )?;
    persistence::atomic(
        &root.join("icons/hicolor/scalable/apps/tonequay.svg"),
        include_bytes!("../../../packaging/studio/tonequay.svg"),
    )?;
    let launcher_prefix = if std::env::var_os("APPIMAGE").is_some() {
        "env APPIMAGE_EXTRACT_AND_RUN=1 "
    } else {
        ""
    };
    let entry=format!("[Desktop Entry]\nType=Application\nName=Tonequay\nComment=Your firmware. Your instrument.\nExec={launcher_prefix}{} %f\nIcon=fm1-studio\nTerminal=false\nCategories=AudioVideo;Audio;Music;\nStartupWMClass=tonequay\nKeywords=FM-1;MIDI;Synthesizer;Emulator;\n",quote(executable.to_str().ok_or("launcher path must be valid UTF-8")?)?);
    let path = root.join("applications/tonequay.desktop");
    persistence::atomic(&path, entry.as_bytes())?;
    Ok(format!(
        "Desktop launcher installed at {}. Keep the application at {}.",
        path.display(),
        executable.display()
    ))
}
fn quote(path: &str) -> Result<String, String> {
    if path.contains(['\n', '\r']) {
        return Err("launcher path contains a newline".into());
    }
    let mut quoted = String::from("\"");
    for ch in path.chars() {
        match ch {
            '%' => quoted.push_str("%%"),
            '\\' => quoted.push_str("\\\\\\\\"),
            '"' | '$' | '`' => {
                quoted.push('\\');
                quoted.push(ch);
            }
            _ => quoted.push(ch),
        }
    }
    quoted.push('"');
    Ok(quoted)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_paths_preserve_spaces_and_escape_field_codes() {
        assert_eq!(quote("/tmp/My App%f").unwrap(), "\"/tmp/My App%%f\"");
        assert!(quote("/tmp/bad\nentry").is_err());
    }
}
