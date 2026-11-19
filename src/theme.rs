//! Named palettes and the one-line state file that remembers the choice.

use std::path::PathBuf;

pub struct Theme {
    pub name: &'static str,
    /// RGB triples and 256-colour fallbacks, indexed by role order:
    /// user, assistant, thinking, tool, error, info.
    pub rgb: [[u8; 3]; 6],
    pub ansi: [u8; 6],
    pub add: ([u8; 3], u8),
    pub delete: ([u8; 3], u8),
}

pub const THEMES: &[Theme] = &[
    Theme {
        name: "default",
        rgb: [
            [0x8f, 0xbf, 0xff],
            [0xe6, 0xe6, 0xe6],
            [0x80, 0x80, 0x80],
            [0x7f, 0xc8, 0xa0],
            [0xff, 0x6b, 0x6b],
            [0xd7, 0xa6, 0x5a],
        ],
        ansi: [111, 255, 244, 108, 203, 214],
        add: ([0x7f, 0xd6, 0x8f], 114),
        delete: ([0xff, 0x6b, 0x6b], 203),
    },
    Theme {
        name: "mono",
        rgb: [
            [0xe6, 0xe6, 0xe6],
            [0xe6, 0xe6, 0xe6],
            [0x9a, 0x9a, 0x9a],
            [0xe6, 0xe6, 0xe6],
            [0xff, 0x6b, 0x6b],
            [0xd7, 0xa6, 0x5a],
        ],
        ansi: [255, 255, 245, 255, 203, 214],
        add: ([0x7f, 0xd6, 0x8f], 114),
        delete: ([0xff, 0x6b, 0x6b], 203),
    },
    Theme {
        name: "solarized-dark",
        rgb: [
            [0x26, 0x8b, 0xd2],
            [0x83, 0x94, 0x96],
            [0x58, 0x6e, 0x75],
            [0x85, 0x99, 0x00],
            [0xdc, 0x32, 0x2f],
            [0xb5, 0x89, 0x00],
        ],
        ansi: [33, 245, 242, 100, 167, 136],
        add: ([0x7f, 0xd6, 0x8f], 114),
        delete: ([0xff, 0x6b, 0x6b], 203),
    },
    Theme {
        name: "gruvbox-dark",
        rgb: [
            [0x83, 0xa5, 0x98],
            [0xeb, 0xdb, 0xb2],
            [0x92, 0x83, 0x74],
            [0xb8, 0xbb, 0x26],
            [0xfb, 0x49, 0x34],
            [0xfa, 0xbd, 0x2f],
        ],
        ansi: [109, 223, 246, 142, 167, 214],
        add: ([0x7f, 0xd6, 0x8f], 114),
        delete: ([0xff, 0x6b, 0x6b], 203),
    },
];

fn state_path() -> PathBuf {
    crate::config::dirs_state().join("imp").join("theme")
}

pub fn load() -> &'static Theme {
    let name = std::fs::read_to_string(state_path()).unwrap_or_default();
    by_name(name.trim()).unwrap_or(&THEMES[0])
}

pub fn by_name(name: &str) -> Option<&'static Theme> {
    THEMES.iter().find(|t| t.name == name)
}

pub fn names() -> Vec<&'static str> {
    THEMES.iter().map(|t| t.name).collect()
}

pub fn save(name: &str) -> Result<(), String> {
    let path = state_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, name).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_stable() {
        assert_eq!(names(), vec!["default", "mono", "solarized-dark", "gruvbox-dark"]);
        assert!(by_name("neon").is_none());
    }
}
