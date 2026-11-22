//! Image attachments: copy into the state directory and describe them.

use crate::api::Image;
use std::path::{Path, PathBuf};

pub const MAX_BYTES: u64 = 10 * 1024 * 1024;
pub const MAX_PER_MESSAGE: usize = 4;

pub fn dir() -> PathBuf {
    crate::config::dirs_state().join("imp").join("images")
}

fn mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF8") {
        Some("image/gif")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Copy an image into the state directory and return its record.
pub fn attach(raw: &str, base: &Path) -> Result<Image, String> {
    attach_in(raw, base, &dir())
}

fn attach_in(raw: &str, base: &Path, dir: &Path) -> Result<Image, String> {
    let given = PathBuf::from(raw.trim());
    let path = if given.is_absolute() {
        given
    } else {
        base.join(given)
    };
    let path = path
        .canonicalize()
        .map_err(|_| format!("{}: no such file", path.display()))?;
    let meta = std::fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("{}: not a file", path.display()));
    }
    if meta.len() > MAX_BYTES {
        return Err(format!(
            "{}: {} MB is over the 10 MB limit",
            path.display(),
            meta.len() / (1024 * 1024)
        ));
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    store(bytes, &name, dir).map_err(|e| format!("{}: {e}", path.display()))
}

/// Keep image bytes in `dir`, named by their hash, and return the record.
pub fn store(bytes: Vec<u8>, name: &str, dir: &Path) -> Result<Image, String> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err(format!("{} MB is over the 10 MB limit", bytes.len() / (1024 * 1024)));
    }
    let mime = mime(&bytes).ok_or("only PNG, JPEG, GIF, and WebP are supported")?;
    let ext = match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        _ => "webp",
    };
    let file = format!("{:016x}.{ext}", hash(&bytes));
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let target = dir.join(&file);
    if !target.exists() {
        std::fs::write(&target, &bytes).map_err(|e| format!("{}: {e}", target.display()))?;
    }
    Ok(Image {
        file,
        name: name.to_string(),
        mime: mime.to_string(),
        bytes: bytes.len() as u64,
    })
}

/// macOS: write the clipboard's PNG to a temporary file and print it.
const MAC_PASTE: &str = r#"f=$(mktemp) && osascript -e "write (the clipboard as «class PNGf») to (open for access POSIX file \"$f\" with write permission)" >/dev/null 2>&1; cat "$f"; rm -f "$f""#;

/// The command that prints a clipboard image, or `None` where there is no
/// clipboard to read: over SSH it belongs to another machine.
pub fn clipboard_command() -> Option<String> {
    command_for(|v| std::env::var_os(v).is_some_and(|x| !x.is_empty()), cfg!(target_os = "macos"))
}

fn command_for(set: impl Fn(&str) -> bool, mac: bool) -> Option<String> {
    if set("SSH_CONNECTION") {
        return None;
    }
    if mac {
        Some(MAC_PASTE.into())
    } else if set("WAYLAND_DISPLAY") {
        Some("wl-paste --no-newline --type image/png".into())
    } else if set("DISPLAY") {
        Some("xclip -selection clipboard -t image/png -o".into())
    } else {
        None
    }
}

/// What a clipboard read found.
#[derive(Debug, PartialEq)]
pub enum Clip {
    Image(Vec<u8>),
    /// Text, nothing, or an image the tool would not give.
    Nothing,
    /// No tool to ask, or it took longer than two seconds.
    Unsupported,
}

/// Run the clipboard command for at most two seconds.
pub fn read_clipboard(command: &str) -> Clip {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let Ok(mut child) = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Clip::Unsupported;
    };
    let Some(mut out) = child.stdout.take() else { return Clip::Unsupported };
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = out.by_ref().take(MAX_BYTES + 1).read_to_end(&mut bytes);
        bytes
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if std::time::Instant::now() < deadline => std::thread::sleep(std::time::Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Clip::Unsupported;
            }
        }
    };
    let bytes = reader.join().unwrap_or_default();
    match status.code() {
        // The shell's "command not found".
        Some(127) => Clip::Unsupported,
        _ if mime(&bytes).is_some() => Clip::Image(bytes),
        _ => Clip::Nothing,
    }
}

