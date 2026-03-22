# Deepwrite TUI MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a terminal Markdown writing tool with Yazi-style file browsing and iA Writer-style Focus Mode, distributed as a single Rust binary.

**Architecture:** Ratatui + crossterm app shell with dual-panel layout (file browser left, editor right). edtui as the text editing engine, forked as an in-workspace crate to allow internal modifications. Customized with a custom `KeyEventHandler` for non-modal editing and the `Highlight` API for Markdown coloring and Focus Mode dimming. tree-sitter for Markdown parsing. notify for detecting external file changes (e.g., Claude Code editing the same file).

**Why fork edtui instead of using as a dependency:** The spec requires non-modal editing, custom scroll-past-end behavior, and potential internal changes to the rendering pipeline for Focus Mode dimming integration. While edtui's public API covers many needs (Highlights, custom keymaps), having the source in-workspace ensures we can modify internals if the public API proves insufficient — without being blocked mid-build.

**Tech Stack:** Rust, ratatui, crossterm, edtui (forked), tree-sitter, tree-sitter-markdown, unicode-segmentation, encoding_rs, chardetng, notify, clap, toml, serde

---

## File Structure

```
deepwrite/
├── Cargo.toml                   # Workspace root
├── crates/
│   └── edtui/                   # Forked edtui crate (local workspace member)
├── src/
│   ├── main.rs                  # Entry point: CLI parsing, terminal setup, run app
│   ├── app.rs                   # App state, event loop, mode management (Browse/Edit)
│   ├── config.rs                # Config loading from TOML with defaults
│   ├── theme.rs                 # Color system: light/dark themes, all color constants
│   ├── ui/
│   │   ├── mod.rs               # Re-exports
│   │   ├── layout.rs            # Dual-panel layout calculation, panel toggling
│   │   └── status_bar.rs        # Status bar widget (filename, word/char count)
│   ├── browser/
│   │   ├── mod.rs               # Re-exports
│   │   ├── entries.rs           # Directory listing: read, filter (.md/.txt), sort
│   │   ├── navigator.rs         # Navigation state: selected index, current directory
│   │   ├── actions.rs           # Create/rename/delete files, fuzzy search
│   │   └── widget.rs            # Ratatui widget rendering for file browser
│   ├── editor/
│   │   ├── mod.rs               # Re-exports, EditorWrapper struct
│   │   ├── keymap.rs            # Custom non-modal KeyEventHandler for edtui
│   │   ├── markdown.rs          # tree-sitter → Highlight ranges for Markdown coloring
│   │   ├── focus.rs             # Focus Mode engine: state machine, dimming highlights
│   │   ├── sentence.rs          # Sentence boundary detection (Chinese/English)
│   │   ├── formatting.rs        # Ctrl+B/I/K/U/1-6 text manipulation
│   │   └── word_count.rs        # Word/character counting with unicode-segmentation
│   └── services/
│       ├── mod.rs               # Re-exports
│       ├── auto_save.rs         # Debounced auto-save, immediate save (Ctrl+S)
│       └── file_io.rs           # Load/save files, encoding detection
├── tests/
│   ├── config_test.rs           # Config parsing: defaults, overrides, invalid TOML
│   ├── entries_test.rs          # File listing: filter, sort, hidden files
│   ├── sentence_test.rs         # Sentence boundary: English, Chinese, mixed, edge cases
│   ├── word_count_test.rs       # Word counting: English, Chinese, mixed
│   └── formatting_test.rs       # Format toggling: wrap, unwrap, toggle
```

Each file has one clear responsibility. Files that change together (e.g., `focus.rs` and `sentence.rs`) live in the same module. The `editor/` module is the densest — it contains the core writing experience.

---

## Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/app.rs`

- [ ] **Step 1: Initialize Cargo project**

```bash
cd /Users/tomdhyang/Projects/Deepwrite
cargo init --name deepwrite
```

- [ ] **Step 2: Add core dependencies to Cargo.toml**

```toml
[package]
name = "deepwrite"
version = "0.1.0"
edition = "2021"
description = "A terminal Markdown writing tool with Focus Mode"

[dependencies]
ratatui = "0.30"
crossterm = "0.29"
anyhow = "1"
```

- [ ] **Step 3: Write minimal app loop in main.rs**

```rust
// src/main.rs
mod app;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io::stdout;

fn main() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = app::run(&mut terminal);

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    result
}
```

- [ ] **Step 4: Write app.rs with basic event loop**

```rust
// src/app.rs
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(
                Paragraph::new("Deepwrite — press q to quit"),
                area,
            );
        })?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}
```

- [ ] **Step 5: Build and run**

```bash
cargo build
cargo run
```

Expected: Terminal shows "Deepwrite — press q to quit". Press `q` exits cleanly.

- [ ] **Step 6: Commit**

```bash
git init
echo "target/" > .gitignore
git add Cargo.toml src/ .gitignore
git commit -m "feat: project scaffold with basic ratatui app loop"
```

---

## Task 2: Config System

**Files:**
- Create: `src/config.rs`
- Create: `tests/config_test.rs`
- Modify: `src/main.rs` (add mod declaration)
- Add dep: `toml`, `serde` in `Cargo.toml`

- [ ] **Step 1: Add dependencies**

Add to `Cargo.toml`:
```toml
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "6"
```

- [ ] **Step 2: Write config test**

```rust
// tests/config_test.rs
use deepwrite::config::Config;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.editor.line_width, 72);
    assert!(config.editor.auto_save);
    assert_eq!(config.editor.auto_save_delay_ms, 500);
    assert_eq!(config.focus.mode, "off");
    assert_eq!(config.focus.opacity, 30);
    assert_eq!(config.theme.mode, "system");
    assert!(!config.browser.show_hidden);
    assert_eq!(config.browser.panel_width, 30);
}

#[test]
fn test_parse_partial_toml() {
    let toml_str = r#"
[editor]
line_width = 80

[focus]
opacity = 50
"#;
    let config: Config = Config::from_toml_str(toml_str).unwrap();
    assert_eq!(config.editor.line_width, 80);
    assert_eq!(config.focus.opacity, 50);
    // Defaults for unspecified fields
    assert!(config.editor.auto_save);
    assert_eq!(config.focus.mode, "off");
}

#[test]
fn test_invalid_toml_returns_error() {
    let result = Config::from_toml_str("not valid toml [[[");
    assert!(result.is_err());
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test --test config_test
```

Expected: FAIL — `config` module does not exist.

- [ ] **Step 4: Implement config.rs**

```rust
// src/config.rs
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub editor: EditorConfig,
    pub focus: FocusConfig,
    pub theme: ThemeConfig,
    pub browser: BrowserConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub line_width: u16,
    pub auto_save: bool,
    pub auto_save_delay_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct FocusConfig {
    pub mode: String,
    pub opacity: u8,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct BrowserConfig {
    pub show_hidden: bool,
    pub panel_width: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: EditorConfig::default(),
            focus: FocusConfig::default(),
            theme: ThemeConfig::default(),
            browser: BrowserConfig::default(),
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            line_width: 72,
            auto_save: true,
            auto_save_delay_ms: 500,
        }
    }
}

impl Default for FocusConfig {
    fn default() -> Self {
        Self {
            mode: "off".to_string(),
            opacity: 30,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: "system".to_string(),
        }
    }
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            panel_width: 30,
        }
    }
}

impl Config {
    pub fn from_toml_str(s: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(s)?)
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => Self::from_toml_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("deepwrite")
            .join("config.toml")
    }
}
```

