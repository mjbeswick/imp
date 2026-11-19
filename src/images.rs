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
    let mime = mime(&bytes).ok_or_else(|| {
        format!(
            "{}: only PNG, JPEG, GIF, and WebP are supported",
            path.display()
        )
    })?;
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
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        mime: mime.to_string(),
        bytes: bytes.len() as u64,
    })
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
    fn labels_sizes() {
        assert_eq!(size_label(900), "900 B");
        assert_eq!(size_label(120 * 1024), "120 KB");
        assert_eq!(size_label(2 * 1024 * 1024), "2.0 MB");
    }
}