pub fn size_label(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{} KB", (bytes + 512) / 1024)
    } else {
        format!("{bytes} B")
    }
}

pub fn marker(image: &Image) -> String {
    format!("[image {} · {}]", image.name, size_label(image.bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!(
            "imp-img-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn png() -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&[0u8; 16]);
        bytes
    }

    #[test]
    fn attaches_and_dedupes() {
        let base = tmp();
        let out = tmp();
        std::fs::write(base.join("shot.png"), png()).unwrap();
        let a = attach_in("shot.png", &base, &out).unwrap();
        assert_eq!(a.name, "shot.png");
        assert_eq!(a.mime, "image/png");
        assert!(out.join(&a.file).is_file());
        let b = attach_in("shot.png", &base, &out).unwrap();
        assert_eq!(a.file, b.file);
        assert_eq!(std::fs::read_dir(&out).unwrap().count(), 1);
    }

    #[test]
    fn refuses_bad_files() {
        let base = tmp();
        let out = tmp();
        std::fs::write(base.join("bad.png"), b"not an image").unwrap();
        let err = attach_in("bad.png", &base, &out).unwrap_err();
        assert!(err.contains("only PNG"), "{err}");

        let big = base.join("big.png");
        let file = std::fs::File::create(&big).unwrap();
        file.set_len(MAX_BYTES + 1).unwrap();
        let err = attach_in("big.png", &base, &out).unwrap_err();
        assert!(err.contains("10 MB"), "{err}");
    }

    #[test]
    fn the_clipboard_gives_an_image_nothing_or_no_answer() {
        let dir = tmp();
        let png_path = dir.join("p.png");
        std::fs::write(&png_path, png()).unwrap();
        let image = read_clipboard(&format!("cat '{}'", png_path.display()));
        assert_eq!(image, Clip::Image(png()));
        assert_eq!(read_clipboard("printf 'just text'"), Clip::Nothing);
        assert_eq!(read_clipboard("exit 1"), Clip::Nothing);
        assert_eq!(read_clipboard("imp-no-such-clipboard-tool"), Clip::Unsupported);
        let start = std::time::Instant::now();
        assert_eq!(read_clipboard("sleep 5"), Clip::Unsupported);
        assert!(start.elapsed() < std::time::Duration::from_secs(4), "the limit did not hold");
        let stored = store(png(), "pasted.png", &dir).unwrap();
        assert_eq!((stored.name.as_str(), stored.mime.as_str()), ("pasted.png", "image/png"));
        assert!(store(b"text".to_vec(), "x", &dir).unwrap_err().contains("only PNG"));
    }

    #[test]
    fn the_clipboard_tool_follows_the_session() {
        let env = |vars: &'static [&'static str]| move |v: &str| vars.contains(&v);
        assert!(command_for(env(&[]), true).unwrap().contains("osascript"));
        assert!(command_for(env(&["WAYLAND_DISPLAY", "DISPLAY"]), false).unwrap().starts_with("wl-paste"));
        assert!(command_for(env(&["DISPLAY"]), false).unwrap().starts_with("xclip"));
        assert_eq!(command_for(env(&[]), false), None, "a bare console has no clipboard");
        assert_eq!(command_for(env(&["SSH_CONNECTION", "DISPLAY"]), true), None, "over SSH");
    }

    #[test]
    fn labels_sizes() {
        assert_eq!(size_label(900), "900 B");
        assert_eq!(size_label(120 * 1024), "120 KB");
        assert_eq!(size_label(2 * 1024 * 1024), "2.0 MB");
    }
}