- [ ] **Step 5: Add `pub mod config;` to main.rs and make crate a lib+bin**

Create `src/lib.rs`:
```rust
pub mod config;
```

Update `src/main.rs` to add:
```rust
mod app;
use deepwrite::config::Config;
```

- [ ] **Step 6: Run tests**

```bash
cargo test --test config_test
```

Expected: All 3 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: config system with TOML loading and defaults"
```

---

## Task 3: Theme System

**Files:**
- Create: `src/theme.rs`
- Modify: `src/lib.rs` (add mod)

- [ ] **Step 1: Implement theme.rs**

```rust
// src/theme.rs
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub dimmed_fg: Color,
    pub browser_dir: Color,
    pub browser_selected_bg: Color,
    pub browser_selected_fg: Color,
    // Markdown colors
    pub md_heading: Color,
    pub md_link: Color,
    pub md_code: Color,
    pub md_muted: Color,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            bg: Color::Rgb(245, 246, 246),       // #F5F6F6
            fg: Color::Rgb(66, 66, 66),          // #424242
            accent: Color::Rgb(0, 186, 255),     // #00BAFF
            status_bar_bg: Color::Rgb(234, 234, 234), // #EAEAEA
            status_bar_fg: Color::Rgb(153, 153, 153), // #999999
            dimmed_fg: Color::Rgb(192, 192, 192),     // #C0C0C0
            browser_dir: Color::Rgb(64, 128, 160),    // #4080A0
            browser_selected_bg: Color::Rgb(0, 186, 255), // accent
            browser_selected_fg: Color::Rgb(255, 255, 255),
            md_heading: Color::Rgb(64, 128, 160),     // #4080A0
            md_link: Color::Rgb(42, 122, 181),        // #2A7AB5
            md_code: Color::Rgb(107, 142, 107),       // #6B8E6B
            md_muted: Color::Rgb(136, 136, 136),      // #888888
        }
    }

    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            bg: Color::Rgb(29, 31, 32),          // #1D1F20
            fg: Color::Rgb(197, 201, 198),       // #C5C9C6
            accent: Color::Rgb(21, 189, 236),    // #15BDEC
            status_bar_bg: Color::Rgb(22, 24, 25),    // #161819
            status_bar_fg: Color::Rgb(102, 102, 102), // #666666
            dimmed_fg: Color::Rgb(85, 85, 85),        // #555555
            browser_dir: Color::Rgb(122, 164, 194),   // #7AA4C2
            browser_selected_bg: Color::Rgb(21, 189, 236), // accent
            browser_selected_fg: Color::Rgb(255, 255, 255),
            md_heading: Color::Rgb(122, 164, 194),    // #7AA4C2
            md_link: Color::Rgb(91, 163, 217),        // #5BA3D9
            md_code: Color::Rgb(143, 184, 143),       // #8FB88F
            md_muted: Color::Rgb(119, 119, 119),      // #777777
        }
    }

    pub fn from_config(mode_str: &str) -> Self {
        match mode_str {
            "light" => Self::light(),
            "dark" => Self::dark(),
            "system" | _ => Self::detect_system(),
        }
    }

    fn detect_system() -> Self {
        // Try to detect terminal background color.
        // 1. Check $COLORTERM for truecolor support indicator (not theme detection)
        // 2. Use crossterm's background color query (OSC 11) if available
        // 3. Heuristic: check common env vars (COLORFGBG, TERMINAL_EMULATOR)
        // 4. Fallback: dark theme (most terminal users use dark backgrounds)
        //
        // crossterm does not expose a stable background-query API as of v0.29.
        // Practical approach: check COLORFGBG env var (set by some terminals).
        // Format: "foreground;background" where 0-6 = dark, 7-15 = light.
        if let Ok(val) = std::env::var("COLORFGBG") {
            if let Some(bg_str) = val.split(';').last() {
                if let Ok(bg) = bg_str.parse::<u8>() {
                    if bg >= 7 {
                        return Self::light();
                    }
                }
            }
        }
        Self::dark()
    }

    pub fn base_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn dimmed_style(&self) -> Style {
        Style::default().fg(self.dimmed_fg).bg(self.bg)
    }
}
```

- [ ] **Step 2: Add `pub mod theme;` to lib.rs**

- [ ] **Step 3: Build to verify**

```bash
cargo build
```

Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add src/theme.rs src/lib.rs
git commit -m "feat: theme system with light/dark color definitions"
```

---

## Task 4: App Shell with Dual-Panel Layout

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/layout.rs`
- Rewrite: `src/app.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create layout module**

```rust
// src/ui/mod.rs
pub mod layout;

// src/ui/layout.rs
use ratatui::prelude::*;

pub struct PanelLayout {
    pub browser: Rect,
    pub editor: Rect,
    pub status_bar: Rect,
}

pub fn compute_layout(area: Rect, browser_width: u16, show_browser: bool) -> PanelLayout {
    // Split off status bar (1 line at bottom)
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let status_bar = vertical[1];

    if show_browser {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(browser_width),
                Constraint::Min(1),
            ])
            .split(main_area);

        PanelLayout {
            browser: horizontal[0],
            editor: horizontal[1],
            status_bar,
        }
    } else {
        PanelLayout {
            browser: Rect::default(),
            editor: main_area,
            status_bar,
        }
    }
}
```

- [ ] **Step 2: Rewrite app.rs with mode management**

```rust
// src/app.rs
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::config::Config;
use crate::theme::Theme;
use crate::ui::layout;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Browse,
    Edit,
}

pub struct App {
    pub mode: AppMode,
    pub config: Config,
    pub theme: Theme,
    pub show_browser: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        let theme = Theme::from_config(&config.theme.mode);
        Self {
            mode: AppMode::Browse,
            config,
            theme,
            show_browser: true,
            should_quit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_event()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let panels = layout::compute_layout(
            frame.area(),
            self.config.browser.panel_width,
            self.show_browser,
        );

        // Background
        frame.render_widget(
            Block::default().style(self.theme.base_style()),
            frame.area(),
        );

        // Browser panel
        if self.show_browser {
            let browser_block = Block::default()
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(self.theme.md_muted))
                .style(self.theme.base_style());
            frame.render_widget(
                Paragraph::new("File Browser").block(browser_block),
                panels.browser,
            );
        }

        // Editor panel
        let mode_label = match self.mode {
            AppMode::Browse => "[Browse]",
            AppMode::Edit => "[Edit]",
        };
        frame.render_widget(
            Paragraph::new(format!("Editor {}", mode_label))
                .style(self.theme.base_style()),
            panels.editor,
        );

        // Status bar
        frame.render_widget(
            Paragraph::new(" Deepwrite")
                .style(Style::default()
                    .fg(self.theme.status_bar_fg)
                    .bg(self.theme.status_bar_bg)),
            panels.status_bar,
        );
    }

    fn handle_event(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            match self.mode {
                AppMode::Browse => self.handle_browse_key(key),
                AppMode::Edit => self.handle_edit_key(key),
            }
        }
        Ok(())
    }

    fn handle_browse_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Enter => self.mode = AppMode::Edit,
            _ => {}
        }
    }

    fn handle_edit_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = AppMode::Browse,
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.show_browser = !self.show_browser;
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 3: Update main.rs to use App**

```rust
// src/main.rs
mod app;

