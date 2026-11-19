//! Named palettes and the one-line state file that remembers the choice.

use std::path::PathBuf;

/// Colour slots, in the order every theme lists them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Slot {
    Text,
    User,
    Muted,
    Border,
    Accent,
    Tool,
    Success,
    Error,
    Code,
    Link,
    /// A background, for the highlighted picker row.
    Select,
}

pub struct Theme {
    pub name: &'static str,
    /// Made for a light terminal background.
    pub light: bool,
    /// RGB per `Slot`. The 256-colour fallback is computed from it.
    pub rgb: [[u8; 3]; 11],
}

impl Theme {
    pub fn rgb(&self, slot: Slot) -> [u8; 3] {
        self.rgb[slot as usize]
    }
}

pub const THEMES: &[Theme] = &[
    Theme {
        name: "default",
        light: false,
        rgb: [
            [0xe6, 0xe6, 0xe6],
            [0x8f, 0xbf, 0xff],
            [0x80, 0x80, 0x80],
            [0x3a, 0x3f, 0x47],
            [0xd7, 0xa6, 0x5a],
            [0x7f, 0xc8, 0xa0],
            [0x7f, 0xd6, 0x8f],
            [0xff, 0x6b, 0x6b],
            [0xc4, 0x9b, 0xff],
            [0x6f, 0xc3, 0xff],
            [0x24, 0x32, 0x4a],
        ],
    },
    Theme {
        name: "default-light",
        light: true,
        rgb: [
            [0x1f, 0x23, 0x28],
            [0x0b, 0x5c, 0xad],
            [0x6e, 0x77, 0x81],
            [0xd0, 0xd7, 0xde],
            [0x9a, 0x67, 0x00],
            [0x11, 0x63, 0x29],
            [0x1a, 0x7f, 0x37],
            [0xcf, 0x22, 0x2e],
            [0x82, 0x50, 0xdf],
            [0x09, 0x69, 0xda],
            [0xdd, 0xf4, 0xff],
        ],
    },
    Theme {
        name: "solarized-dark",
        light: false,
        rgb: [
            [0x93, 0xa1, 0xa1],
            [0x26, 0x8b, 0xd2],
            [0x58, 0x6e, 0x75],
            [0x0e, 0x45, 0x52],
            [0xb5, 0x89, 0x00],
            [0x2a, 0xa1, 0x98],
            [0x85, 0x99, 0x00],
            [0xdc, 0x32, 0x2f],
            [0xd3, 0x36, 0x82],
            [0x6c, 0x71, 0xc4],
            [0x07, 0x36, 0x42],
        ],
    },
    Theme {
        name: "solarized-light",
        light: true,
        rgb: [
            [0x58, 0x6e, 0x75],
            [0x26, 0x8b, 0xd2],
            [0x93, 0xa1, 0xa1],
            [0xe4, 0xdc, 0xc4],
            [0xb5, 0x89, 0x00],
            [0x2a, 0xa1, 0x98],
            [0x85, 0x99, 0x00],
            [0xdc, 0x32, 0x2f],
            [0xd3, 0x36, 0x82],
            [0x6c, 0x71, 0xc4],
            [0xee, 0xe8, 0xd5],
        ],
    },
    Theme {
        name: "gruvbox-dark",
        light: false,
        rgb: [
            [0xeb, 0xdb, 0xb2],
            [0x83, 0xa5, 0x98],
            [0x92, 0x83, 0x74],
            [0x50, 0x49, 0x45],
            [0xfa, 0xbd, 0x2f],
            [0x8e, 0xc0, 0x7c],
            [0xb8, 0xbb, 0x26],
            [0xfb, 0x49, 0x34],
            [0xd3, 0x86, 0x9b],
            [0x83, 0xa5, 0x98],
            [0x3c, 0x38, 0x36],
        ],
    },
    Theme {
        name: "gruvbox-light",
        light: true,
        rgb: [
            [0x3c, 0x38, 0x36],
            [0x07, 0x66, 0x78],
            [0x92, 0x83, 0x74],
            [0xd5, 0xc4, 0xa1],
            [0xb5, 0x76, 0x14],
            [0x42, 0x7b, 0x58],
            [0x79, 0x74, 0x0e],
            [0x9d, 0x00, 0x06],
            [0x8f, 0x3f, 0x71],
            [0x07, 0x66, 0x78],
            [0xeb, 0xdb, 0xb2],
        ],
    },
    Theme {
        name: "catppuccin-mocha",
        light: false,
        rgb: [
            [0xcd, 0xd6, 0xf4],
            [0x89, 0xb4, 0xfa],
            [0x7f, 0x84, 0x9c],
            [0x45, 0x47, 0x5a],
            [0xf9, 0xe2, 0xaf],
            [0x94, 0xe2, 0xd5],
            [0xa6, 0xe3, 0xa1],
            [0xf3, 0x8b, 0xa8],
            [0xcb, 0xa6, 0xf7],
            [0x89, 0xdc, 0xeb],
            [0x31, 0x32, 0x44],
        ],
    },
    Theme {
        name: "catppuccin-latte",
        light: true,
        rgb: [
            [0x4c, 0x4f, 0x69],
            [0x1e, 0x66, 0xf5],
            [0x8c, 0x8f, 0xa1],
            [0xcc, 0xd0, 0xda],
            [0xdf, 0x8e, 0x1d],
            [0x17, 0x92, 0x99],
            [0x40, 0xa0, 0x2b],
            [0xd2, 0x0f, 0x39],
            [0x88, 0x39, 0xef],
            [0x04, 0xa5, 0xe5],
            [0xcc, 0xd0, 0xda],
        ],
    },
    Theme {
        name: "tokyo-night",
        light: false,
        rgb: [
            [0xc0, 0xca, 0xf5],
            [0x7a, 0xa2, 0xf7],
            [0x56, 0x5f, 0x89],
            [0x3b, 0x42, 0x61],
            [0xe0, 0xaf, 0x68],
            [0x7d, 0xcf, 0xff],
            [0x9e, 0xce, 0x6a],
            [0xf7, 0x76, 0x8e],
            [0xbb, 0x9a, 0xf7],
            [0x2a, 0xc3, 0xde],
            [0x28, 0x34, 0x57],
        ],
    },
    Theme {
        name: "nord",
        light: false,
        rgb: [
            [0xd8, 0xde, 0xe9],
            [0x88, 0xc0, 0xd0],
            [0x7b, 0x88, 0xa1],
            [0x4c, 0x56, 0x6a],
            [0xeb, 0xcb, 0x8b],
            [0x8f, 0xbc, 0xbb],
            [0xa3, 0xbe, 0x8c],
            [0xbf, 0x61, 0x6a],
            [0xb4, 0x8e, 0xad],
            [0x81, 0xa1, 0xc1],
            [0x3b, 0x42, 0x52],
        ],
    },
    // Colour off; the slots are never read.
    Theme {
        name: "mono",
        light: false,
        rgb: [[0, 0, 0]; 11],
    },
];

