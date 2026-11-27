//! Phosphor palettes. Every color is RGB so the effects pass can dim and blend them.

use ratatui::style::Color;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub name: &'static str,
    /// Screen background.
    pub bg: Color,
    /// Main phosphor color (text, borders).
    pub fg: Color,
    /// Secondary text, labels.
    pub dim: Color,
    /// Grid dots, empty segments, decoration.
    pub faint: Color,
    /// Highlights, values that matter.
    pub bright: Color,
    /// High load.
    pub warn: Color,
    /// Critical load.
    pub crit: Color,
}

pub const THEMES: [Theme; 6] = [
    Theme {
        name: "AMBER",
        bg: Color::Rgb(16, 10, 3),
        fg: Color::Rgb(255, 170, 30),
        dim: Color::Rgb(160, 98, 22),
        faint: Color::Rgb(72, 44, 12),
        bright: Color::Rgb(255, 226, 150),
        warn: Color::Rgb(255, 110, 30),
        crit: Color::Rgb(255, 52, 36),
    },
    Theme {
        name: "PHOSPHOR",
        bg: Color::Rgb(3, 13, 6),
        fg: Color::Rgb(70, 255, 130),
        dim: Color::Rgb(34, 150, 76),
        faint: Color::Rgb(14, 62, 32),
        bright: Color::Rgb(200, 255, 210),
        warn: Color::Rgb(230, 235, 70),
        crit: Color::Rgb(255, 84, 60),
    },
    Theme {
        name: "COBALT",
        bg: Color::Rgb(3, 8, 18),
        fg: Color::Rgb(110, 200, 255),
        dim: Color::Rgb(52, 112, 165),
        faint: Color::Rgb(20, 44, 72),
        bright: Color::Rgb(225, 246, 255),
        warn: Color::Rgb(255, 190, 64),
        crit: Color::Rgb(255, 70, 96),
    },
    Theme {
        name: "LOCKDOWN",
        bg: Color::Rgb(18, 3, 3),
        fg: Color::Rgb(255, 72, 52),
        dim: Color::Rgb(155, 40, 30),
        faint: Color::Rgb(64, 16, 12),
        bright: Color::Rgb(255, 196, 176),
        warn: Color::Rgb(255, 205, 64),
        crit: Color::Rgb(255, 255, 255),
    },
    // teal line-art on near-black, like a silo's schematic screens
    Theme {
        name: "SILO",
        bg: Color::Rgb(3, 14, 13),
        fg: Color::Rgb(86, 226, 196),
        dim: Color::Rgb(42, 138, 120),
        faint: Color::Rgb(14, 54, 48),
        bright: Color::Rgb(206, 255, 240),
        warn: Color::Rgb(238, 206, 92),
        crit: Color::Rgb(255, 92, 72),
    },
    // the secret one: white and red, unlocked by an old code (always last, see SECRET_THEME)
    Theme {
        name: "SAFEGUARD",
        bg: Color::Rgb(12, 6, 8),
        fg: Color::Rgb(236, 232, 226),
        dim: Color::Rgb(196, 72, 64),
        faint: Color::Rgb(72, 26, 26),
        bright: Color::Rgb(255, 255, 255),
        warn: Color::Rgb(255, 172, 64),
        crit: Color::Rgb(255, 56, 44),
    },
];

/// Index of the secret palette, which `t` skips until it is unlocked.
pub const SECRET_THEME: usize = THEMES.len() - 1;

impl Theme {
    /// Color for a load level in `0.0..=1.0`: dim at rest, warning and critical when it climbs.
    pub fn level(&self, frac: f64) -> Color {
        let f = frac.clamp(0.0, 1.0);
        if f < 0.6 {
            lerp(self.dim, self.fg, f / 0.6)
        } else if f < 0.85 {
            lerp(self.fg, self.warn, (f - 0.6) / 0.25)
        } else {
            lerp(self.warn, self.crit, (f - 0.85) / 0.15)
        }
    }
}

pub fn rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (128, 128, 128),
    }
}

pub fn lerp(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (ar, ag, ab) = rgb(a);
    let (br, bg, bb) = rgb(b);
    let mix = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb))
}

/// Multiplies an RGB color by `k` (used by the scanline pass).
pub fn scale(c: Color, k: f64) -> Color {
    match c {
        Color::Rgb(r, g, b) => {
            let m = |x: u8| (x as f64 * k).clamp(0.0, 255.0) as u8;
            Color::Rgb(m(r), m(g), m(b))
        }
        other => other,
    }
}