use anyhow::Result;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use deepwrite::config::Config;
use ratatui::prelude::*;
use std::io::stdout;

fn main() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let config = Config::load();
    let mut app = app::App::new(config);
    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    result
}
```

- [ ] **Step 4: Update lib.rs**

```rust
pub mod config;
pub mod theme;
pub mod ui;
```

- [ ] **Step 5: Build and run**

```bash
cargo run
```

Expected: Two-panel layout visible. `q` quits. `Enter` switches to Edit mode (label changes). `Esc` returns to Browse. `Ctrl+E` toggles left panel.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: app shell with dual-panel layout and mode switching"
```

---

## Task 5: File Browser — Directory Listing

**Files:**
- Create: `src/browser/mod.rs`, `src/browser/entries.rs`, `src/browser/navigator.rs`, `src/browser/widget.rs`
- Create: `tests/entries_test.rs`
- Modify: `src/lib.rs`, `src/app.rs`

- [ ] **Step 1: Write entries test**

```rust
// tests/entries_test.rs
use deepwrite::browser::entries::{list_entries, Entry, EntryKind};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_filters_non_markdown_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("readme.md"), "# Hello").unwrap();
    fs::write(dir.path().join("notes.txt"), "notes").unwrap();
    fs::write(dir.path().join("image.png"), "binary").unwrap();
    fs::write(dir.path().join("code.rs"), "fn main(){}").unwrap();

    let entries = list_entries(dir.path(), false).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"readme.md"));
    assert!(names.contains(&"notes.txt"));
    assert!(!names.contains(&"image.png"));
    assert!(!names.contains(&"code.rs"));
}

#[test]
fn test_directories_first_then_alphabetical() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("beta")).unwrap();
    fs::create_dir(dir.path().join("alpha")).unwrap();
    fs::write(dir.path().join("zebra.md"), "").unwrap();
    fs::write(dir.path().join("apple.md"), "").unwrap();

    let entries = list_entries(dir.path(), false).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "beta", "apple.md", "zebra.md"]);
}

#[test]
fn test_hides_dotfiles_by_default() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join(".env"), "SECRET=x").unwrap();
    fs::write(dir.path().join("readme.md"), "").unwrap();

    let entries = list_entries(dir.path(), false).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["readme.md"]);
}

#[test]
fn test_shows_dotfiles_when_enabled() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join("readme.md"), "").unwrap();

    let entries = list_entries(dir.path(), true).unwrap();
    assert!(entries.len() >= 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Add `tempfile = "3"` to `[dev-dependencies]` in Cargo.toml.

```bash
cargo test --test entries_test
```

Expected: FAIL — module does not exist.

- [ ] **Step 3: Implement entries.rs**

```rust
// src/browser/entries.rs
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum EntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
}

pub fn list_entries(dir: &Path, show_hidden: bool) -> anyhow::Result<Vec<Entry>> {
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Filter hidden files
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            entries.push(Entry { name, kind: EntryKind::Directory });
        } else if file_type.is_file() {
            // Only show .md and .txt files
            if name.ends_with(".md") || name.ends_with(".txt") {
                entries.push(Entry { name, kind: EntryKind::File });
            }
        }
    }

    // Sort: directories first, then alphabetical
    entries.sort_by(|a, b| {
        match (&a.kind, &b.kind) {
            (EntryKind::Directory, EntryKind::File) => std::cmp::Ordering::Less,
            (EntryKind::File, EntryKind::Directory) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}
```

- [ ] **Step 4: Create browser module files**

```rust
// src/browser/mod.rs
pub mod entries;
pub mod navigator;
pub mod widget;
pub mod actions;

// src/browser/navigator.rs
use std::path::PathBuf;
use super::entries::{list_entries, Entry};

pub struct Navigator {
    pub current_dir: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub show_hidden: bool,
}

impl Navigator {
    pub fn new(dir: PathBuf, show_hidden: bool) -> Self {
        let entries = list_entries(&dir, show_hidden).unwrap_or_default();
        Self {
            current_dir: dir,
            entries,
            selected: 0,
            show_hidden,
        }
    }

    pub fn refresh(&mut self) {
        self.entries = list_entries(&self.current_dir, self.show_hidden).unwrap_or_default();
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.entries.get(self.selected)
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh();
    }
}

// src/browser/widget.rs
// Placeholder — implemented in Task 5 Step 6

// src/browser/actions.rs
// Placeholder — implemented in Task 13
```

- [ ] **Step 5: Add `pub mod browser;` to lib.rs**

- [ ] **Step 6: Implement browser widget**

```rust
// src/browser/widget.rs
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use super::entries::EntryKind;
use super::navigator::Navigator;
use crate::theme::Theme;

pub fn render_browser(frame: &mut Frame, area: Rect, nav: &Navigator, theme: &Theme) {
    let items: Vec<ListItem> = nav
        .entries
        .iter()
        .map(|entry| {
            let style = match entry.kind {
                EntryKind::Directory => Style::default().fg(theme.browser_dir),
                EntryKind::File => Style::default().fg(theme.fg),
            };
            let prefix = match entry.kind {
                EntryKind::Directory => "  ",
                EntryKind::File => "  ",
            };
            ListItem::new(format!("{}{}", prefix, entry.name)).style(style)
        })
        .collect();

    let title = nav
        .current_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| nav.current_dir.to_string_lossy().to_string());

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" {} ", title))
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(theme.md_muted))
                .style(theme.base_style()),
        )
        .highlight_style(
            Style::default()
                .fg(theme.browser_selected_fg)
                .bg(theme.browser_selected_bg),
        );

    let mut state = ListState::default().with_selected(Some(nav.selected));
    frame.render_stateful_widget(list, area, &mut state);
}
```

- [ ] **Step 7: Integrate browser into app.rs**

Update `app.rs`:
- Add `navigator: Navigator` field to `App`
- In `draw()`, call `render_browser()` for the browser panel
- In `handle_browse_key()`, handle `↑`/`↓`/`j`/`k` for navigation, `→`/`Enter` for enter, `←`/`Backspace` for go up, `.` for toggle hidden

- [ ] **Step 8: Run tests**

```bash
cargo test --test entries_test
```

Expected: All 4 tests PASS.

- [ ] **Step 9: Build and run**

```bash
cargo run
```

Expected: Left panel shows files in current directory (only .md/.txt and directories). Arrow keys navigate. `Enter` on a directory enters it. `←` goes up.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat: file browser with directory listing, filtering, and navigation"
```

---

## Task 6: Editor Integration (edtui)

**Files:**
- Create: `crates/edtui/` (forked crate)
- Create: `src/editor/mod.rs`, `src/editor/keymap.rs`
- Modify: `Cargo.toml`, `src/lib.rs`, `src/app.rs`

- [ ] **Step 1: Fork edtui into workspace**

```bash
# Clone edtui source into workspace
git clone https://github.com/preiter93/edtui.git crates/edtui
rm -rf crates/edtui/.git

# Set up as Cargo workspace
```

Update `Cargo.toml` to use workspace and path dependency:

```toml
# Root Cargo.toml
[workspace]
members = [".", "crates/edtui"]

[dependencies]
edtui = { path = "crates/edtui", default-features = false, features = ["arboard"] }
```