/// The nearest xterm 256-colour index: the 6×6×6 cube or the grey ramp.
pub fn to_256([r, g, b]: [u8; 3]) -> u8 {
    const STEPS: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let near = |v: u8| {
        (0..6)
            .min_by_key(|&i| (STEPS[i] - v as i32).abs())
            .unwrap_or(0)
    };
    let (ri, gi, bi) = (near(r), near(g), near(b));
    let dist = |c: [i32; 3]| {
        (c[0] - r as i32).pow(2) + (c[1] - g as i32).pow(2) + (c[2] - b as i32).pow(2)
    };
    let cube = dist([STEPS[ri], STEPS[gi], STEPS[bi]]);
    let mean = (r as i32 + g as i32 + b as i32) / 3;
    let step = ((mean - 8).max(0) / 10).min(23);
    let grey = 8 + 10 * step;
    if dist([grey; 3]) < cube {
        232 + step as u8
    } else {
        16 + 36 * ri as u8 + 6 * gi as u8 + bi as u8
    }
}

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
        assert_eq!(
            names(),
            vec![
                "default",
                "default-light",
                "solarized-dark",
                "solarized-light",
                "gruvbox-dark",
                "gruvbox-light",
                "catppuccin-mocha",
                "catppuccin-latte",
                "tokyo-night",
                "nord",
                "mono",
            ]
        );
        assert!(by_name("neon").is_none());
    }

    #[test]
    fn every_theme_keeps_its_roles_apart() {
        for t in THEMES.iter().filter(|t| t.name != "mono") {
            let text = t.rgb(Slot::Text);
            for slot in [Slot::User, Slot::Muted, Slot::Border, Slot::Select] {
                assert_ne!(t.rgb(slot), text, "{} {slot:?} matches text", t.name);
            }
            assert_ne!(t.rgb(Slot::Success), t.rgb(Slot::Error), "{}", t.name);
            // Light themes use dark text, dark themes light text.
            let luma = |[r, g, b]: [u8; 3]| r as u32 * 3 + g as u32 * 6 + b as u32;
            assert_eq!(luma(text) < 1280, t.light, "{} text vs light flag", t.name);
        }
    }

    #[test]
    fn fallbacks_land_on_the_cube_or_the_ramp() {
        assert_eq!(to_256([255, 0, 0]), 196);
        assert_eq!(to_256([0, 0, 255]), 21);
        assert_eq!(to_256([255, 255, 255]), 231);
        assert_eq!(to_256([0x80, 0x80, 0x80]), 244);
        assert_eq!(to_256([0x26, 0x8b, 0xd2]), 32);
    }
}
