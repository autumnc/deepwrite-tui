use ratatui::style::{Color, Style};

/// All color slots used by the application.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    // Base colors
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,

    // Status bar
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,

    // Dimmed / unfocused text
    pub dimmed_fg: Color,

    // File browser
    pub browser_dir: Color,
    pub browser_selected_bg: Color,
    pub browser_selected_fg: Color,

    // Markdown syntax — headings have per-level colors for visual hierarchy
    pub md_heading_1: Color,
    pub md_heading_2: Color,
    pub md_heading_3: Color,
    pub md_heading_4: Color,
    pub md_heading_5: Color,
    pub md_heading_6: Color,
    pub md_link: Color,
    pub md_code: Color,
    pub md_muted: Color,
    pub md_underline: Color,
    pub md_highlight_bg: Color,
    pub md_block_quote: Color,
}

impl Theme {
    /// Return the heading color for a given level (1-6).
    pub fn heading_color(&self, level: usize) -> Color {
        match level {
            1 => self.md_heading_1,
            2 => self.md_heading_2,
            3 => self.md_heading_3,
            4 => self.md_heading_4,
            5 => self.md_heading_5,
            _ => self.md_heading_6,
        }
    }
}

impl Theme {
    /// Light theme colors.
    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(0xF5, 0xF6, 0xF6),
            fg: Color::Rgb(0x42, 0x42, 0x42),
            accent: Color::Rgb(0x00, 0xBA, 0xFF),

            status_bar_bg: Color::Rgb(0xEA, 0xEA, 0xEA),
            status_bar_fg: Color::Rgb(0x99, 0x99, 0x99),

            dimmed_fg: Color::Rgb(0xC0, 0xC0, 0xC0),

            browser_dir: Color::Rgb(0x40, 0x80, 0xA0),
            browser_selected_bg: Color::Rgb(0x00, 0xBA, 0xFF), // accent
            browser_selected_fg: Color::Rgb(0xFF, 0xFF, 0xFF), // white