Disable `syntax-highlighting` feature (we'll use tree-sitter instead of syntect). Having edtui as a local crate allows internal modifications for Focus Mode rendering hooks and scroll-past-end behavior if the public API proves insufficient.

- [ ] **Step 2: Create custom non-modal keymap**

```rust
// src/editor/keymap.rs
use edtui::actions::*;
use edtui::EditorMode;
use edtui::events::key::{KeyEventHandler, KeyEventRegister, KeyInput};

pub fn build_non_modal_keymap() -> KeyEventHandler {
    let mut handler = KeyEventHandler::default();

    // Cursor movement
    handler.insert(KeyEventRegister::i(vec![KeyInput::up()]), motion::MoveUp(1));
    handler.insert(KeyEventRegister::i(vec![KeyInput::down()]), motion::MoveDown(1));
    handler.insert(KeyEventRegister::i(vec![KeyInput::left()]), motion::MoveLeft(1));
    handler.insert(KeyEventRegister::i(vec![KeyInput::right()]), motion::MoveRight(1));
    handler.insert(KeyEventRegister::i(vec![KeyInput::home()]), motion::MoveToLineStart);
    handler.insert(KeyEventRegister::i(vec![KeyInput::end()]), motion::MoveToLineEnd);

    // Editing
    handler.insert(KeyEventRegister::i(vec![KeyInput::backspace()]), delete::DeleteBeforeCursor);
    handler.insert(KeyEventRegister::i(vec![KeyInput::delete()]), delete::DeleteAtCursor);
    handler.insert(KeyEventRegister::i(vec![KeyInput::enter()]), insert::InsertNewline);

    // Clipboard
    handler.insert(KeyEventRegister::i(vec![KeyInput::ctrl('c')]), cpaste::Copy);
    handler.insert(KeyEventRegister::i(vec![KeyInput::ctrl('v')]), cpaste::Paste);
    handler.insert(KeyEventRegister::i(vec![KeyInput::ctrl('x')]), cpaste::Cut);

    // Undo/Redo
    handler.insert(KeyEventRegister::i(vec![KeyInput::ctrl('z')]), Undo);
    handler.insert(KeyEventRegister::i(vec![KeyInput::ctrl('y')]), Redo);

    // Select all
    handler.insert(KeyEventRegister::i(vec![KeyInput::ctrl('a')]), select::SelectAll);

    handler
}
```

Note: The exact edtui action types and `KeyInput` constructors will need to be verified against the actual edtui API during implementation. The structure above is the intent — adjust imports and constructor names as needed.

- [ ] **Step 3: Create editor module**

```rust
// src/editor/mod.rs
pub mod keymap;

use edtui::{EditorState, EditorView, EditorEventHandler, EditorTheme, Lines};
use ratatui::prelude::*;
use crate::theme::Theme;

pub struct EditorWrapper {
    pub state: EditorState,
    pub handler: EditorEventHandler,
}

impl EditorWrapper {
    pub fn new() -> Self {
        let key_handler = keymap::build_non_modal_keymap();
        let mut state = EditorState::default();
        // Start in Insert mode (non-modal: always inserting)
        state.mode = edtui::EditorMode::Insert;
        Self {
            state,
            handler: EditorEventHandler::new(key_handler),
        }
    }

    pub fn load_content(&mut self, content: &str) {
        self.state = EditorState::new(Lines::from(content));
        self.state.mode = edtui::EditorMode::Insert;
    }

    pub fn get_content(&self) -> String {
        self.state.lines.to_string()
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let editor_theme = EditorTheme::default()
            .base(theme.base_style())
            .cursor_style(Style::default().bg(theme.accent))
            .hide_status_line();

        let view = EditorView::new(&mut self.state)
            .theme(editor_theme)
            .wrap(true);

        frame.render_widget(view, area);
    }

    pub fn handle_event(&mut self, event: &crossterm::event::Event) {
        self.handler.on_event(event.clone(), &mut self.state);
    }
}
```

- [ ] **Step 4: Integrate editor into app.rs**

- Add `editor: EditorWrapper` field to `App`
- In `draw()`: render editor in the right panel area, but with centered 72-char content width by calculating inner padding
- In `handle_edit_key()`: pass events to `editor.handle_event()`, keep `Esc` and `Ctrl+E` handling in `App`
- When transitioning from Browse→Edit (opening a file): load file content into editor

- [ ] **Step 5: Add content centering logic**

Calculate the inner area for the editor: if the right panel is wider than 72 chars + padding, center the content area.

```rust
fn centered_editor_area(panel: Rect, line_width: u16) -> Rect {
    let content_width = line_width.min(panel.width);
    let padding = (panel.width.saturating_sub(content_width)) / 2;
    Rect {
        x: panel.x + padding,
        y: panel.y,
        width: content_width,
        height: panel.height,
    }
}
```

- [ ] **Step 6: Update lib.rs**

Add `pub mod editor;`

- [ ] **Step 7: Build and run**

```bash
cargo run
```

Expected: Navigate to a .md file in the browser, press `Enter` — file content loads in the right panel. Typing inserts text. Arrow keys move cursor. `Esc` returns to browse mode.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: edtui editor integration with non-modal keymap and centered content"
```

---

## Task 7: File I/O — Load, Save, Auto-save

**Files:**
- Create: `src/services/mod.rs`, `src/services/file_io.rs`, `src/services/auto_save.rs`
- Modify: `Cargo.toml`, `src/lib.rs`, `src/app.rs`

- [ ] **Step 1: Add dependencies**

```toml
encoding_rs = "0.8"
chardetng = "0.1"
```

- [ ] **Step 2: Implement file_io.rs**

```rust
// src/services/file_io.rs
use anyhow::Result;
use std::path::Path;

pub fn load_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    // Try UTF-8 first (fast path for most files)
    if let Ok(content) = String::from_utf8(bytes.clone()) {
        return Ok(content);
    }
    // Check for BOM
    if let Some((encoding, _)) = encoding_rs::Encoding::for_bom(&bytes) {
        let (content, _, _) = encoding.decode(&bytes);
        return Ok(content.to_string());
    }
    // Fallback: try common CJK encodings
    // chardetng provides automatic charset detection
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(&bytes, true);
    let encoding = detector.guess(None, true);
    let (content, _, had_errors) = encoding.decode(&bytes);
    if had_errors {
        anyhow::bail!("Failed to decode file: lossy conversion detected");
    }
    Ok(content.to_string())
}

pub fn save_file(path: &Path, content: &str) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let temp_path = dir.join(format!(".deepwrite-tmp-{}", std::process::id()));
    std::fs::write(&temp_path, content.as_bytes())?;
    std::fs::rename(&temp_path, path)?;
    Ok(())
}
```

- [ ] **Step 3: Implement auto_save.rs**

```rust
// src/services/auto_save.rs
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct AutoSave {
    pub path: Option<PathBuf>,
    pub delay: Duration,
    pub last_edit: Option<Instant>,
    pub last_save_content: String,
    pub dirty: bool,
}

impl AutoSave {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            path: None,
            delay: Duration::from_millis(delay_ms),
            last_edit: None,
            last_save_content: String::new(),
            dirty: false,
        }
    }

    pub fn mark_edited(&mut self) {
        self.last_edit = Some(Instant::now());
        self.dirty = true;
    }

    pub fn should_save(&self) -> bool {
        if !self.dirty || self.path.is_none() {
            return false;
        }
        match self.last_edit {
            Some(last) => last.elapsed() >= self.delay,
            None => false,
        }
    }

    pub fn save(&mut self, content: &str) -> anyhow::Result<()> {
        if let Some(ref path) = self.path {
            if content != self.last_save_content {
                super::file_io::save_file(path, content)?;
                self.last_save_content = content.to_string();
            }
        }
        self.dirty = false;
        self.last_edit = None;
        Ok(())
    }

    pub fn force_save(&mut self, content: &str) -> anyhow::Result<()> {
        self.dirty = true;
        self.save(content)
    }
}
```

- [ ] **Step 4: Create services module**

```rust
// src/services/mod.rs
pub mod auto_save;
pub mod file_io;
```

- [ ] **Step 5: Integrate into app.rs**

- Add `auto_save: AutoSave` to App
- On file open: set `auto_save.path`, load content
- On each keypress in Edit mode: call `auto_save.mark_edited()`
- In the event loop: check `auto_save.should_save()` with a tick-based polling (use `event::poll(Duration::from_millis(100))` instead of blocking `event::read()`)
- On `Ctrl+S`: call `auto_save.force_save()`

- [ ] **Step 6: Switch event loop to poll-based**

```rust
// In app.rs run() method:
loop {
    terminal.draw(|frame| self.draw(frame))?;

    // Check auto-save
    if self.auto_save.should_save() {
        let content = self.editor.get_content();
        self.auto_save.save(&content)?;
    }

    // Poll for events (non-blocking with timeout)
    if event::poll(std::time::Duration::from_millis(100))? {
        let ev = event::read()?;
        self.handle_event(&ev)?;
    }
}
```

- [ ] **Step 7: Add `pub mod services;` to lib.rs**

- [ ] **Step 8: Implement external file watcher**

Add `notify = "7"` to Cargo.toml.

```rust
// src/services/file_watcher.rs
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::path::PathBuf;
use std::sync::mpsc;

pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    pub rx: mpsc::Receiver<PathBuf>,
}

impl FileWatcher {
    pub fn new(path: &PathBuf) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    for path in event.paths {
                        let _ = tx.send(path);
                    }
                }
            }
        })?;
        watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
        Ok(Self { _watcher: watcher, rx })
    }
}
```

Add `pub mod file_watcher;` to `src/services/mod.rs`.

In the event loop, check `file_watcher.rx.try_recv()` — if the currently open file was modified externally, reload its content into the editor and show a brief notification in the status bar ("File reloaded — modified externally").

- [ ] **Step 9: Build and run**

```bash
cargo run
```

Expected: Open a .md file, edit it, wait 500ms — file is saved. `Ctrl+S` saves immediately. Edit the same file from another terminal — Deepwrite reloads the content.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "feat: file I/O with encoding detection, auto-save, and external file watching"
```

---

## Task 8: Markdown Syntax Coloring

**Files:**
- Create: `src/editor/markdown.rs`
- Modify: `Cargo.toml`, `src/editor/mod.rs`

- [ ] **Step 1: Add tree-sitter dependencies**

```toml
tree-sitter = "0.24"
tree-sitter-md = "0.3"
```

Note: Exact crate names and versions should be verified on crates.io. `tree-sitter-md` provides the Markdown grammar. If not available, use `tree-sitter-markdown` or build from the grammar source.

- [ ] **Step 2: Implement markdown.rs**

```rust
// src/editor/markdown.rs
use ratatui::style::{Color, Modifier, Style};
use tree_sitter::{Parser, Tree};
use crate::theme::Theme;

pub struct MarkdownHighlighter {
    parser: Parser,
    tree: Option<Tree>,
}

#[derive(Debug, Clone)]
pub struct HighlightRange {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
    pub style: Style,
}

impl MarkdownHighlighter {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_md::language())
            .expect("Failed to load Markdown grammar");
        Self { parser, tree: None }
    }

    pub fn parse(&mut self, source: &str, theme: &Theme) -> Vec<HighlightRange> {
        let tree = self.parser.parse(source, self.tree.as_ref());
        self.tree = tree.clone();

        let Some(tree) = tree else {
            return Vec::new();
        };

        let mut highlights = Vec::new();
        let root = tree.root_node();
        Self::walk_node(root, source, &mut highlights, theme);
        highlights
    }

    fn walk_node(
        node: tree_sitter::Node,
        source: &str,
        highlights: &mut Vec<HighlightRange>,
        theme: &Theme,
    ) {
        // Map tree-sitter node kinds to themed styles.
        // Note: exact node kind names depend on the grammar version.
        // Common kinds: atx_heading, emphasis, strong_emphasis, link,
        // code_span, fenced_code_block, block_quote, list_marker_minus, etc.
        let style = match node.kind() {
            // Headings: theme color + bold
            "atx_heading" | "heading_content" => {
                Some(Style::default().fg(theme.md_heading).add_modifier(Modifier::BOLD))
            }
            // Bold: inherit color, add bold
            "strong_emphasis" => Some(Style::default().add_modifier(Modifier::BOLD)),
            // Italic: inherit color, add italic
            "emphasis" => Some(Style::default().add_modifier(Modifier::ITALIC)),
            // Links and images: theme link color + underline
            "link" | "image" | "link_destination" | "uri_autolink" | "link_text" => {
                Some(Style::default().fg(theme.md_link).add_modifier(Modifier::UNDERLINED))
            }
            // Code: theme code color
            "code_span" | "fenced_code_block" | "code_fence_content" => {
                Some(Style::default().fg(theme.md_code))
            }
            // Block quotes: muted color
            "block_quote" | "block_quote_marker" => {
                Some(Style::default().fg(theme.md_muted))
            }
            // List markers: muted color
            "list_marker_minus" | "list_marker_plus" | "list_marker_star"
            | "list_marker_dot" | "list_marker_parenthesis" => {
                Some(Style::default().fg(theme.md_muted))
            }
            // Task list markers
            "task_list_marker_checked" | "task_list_marker_unchecked" => {
                Some(Style::default().fg(theme.md_muted))
            }
            // Strikethrough: muted + strikethrough modifier
            "strikethrough" => {
                Some(Style::default().fg(theme.md_muted).add_modifier(Modifier::CROSSED_OUT))
            }
            // Horizontal rule (thematic break)
            "thematic_break" => {
                Some(Style::default().fg(theme.md_muted))
            }
            _ => None,
        };

        if let Some(s) = style {
            let start = node.start_position();
            let end = node.end_position();
            highlights.push(HighlightRange {
                start_row: start.row,
                start_col: start.column,
                end_row: end.row,
                end_col: end.column,
                style: s,
            });
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::walk_node(child, source, highlights, theme);
        }
    }
}
```

Note: The tree-sitter node kinds will need to be verified against the actual tree-sitter-markdown grammar during implementation. Use `node.kind()` debugging to discover correct names.

- [ ] **Step 3: Integrate into EditorWrapper**

In `editor/mod.rs`, add a `MarkdownHighlighter` field. After each edit (detected by comparing content), reparse and rebuild `edtui::Highlight` ranges:

```rust
pub fn update_highlights(&mut self, theme: &Theme) {
    let content = self.get_content();
    let ranges = self.highlighter.parse(&content);

    self.state.clear_highlights();
    for range in ranges {
        // Map our HighlightRange to edtui Highlight
        // Apply theme colors based on the style modifiers
        let style = self.resolve_style(&range, theme);
        self.state.add_highlight(edtui::Highlight::new(
            edtui::Index2::new(range.start_row, range.start_col),
            edtui::Index2::new(range.end_row, range.end_col),
            style,
        ));
    }
}
```