            md_heading_1: Color::Rgb(0x30, 0x60, 0x80),
            md_heading_2: Color::Rgb(0x38, 0x70, 0x90),
            md_heading_3: Color::Rgb(0x40, 0x80, 0xA0),
            md_heading_4: Color::Rgb(0x48, 0x90, 0xB0),
            md_heading_5: Color::Rgb(0x50, 0xA0, 0xC0),
            md_heading_6: Color::Rgb(0x58, 0xB0, 0xD0),
            md_link: Color::Rgb(0x2A, 0x7A, 0xB5),
            md_code: Color::Rgb(0x6B, 0x8E, 0x6B),
            md_muted: Color::Rgb(0x88, 0x88, 0x88),
            md_underline: Color::Rgb(0xD4, 0x4A, 0x4A),
            md_highlight_bg: Color::Rgb(0xFF, 0xEB, 0x3B),
            md_block_quote: Color::Rgb(0x66, 0x66, 0x66),
        }
    }

    /// Dark theme colors.
    pub fn dark() -> Self {
        Self {
            bg: Color::Rgb(0x26, 0x26, 0x26),
            fg: Color::Rgb(0xC5, 0xC9, 0xC6),
            accent: Color::Rgb(0x15, 0xBD, 0xEC),

            status_bar_bg: Color::Rgb(0x1E, 0x1E, 0x1E),
            status_bar_fg: Color::Rgb(0x66, 0x66, 0x66),

            dimmed_fg: Color::Rgb(0x55, 0x55, 0x55),

            browser_dir: Color::Rgb(0x7A, 0xA4, 0xC2),
            browser_selected_bg: Color::Rgb(0x15, 0xBD, 0xEC), // accent
            browser_selected_fg: Color::Rgb(0xFF, 0xFF, 0xFF), // white

            md_heading_1: Color::Rgb(0x7A, 0xA4, 0xC2),
            md_heading_2: Color::Rgb(0x7A, 0xA4, 0xC2),
            md_heading_3: Color::Rgb(0x7A, 0xA4, 0xC2),
            md_heading_4: Color::Rgb(0x7A, 0xA4, 0xC2),
            md_heading_5: Color::Rgb(0x7A, 0xA4, 0xC2),
            md_heading_6: Color::Rgb(0x7A, 0xA4, 0xC2),
            md_link: Color::Rgb(0x5B, 0xA3, 0xD9),
            md_code: Color::Rgb(0x8F, 0xB8, 0x8F),
            md_muted: Color::Rgb(0x77, 0x77, 0x77),
            md_underline: Color::Rgb(0xE0, 0x6C, 0x6C),
            md_highlight_bg: Color::Rgb(0x8B, 0x80, 0x00),
            md_block_quote: Color::Rgb(0x9E, 0x9E, 0x9E),
        }
    }

    /// High-contrast theme optimized for dark backgrounds like #101010.
    /// All heading levels get bold rendering and distinct colors for
    /// clear visual hierarchy. Foreground text is bright white for
    /// maximum readability.
    pub fn high_contrast() -> Self {
        Self {
            bg: Color::Rgb(0x10, 0x10, 0x10),
            fg: Color::Rgb(0xF0, 0xF0, 0xF0),
            accent: Color::Rgb(0xFF, 0xB7, 0x4D),

            status_bar_bg: Color::Rgb(0x08, 0x08, 0x08),
            status_bar_fg: Color::Rgb(0x88, 0x88, 0x88),

            dimmed_fg: Color::Rgb(0x61, 0x61, 0x61),

            browser_dir: Color::Rgb(0x90, 0xCA, 0xF9),
            browser_selected_bg: Color::Rgb(0xFF, 0xB7, 0x4D), // accent
            browser_selected_fg: Color::Rgb(0x10, 0x10, 0x10), // dark on accent

            // Headings: warm-to-cool gradient for clear hierarchy
            md_heading_1: Color::Rgb(0xFF, 0xFF, 0xFF), // H1: pure white
            md_heading_2: Color::Rgb(0xFF, 0xD7, 0x4F), // H2: warm gold
            md_heading_3: Color::Rgb(0xFF, 0xA7, 0x26), // H3: orange
            md_heading_4: Color::Rgb(0x64, 0xB5, 0xF6), // H4: blue
            md_heading_5: Color::Rgb(0x81, 0xC7, 0x84), // H5: green
            md_heading_6: Color::Rgb(0xBA, 0x68, 0xC8), // H6: purple
            md_link: Color::Rgb(0x42, 0xA5, 0xF5),
            md_code: Color::Rgb(0xA5, 0xD6, 0xA7),
            md_muted: Color::Rgb(0x9E, 0x9E, 0x9E),
            md_underline: Color::Rgb(0xFF, 0x8A, 0x65),
            md_highlight_bg: Color::Rgb(0xFF, 0xD7, 0x00),
            md_block_quote: Color::Rgb(0xB0, 0xB0, 0xB0),
        }
    }

    /// Create a theme from config values.
    pub fn from_config(mode_str: &str, focus_opacity: u8) -> Self {
        let mut theme = match mode_str {
            "light" => Self::light(),
            "dark" => Self::dark(),
            "high_contrast" => Self::high_contrast(),
            _ => Self::detect_system(),
        };
        theme.apply_focus_opacity(focus_opacity);
        theme
    }

    /// Detect whether the terminal is light or dark by inspecting
    /// the `COLORFGBG` environment variable. Falls back to dark theme.
    ///
    /// `COLORFGBG` is typically "fg;bg" where bg >= 8 means dark background.
    pub fn detect_system() -> Self {
        if let Ok(val) = std::env::var("COLORFGBG") {
            if let Some(bg_str) = val.rsplit(';').next() {
                if let Ok(bg_num) = bg_str.parse::<u8>() {
                    // Low background number (0-6) typically means dark;
                    // high number (7+) typically means light.
                    if bg_num >= 7 && bg_num != 8 {
                        return Self::light();
                    }
                }
            }
        }
        Self::dark()
    }

    /// Base style with the theme's foreground and background colors.
    pub fn base_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    /// Style for dimmed / unfocused text.
    pub fn dimmed_style(&self) -> Style {
        Style::default().fg(self.dimmed_fg).bg(self.bg)
    }

    fn apply_focus_opacity(&mut self, opacity: u8) {
        let opacity = opacity.clamp(10, 60);
        self.dimmed_fg = blend_color(self.fg, self.bg, opacity);
    }
}

fn blend_color(fg: Color, bg: Color, opacity: u8) -> Color {
    match (fg, bg) {
        (Color::Rgb(fr, fg, fb), Color::Rgb(br, bg, bb)) => Color::Rgb(
            blend_channel(fr, br, opacity),
            blend_channel(fg, bg, opacity),
            blend_channel(fb, bb, opacity),
        ),
        _ => fg,
    }
}

fn blend_channel(foreground: u8, background: u8, opacity: u8) -> u8 {
    let foreground = u16::from(foreground);
    let background = u16::from(background);
    let opacity = u16::from(opacity);
    let blended = foreground * opacity + background * (100 - opacity);
    ((blended + 50) / 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_opacity_updates_dimmed_color() {
        let theme = Theme::from_config("dark", 10);
        assert_eq!(theme.dimmed_fg, Color::Rgb(0x36, 0x36, 0x36));
        assert_eq!(theme.md_heading_1, Color::Rgb(0x7A, 0xA4, 0xC2));
    }
}