- [ ] **Step 4: Build and test visually**

```bash
cargo run
```

Open a .md file. Expected: headings appear bold, code spans get code color, links get link color, etc.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: markdown syntax coloring via tree-sitter"
```

---

## Task 9: Status Bar

**Files:**
- Create: `src/ui/status_bar.rs`, `src/editor/word_count.rs`
- Create: `tests/word_count_test.rs`
- Modify: `src/ui/mod.rs`, `src/editor/mod.rs`

- [ ] **Step 1: Add dependency**

```toml
unicode-segmentation = "1"
```

- [ ] **Step 2: Write word count test**

```rust
// tests/word_count_test.rs
use deepwrite::editor::word_count::{count_words, count_chars};

#[test]
fn test_english_word_count() {
    assert_eq!(count_words("Hello world"), 2);
    assert_eq!(count_words("one two three four"), 4);
}

#[test]
fn test_chinese_word_count() {
    // Chinese: each character is a "word"
    assert_eq!(count_words("你好世界"), 4);
}

#[test]
fn test_mixed_word_count() {
    assert_eq!(count_words("Hello 你好 world"), 4); // Hello, 你, 好, world
}

#[test]
fn test_empty_string() {
    assert_eq!(count_words(""), 0);
    assert_eq!(count_chars(""), 0);
}

#[test]
fn test_char_count() {
    assert_eq!(count_chars("Hello"), 5);
    assert_eq!(count_chars("你好"), 2);
    assert_eq!(count_chars("Hello 你好"), 8); // including space
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test --test word_count_test
```

- [ ] **Step 4: Implement word_count.rs**

```rust
// src/editor/word_count.rs
use unicode_segmentation::UnicodeSegmentation;

pub fn count_words(text: &str) -> usize {
    text.unicode_words().count()
}

pub fn count_chars(text: &str) -> usize {
    text.graphemes(true).count()
}
```

Note: `unicode_words()` from unicode-segmentation splits on UAX#29 word boundaries which handles CJK correctly — each CJK character is treated as a separate word. Verify this behavior during implementation and add custom logic if needed.

- [ ] **Step 5: Implement status_bar.rs**

```rust
// src/ui/status_bar.rs
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use crate::theme::Theme;

pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    filename: &str,
    word_count: usize,
    char_count: usize,
    theme: &Theme,
) {
    let left = format!(" {}", filename);
    let right = format!("{} words  {} chars ", word_count, char_count);
    let padding = area.width as usize - left.len() - right.len();
    let content = format!("{}{:>width$}", left, right, width = padding + right.len());

    frame.render_widget(
        Paragraph::new(content)
            .style(Style::default().fg(theme.status_bar_fg).bg(theme.status_bar_bg)),
        area,
    );
}
```

- [ ] **Step 6: Add modules to mod.rs files and integrate into app.rs**

Wire `render_status_bar()` call in `App::draw()` with word/char counts from the editor content.

- [ ] **Step 7: Run tests**

```bash
cargo test --test word_count_test
```

Expected: All tests PASS.

- [ ] **Step 8: Build and run**

```bash
cargo run
```

Expected: Status bar shows filename, word count, and char count. Updates as you type.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat: status bar with word and character counting"
```

---

## Task 10: Focus Mode — State Machine and Paragraph Mode

**Files:**
- Create: `src/editor/focus.rs`
- Modify: `src/editor/mod.rs`, `src/app.rs`

- [ ] **Step 1: Implement focus mode state machine**

```rust
// src/editor/focus.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusMode {
    Off,
    Sentence,
    Paragraph,
    Typewriter,
}

impl FocusMode {
    pub fn cycle(self) -> Self {
        match self {
            FocusMode::Off => FocusMode::Sentence,
            FocusMode::Sentence => FocusMode::Paragraph,
            FocusMode::Paragraph => FocusMode::Typewriter,
            FocusMode::Typewriter => FocusMode::Off,
        }
    }

    pub fn has_dimming(self) -> bool {
        matches!(self, FocusMode::Sentence | FocusMode::Paragraph)
    }

    pub fn has_typewriter(self) -> bool {
        matches!(self, FocusMode::Typewriter)
    }

    pub fn label(self) -> &'static str {
        match self {
            FocusMode::Off => "Off",
            FocusMode::Sentence => "Focus: Sentence",
            FocusMode::Paragraph => "Focus: Paragraph",
            FocusMode::Typewriter => "Focus: Typewriter",
        }
    }
}
```

- [ ] **Step 2: Implement paragraph range detection**

Use tree-sitter to find the paragraph containing the cursor:

```rust
// In src/editor/focus.rs
use tree_sitter::Tree;

pub struct FocusRange {
    pub start_row: usize,
    pub end_row: usize,
}

pub fn find_paragraph_at_cursor(tree: &Tree, cursor_row: usize) -> Option<FocusRange> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "paragraph" || child.kind() == "section" {
            let start = child.start_position().row;
            let end = child.end_position().row;
            if cursor_row >= start && cursor_row <= end {
                return Some(FocusRange {
                    start_row: start,
                    end_row: end,
                });
            }
        }
    }
    None
}
```

- [ ] **Step 3: Integrate dimming into EditorWrapper**

In `update_highlights()`:
- If Focus Mode has dimming, first apply dimmed style to ALL text
- Then apply normal (bright) style to the active paragraph/sentence range
- Markdown syntax highlights are applied on top of the active range only

- [ ] **Step 4: Integrate Ctrl+D and panel auto-collapse into app.rs**

- `Ctrl+D` in Edit mode: cycle focus mode
- When focus mode is not Off: set `show_browser = false`
- When focus mode cycles back to Off: set `show_browser = true`
- Show focus mode label in status bar

- [ ] **Step 5: Implement typewriter mode scrolling**

In `EditorWrapper::render()`, if typewriter mode is active:
- Calculate the cursor's row position
- Set scroll offset so cursor row is vertically centered in the area

- [ ] **Step 6: Build and test visually**

```bash
cargo run
```

Expected: `Ctrl+D` cycles through modes. In Paragraph mode, only the current paragraph is bright. In Typewriter mode, cursor stays centered. Left panel collapses/restores.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: focus mode with paragraph dimming and typewriter scrolling"
```

---

## Task 11: Sentence Boundary Detection

**Files:**
- Create: `src/editor/sentence.rs`
- Create: `tests/sentence_test.rs`
- Modify: `src/editor/focus.rs`

- [ ] **Step 1: Write sentence boundary tests**

```rust
// tests/sentence_test.rs
use deepwrite::editor::sentence::find_sentence_at;

#[test]
fn test_english_sentences() {
    let text = "Hello world. This is a test. Another sentence here.";
    let range = find_sentence_at(text, 15); // cursor in "This is a test."
    assert_eq!(&text[range.start..range.end], "This is a test.");
}

#[test]
fn test_chinese_sentences() {
    let text = "你好世界。這是一個測試。另一個句子。";
    let range = find_sentence_at(text, "你好世界。".len() + 1); // cursor in second sentence
    assert_eq!(&text[range.start..range.end], "這是一個測試。");
}

#[test]
fn test_mixed_language() {
    let text = "Hello world。這是中文。More English.";
    let range = find_sentence_at(text, "Hello world。".len() + 1);
    assert_eq!(&text[range.start..range.end], "這是中文。");
}

#[test]
fn test_exclamation_and_question() {
    let text = "Really? Yes! OK.";
    let range = find_sentence_at(text, 0);
    assert_eq!(&text[range.start..range.end], "Really?");
    let range = find_sentence_at(text, 9);
    assert_eq!(&text[range.start..range.end], "Yes!");
}

#[test]
fn test_chinese_punctuation() {
    let text = "你好！真的嗎？是的。";
    let range = find_sentence_at(text, "你好！".len() + 1);
    assert_eq!(&text[range.start..range.end], "真的嗎？");
}

#[test]
fn test_single_sentence() {
    let text = "Just one sentence.";
    let range = find_sentence_at(text, 5);
    assert_eq!(&text[range.start..range.end], "Just one sentence.");
}

#[test]
fn test_cursor_at_end() {
    let text = "First. Second.";
    let range = find_sentence_at(text, text.len() - 1);
    assert_eq!(&text[range.start..range.end], "Second.");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test sentence_test
```

- [ ] **Step 3: Implement sentence.rs**

```rust
// src/editor/sentence.rs

#[derive(Debug, Clone, PartialEq)]
pub struct SentenceRange {
    pub start: usize,
    pub end: usize,
}

/// Find the sentence containing the given byte offset in the text.
pub fn find_sentence_at(text: &str, byte_offset: usize) -> SentenceRange {
    let boundaries = find_sentence_boundaries(text);

    for range in &boundaries {
        if byte_offset >= range.start && byte_offset < range.end {
            return range.clone();
        }
    }

    // If cursor is at or past the last boundary, return the last sentence
    boundaries
        .last()
        .cloned()
        .unwrap_or(SentenceRange { start: 0, end: text.len() })
}

fn find_sentence_boundaries(text: &str) -> Vec<SentenceRange> {
    let mut ranges = Vec::new();
    let mut start = 0;

    // Skip leading whitespace
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Find sentence end
        let byte_pos = text[..].char_indices()
            .nth(i)
            .map(|(pos, _)| pos)
            .unwrap_or(text.len());

        if is_sentence_terminator(chars[i]) {
            let end_byte = text.char_indices()
                .nth(i + 1)
                .map(|(pos, _)| pos)
                .unwrap_or(text.len());

            // Trim leading whitespace from sentence start
            let trimmed_start = text[start..].find(|c: char| !c.is_whitespace())
                .map(|offset| start + offset)
                .unwrap_or(start);

            if trimmed_start < end_byte {
                ranges.push(SentenceRange {
                    start: trimmed_start,
                    end: end_byte,
                });
            }
            start = end_byte;
        }
        i += 1;
    }

    // Handle trailing text without terminator
    if start < text.len() {
        let trimmed_start = text[start..].find(|c: char| !c.is_whitespace())
            .map(|offset| start + offset)
            .unwrap_or(start);
        if trimmed_start < text.len() {
            ranges.push(SentenceRange {
                start: trimmed_start,
                end: text.len(),
            });
        }
    }

    if ranges.is_empty() {
        ranges.push(SentenceRange { start: 0, end: text.len() });
    }

    ranges
}

fn is_sentence_terminator(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '。' | '！' | '？' | '；')
}
```

Note: This implementation uses byte offsets. The actual implementation may need to handle row/column indexing to integrate with edtui's `Index2` cursor positions. Adjust as needed during implementation.

- [ ] **Step 4: Run tests**

```bash
cargo test --test sentence_test
```

Expected: All tests PASS. Some tests may need adjustment based on exact whitespace handling.

- [ ] **Step 5: Integrate into focus.rs**

Add `find_sentence_at_cursor()` function that converts edtui's cursor position (row, col) to a byte offset, calls `find_sentence_at()`, then converts the result back to row ranges for dimming.

- [ ] **Step 6: Build and test visually**

```bash
cargo run
```

Expected: In Focus Sentence mode, only the current sentence is bright. Moving cursor between sentences updates highlighting.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: sentence boundary detection for Chinese and English"
```

---

## Task 12: Formatting Shortcuts

**Files:**
- Create: `src/editor/formatting.rs`
- Create: `tests/formatting_test.rs`
- Modify: `src/editor/mod.rs`, `src/app.rs`

- [ ] **Step 1: Write formatting tests**

```rust
// tests/formatting_test.rs
use deepwrite::editor::formatting::*;

#[test]
fn test_bold_wrap() {
    let result = wrap_selection("hello", "**");
    assert_eq!(result, "**hello**");
}

#[test]
fn test_bold_unwrap() {
    let result = unwrap_if_wrapped("**hello**", "**");
    assert_eq!(result, Some("hello".to_string()));
}

#[test]
fn test_bold_unwrap_not_wrapped() {
    let result = unwrap_if_wrapped("hello", "**");
    assert_eq!(result, None);
}

#[test]
fn test_italic_wrap() {
    let result = wrap_selection("hello", "*");
    assert_eq!(result, "*hello*");
}

#[test]
fn test_link_no_selection() {
    let result = link_template("");
    assert_eq!(result, "[](url)");
}

#[test]
fn test_link_with_selection() {
    let result = link_template("click here");
    assert_eq!(result, "[click here](url)");
}

#[test]
fn test_heading_toggle_add() {
    let result = toggle_heading("Some text", 2);
    assert_eq!(result, "## Some text");
}

#[test]
fn test_heading_toggle_remove_same() {
    let result = toggle_heading("## Some text", 2);
    assert_eq!(result, "Some text");
}

#[test]
fn test_heading_toggle_change_level() {
    let result = toggle_heading("## Some text", 3);
    assert_eq!(result, "### Some text");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --test formatting_test
```

- [ ] **Step 3: Implement formatting.rs**

```rust
// src/editor/formatting.rs

pub fn wrap_selection(text: &str, marker: &str) -> String {
    format!("{}{}{}", marker, text, marker)
}

pub fn unwrap_if_wrapped(text: &str, marker: &str) -> Option<String> {
    if text.starts_with(marker) && text.ends_with(marker) && text.len() >= marker.len() * 2 {
        Some(text[marker.len()..text.len() - marker.len()].to_string())
    } else {
        None
    }
}

pub fn link_template(selected_text: &str) -> String {
    if selected_text.is_empty() {
        "[](url)".to_string()
    } else {
        format!("[{}](url)", selected_text)
    }
}

pub fn toggle_heading(line: &str, level: usize) -> String {
    let target_prefix = format!("{} ", "#".repeat(level));

    // Check if line already has a heading
    if let Some(stripped) = strip_heading(line) {
        let current_level = line.chars().take_while(|c| *c == '#').count();
        if current_level == level {
            // Same level: remove
            stripped.to_string()
        } else {
            // Different level: replace
            format!("{}{}", target_prefix, stripped)
        }
    } else {
        // No heading: add
        format!("{}{}", target_prefix, line)
    }
}

fn strip_heading(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        let after_hashes = trimmed.trim_start_matches('#');
        if after_hashes.starts_with(' ') {
            Some(after_hashes.trim_start_matches(' '))
        } else {
            Some(after_hashes)
        }
    } else {
        None
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test formatting_test
```

Expected: All tests PASS.

- [ ] **Step 5: Integrate into editor**

In `app.rs`, handle `Ctrl+B`, `Ctrl+I`, `Ctrl+K`, `Ctrl+U`, `Ctrl+1-6` in Edit mode:
- Get selected text from `EditorState` (if any)
- Apply formatting function
- Replace selection (or insert at cursor) via edtui's state API

This step requires understanding edtui's selection API (`state.selection()`, `state.insert()`, etc.). Consult edtui source for exact methods.

- [ ] **Step 6: Build and test visually**

```bash
cargo run
```

Expected: Select text, `Ctrl+B` wraps in `**`. `Ctrl+B` again on already-bold text unwraps. `Ctrl+1` adds `# ` to line start.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: markdown formatting shortcuts (bold, italic, link, heading)"
```

---

## Task 13: File Browser Actions

**Files:**
- Create: `src/browser/actions.rs` (replace placeholder)
- Modify: `src/app.rs`, `src/browser/navigator.rs`

- [ ] **Step 1: Implement browser actions**

```rust
// src/browser/actions.rs
use std::path::Path;
use anyhow::Result;

pub fn create_file(dir: &Path, name: &str) -> Result<()> {
    let name_with_ext = if name.ends_with(".md") || name.ends_with(".txt") {
        name.to_string()
    } else {
        format!("{}.md", name)
    };
    std::fs::write(dir.join(&name_with_ext), "")?;
    Ok(())
}

pub fn create_directory(dir: &Path, name: &str) -> Result<()> {
    std::fs::create_dir(dir.join(name))?;
    Ok(())
}

pub fn rename_entry(dir: &Path, old_name: &str, new_name: &str) -> Result<()> {
    std::fs::rename(dir.join(old_name), dir.join(new_name))?;
    Ok(())
}

pub fn delete_entry(dir: &Path, name: &str) -> Result<()> {
    let path = dir.join(name);
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
```

- [ ] **Step 2: Add text input overlay for prompts**

Create a simple inline text input for "New file name:", "Rename to:", "Delete? (y/n)" prompts. Use edtui in single-line mode for the input field, rendered as an overlay at the bottom of the browser panel.

- [ ] **Step 3: Implement fuzzy search**

In `browser/navigator.rs`, add fuzzy filtering:

```rust
pub fn filter_entries(&self, query: &str) -> Vec<&Entry> {
    if query.is_empty() {
        return self.entries.iter().collect();
    }
    self.entries
        .iter()
        .filter(|e| fuzzy_match(&e.name, query))
        .collect()
}

fn fuzzy_match(name: &str, query: &str) -> bool {
    let name_lower = name.to_lowercase();
    let query_lower = query.to_lowercase();
    let mut query_chars = query_lower.chars();
    let mut current = query_chars.next();
    for c in name_lower.chars() {
        if let Some(q) = current {
            if c == q {
                current = query_chars.next();
            }
        } else {
            return true;
        }
    }
    current.is_none()
}
```

- [ ] **Step 4: Wire up keys in app.rs**

In `handle_browse_key()`:
- `n` → show "New file:" prompt → `create_file()` → `navigator.refresh()`
- `N` → show "New directory:" prompt → `create_directory()` → `navigator.refresh()`
- `r` → show "Rename to:" prompt → `rename_entry()` → `navigator.refresh()`
- `d` → show "Delete? (y/n)" prompt → `delete_entry()` → `navigator.refresh()`
- `/` → enter fuzzy search mode, filter entries as user types, `Enter` selects, `Esc` cancels

- [ ] **Step 5: Build and test**

```bash
cargo run
```

Expected: `n` creates a new file. `r` renames. `d` deletes (with confirmation). `/` filters the file list.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: file browser actions (create, rename, delete, fuzzy search)"
```

---

## Task 14: CLI Argument Parsing

**Files:**
- Modify: `Cargo.toml`, `src/main.rs`

- [ ] **Step 1: Add clap dependency**

```toml
clap = { version = "4", features = ["derive"] }
```

- [ ] **Step 2: Implement CLI parsing**

```rust
// In src/main.rs
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "deepwrite", version, about = "A terminal Markdown writing tool with Focus Mode")]
struct Cli {
    /// Directory or file to open
    path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let start_path = cli.path
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Determine if opening a file or directory
    let (start_dir, start_file) = if start_path.is_file() {
        (start_path.parent().unwrap_or(Path::new(".")).to_path_buf(), Some(start_path))
    } else {
        (start_path, None)
    };

    // ... terminal setup ...

    let config = Config::load();
    let mut app = app::App::new(config, start_dir);

    if let Some(file) = start_file {
        app.open_file(&file)?;
    }

    app.run(&mut terminal)
}
```

- [ ] **Step 3: Update App::new() to accept start_dir**

- [ ] **Step 4: Test CLI**

```bash
cargo run -- .
cargo run -- ~/some/path
cargo run -- ~/some/file.md
cargo run -- --version
cargo run -- --help
```

Expected: Each behaves correctly per the spec.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: CLI argument parsing with clap"
```

---

## Task 15: Integration Polish

**Files:**
- Modify: Various files for edge cases

- [ ] **Step 1: Handle terminal resize**

In the event loop, handle `Event::Resize`:
```rust
Event::Resize(_, _) => { /* ratatui redraws automatically */ }
```

- [ ] **Step 2: Add Esc double-press behavior**

Implement the spec behavior: first `Esc` exits Focus Mode (if active), second `Esc` returns to Browse mode:

```rust
fn handle_edit_key(&mut self, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if self.focus_mode != FocusMode::Off {
                self.focus_mode = FocusMode::Off;
                self.show_browser = true;
            } else {
                self.mode = AppMode::Browse;
            }
        }
        // ...
    }
}
```

- [ ] **Step 3: Show focus mode in status bar**

Update status bar to show focus mode label when active:
```
│ 📝 api-design.md  [Focus: Sentence]   42 words  231 chars │
```

- [ ] **Step 4: Handle empty directory gracefully**

If the directory has no .md/.txt files, show a message: "No Markdown files. Press n to create one."

- [ ] **Step 5: Handle unsaved untitled documents**

On quit from Edit mode with unsaved content:
- If file has a path: auto-save before exit
- If untitled: save to `~/.local/share/deepwrite/unsaved/{uuid}.md`

- [ ] **Step 6: Add graceful panic handling**

Ensure terminal is restored on panic:
```rust
fn main() -> Result<()> {
    // Set panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        original_hook(panic);
    }));
    // ...
}
```

- [ ] **Step 7: Full manual test**

Test the complete workflow:
1. `cargo run` — opens current dir in browser
2. Navigate files with arrow keys
3. Open a .md file with Enter
4. Type text — see it appear with Markdown coloring
5. `Ctrl+B` to bold selected text
6. `Ctrl+D` to cycle Focus Mode — verify dimming and panel collapse
7. `Esc` to exit Focus Mode, `Esc` again to go back to browser
8. `n` to create new file, `r` to rename, `d` to delete
9. `Ctrl+S` to force save
10. `q` to quit

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: polish — resize handling, graceful exit, edge cases"
```

---

## Task 16: Final Build and Distribution Prep

- [ ] **Step 1: Verify release build**

```bash
cargo build --release
ls -lh target/release/deepwrite
```

Expected: Binary < 10MB.

- [ ] **Step 2: Test release binary**

```bash
./target/release/deepwrite
./target/release/deepwrite --version
./target/release/deepwrite ~/some/path
```

- [ ] **Step 3: Add README with install instructions**

Only if requested by the user.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "chore: release build verification"
```
