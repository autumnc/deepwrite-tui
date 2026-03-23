# Yazi-Inspired UX Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align deepwrite-tui's file browser UX with Yazi's conventions: unified create (`a`), help screen (`?`), scrolloff, mouse support, and yank path (`y`).

**Architecture:** Six independent features, each self-contained. Tasks 1-4 modify `src/app.rs` and browser modules. Task 5 (mouse) touches `src/main.rs` and `src/app.rs` and stores layout rect for accurate click detection. Task 6 (yank) adds `arboard` as a direct dependency. All features are additive — keybinding changes: replace `n`/`N` with `a`, add `h`/`l` navigation.

**Tech Stack:** Rust, ratatui 0.30, crossterm 0.29 (mouse events), arboard 3 (clipboard)

---

### Task 1: Unified Create with `a` key

Replace `n` (new file) and `N` (new directory) with a single `a` key. If the input ends with `/`, create a directory; otherwise create a file.

**Files:**
- Modify: `src/app.rs` — `BrowserPrompt` enum, `handle_browse_key`, `draw`, `handle_prompt_key`, `confirm_prompt`
- Modify: `src/browser/widget.rs:105` — update empty-directory hint text

- [ ] **Step 1: Write test for `a` creating a file**

In `src/app.rs` tests section, add:

```rust
#[test]
fn create_prompt_without_slash_creates_file() {
    let tmp = TempDir::new().unwrap();
    let mut app = test_app(&tmp);

    // Simulate pressing 'a', typing "notes", Enter
    let a_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    app.handle_browse_key(a_key);
    assert_eq!(app.prompt, BrowserPrompt::Create(String::new()));

    // Type "notes"
    for c in "notes".chars() {
        let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        app.handle_prompt_key(key);
    }

    // Confirm
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    app.handle_prompt_key(enter);

    assert!(tmp.path().join("notes.md").exists());
    assert_eq!(app.prompt, BrowserPrompt::None);
}
```

- [ ] **Step 2: Write test for `a` creating a directory (trailing `/`)**

```rust
#[test]
fn create_prompt_with_trailing_slash_creates_directory() {
    let tmp = TempDir::new().unwrap();
    let mut app = test_app(&tmp);

    let a_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    app.handle_browse_key(a_key);

    for c in "drafts/".chars() {
        let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        app.handle_prompt_key(key);
    }

    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    app.handle_prompt_key(enter);

    assert!(tmp.path().join("drafts").is_dir());
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test create_prompt -- --nocapture`
Expected: compilation error — `BrowserPrompt::Create` does not exist yet.

- [ ] **Step 4: Implement unified create**

In `src/app.rs`:

1. Replace `BrowserPrompt::NewFile(String)` and `BrowserPrompt::NewDirectory(String)` with `BrowserPrompt::Create(String)`.

2. In `handle_browse_key`, replace the `n` and `N` handlers:
```rust
// Before:
KeyCode::Char('n') => {
    self.prompt = BrowserPrompt::NewFile(String::new());
}
KeyCode::Char('N') => {
    self.prompt = BrowserPrompt::NewDirectory(String::new());
}

// After:
KeyCode::Char('a') => {
    self.prompt = BrowserPrompt::Create(String::new());
}
```

3. In `draw`, replace the two `NewFile`/`NewDirectory` prompt info arms:
```rust
// Before:
BrowserPrompt::NewFile(buf) => Some(BrowserPromptInfo {
    label: "New file: ",
    input: buf,
}),
BrowserPrompt::NewDirectory(buf) => Some(BrowserPromptInfo {
    label: "New dir: ",
    input: buf,
}),

// After:
BrowserPrompt::Create(buf) => Some(BrowserPromptInfo {
    label: "Create: ",
    input: buf,
}),
```

4. In `handle_prompt_key`, update the Backspace and Char arms — replace `BrowserPrompt::NewFile(buf) | BrowserPrompt::NewDirectory(buf)` with `BrowserPrompt::Create(buf)`.

5. In `confirm_prompt`, replace the two `NewFile`/`NewDirectory` arms:
```rust
// Before:
BrowserPrompt::NewFile(name) => { ... }
BrowserPrompt::NewDirectory(name) => { ... }

// After:
BrowserPrompt::Create(name) => {
    let trimmed = name.trim();
    if !trimmed.is_empty() {
        if trimmed.ends_with('/') {
            let dir_name = trimmed.trim_end_matches('/');
            let _ = actions::create_directory(&self.navigator.current_dir, dir_name);
        } else {
            let _ = actions::create_file(&self.navigator.current_dir, trimmed);
        }
        self.navigator.refresh();
    }
}
```

6. In `src/browser/widget.rs:105`, update the empty-directory message:
```rust
// Before:
let msg = Paragraph::new("No Markdown files.\nPress n to create one.")

// After:
let msg = Paragraph::new("No Markdown files.\nPress a to create one.")
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test create_prompt -- --nocapture`
Expected: both tests PASS.

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: all tests pass. Fix any compilation errors from the enum variant rename.

- [ ] **Step 7: Commit**

```bash
git add src/app.rs src/browser/widget.rs
git commit -m "feat: unify file/directory creation with 'a' key (Yazi-style)

Replace n/N keybindings with a single 'a' key. Trailing '/' in the
input name creates a directory; otherwise creates a file."
```

---

### Task 2: Add `h`/`l` navigation keybindings (Yazi vim convention)

Add `h` for go-to-parent and `l` for enter-directory/open-file, completing the vim `hjkl` set (currently only `j`/`k` are mapped).

**Files:**
- Modify: `src/app.rs` — add `h` and `l` to `handle_browse_key` match

- [ ] **Step 1: Add `h` and `l` keybindings**

In `src/app.rs`, in `handle_browse_key`, update the existing navigation arms:

```rust
// Before:
KeyCode::Up | KeyCode::Char('k') => {

// After (no change needed — k/j already work):
KeyCode::Up | KeyCode::Char('k') => {
```

Add `l` to the Right/Enter arm:
```rust
// Before:
KeyCode::Right | KeyCode::Enter => {

// After:
KeyCode::Right | KeyCode::Enter | KeyCode::Char('l') => {
```

Add `h` to the Left/Backspace arm:
```rust
// Before:
KeyCode::Left | KeyCode::Backspace => {

// After:
KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => {
```

- [ ] **Step 2: Run tests**

Run: `cargo test`
Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "feat: add h/l navigation keybindings (Yazi vim convention)

h = go to parent directory, l = enter directory or open file.
Completes the hjkl navigation set alongside existing j/k."
```

---

### Task 3: Help screen (`?` in Browse mode)

Show a full-screen overlay listing all keybindings when user presses `?` in Browse mode. Close with `?`, `Esc`, or `q`.

**Files:**
- Modify: `src/app.rs` — add `show_help: bool` field, handle `?` key, render help overlay
- Create: `src/ui/help.rs` — help overlay rendering function
- Modify: `src/ui/mod.rs` — add `pub mod help;`

- [ ] **Step 1: Create `src/ui/help.rs` with the help overlay renderer**

```rust
//! Help screen overlay showing all keybindings.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::theme::Theme;

const HELP_TEXT: &str = "\
 Browse Mode                     Edit Mode
 ──────────────────────────────  ──────────────────────────────
 k / ↑       Move up             Esc          Exit to Browse
 j / ↓       Move down           Ctrl+S       Save
 h / ←       Go to parent dir    Ctrl+E       Toggle browser
 l / → / ⏎   Enter dir / Open    Ctrl+D       Cycle focus mode
 a           Create file/dir     Ctrl+B       Bold
 r           Rename              Ctrl+I       Italic
 d           Delete              Ctrl+U       Strikethrough
 y           Copy path           Ctrl+K       Insert link
 /           Search/filter       Ctrl+1..6    Heading level
 .           Toggle hidden       Ctrl+Z       Undo
 Ctrl+E      Toggle browser      Ctrl+Y       Redo
 q           Quit                Ctrl+A       Select all
                                 Ctrl+C       Copy
                                 Ctrl+X       Cut
                                 Ctrl+V       Paste

 Press ? or Esc to close";

/// Render the help overlay centered on screen.
pub fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    // Size the overlay: fixed width, height based on content
    let width = 66.min(area.width.saturating_sub(4));
    let height = 22.min(area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let overlay = Rect::new(x, y, width, height);

    // Clear the area behind the overlay
    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.bg).fg(theme.fg));

    let paragraph = Paragraph::new(HELP_TEXT)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, overlay);
}
```

- [ ] **Step 2: Register the module**

In `src/ui/mod.rs`, add:
```rust
pub mod help;
```

- [ ] **Step 3: Add `show_help` field to `App` and wire up rendering**

In `src/app.rs`:

1. Add field to `App` struct:
```rust
pub show_help: bool,
```

2. Initialize in `App::new`:
```rust
show_help: false,
```

3. Add import at top of `src/app.rs`:
```rust
use deepwrite::ui::help::render_help;
```

4. In `draw()`, after rendering browser and editor and before status bar, add:
```rust
if self.show_help {
    render_help(frame, area, &self.theme);
}
```

- [ ] **Step 4: Handle `?` key in Browse mode**

In `handle_browse_key`, before the existing match:
```rust
// Help screen toggle
if self.show_help {
    match key.code {
        KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
            self.show_help = false;
        }
        _ => {}
    }
    return;
}
```

In the existing match block, add a new arm:
```rust
KeyCode::Char('?') => {
    self.show_help = true;
}
```

- [ ] **Step 5: Run tests and clippy**

Run: `cargo test && cargo clippy`
Expected: all pass, no warnings.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/ui/help.rs src/ui/mod.rs
git commit -m "feat: add help screen overlay (? key in Browse mode)

Shows all keybindings in a centered overlay. Close with ?, Esc, or q."
```

---

### Task 4: Scrolloff (5-line buffer)

Keep 5 lines of context above/below the cursor when scrolling the browser list. This is handled in the widget renderer by adjusting the `ListState` offset.

**Files:**
- Modify: `src/browser/widget.rs` — add scrolloff logic before `render_stateful_widget`

- [ ] **Step 1: Implement scrolloff in browser widget**

In `src/browser/widget.rs`, in the `render_browser_with_prompt` function, after computing `selected_list_index` and before the `if items.is_empty()` check, add scrolloff calculation:

```rust
// Scrolloff: keep 5 lines of context above/below the cursor.
let scrolloff: usize = 5;
let visible_height = list_area.height.saturating_sub(2) as usize; // subtract border lines
```

Then, after `let mut state = ListState::default(); state.select(selected_list_index);`, replace the `frame.render_stateful_widget(list, list_area, &mut state);` section:

```rust
let mut state = ListState::default();
state.select(selected_list_index);

// Apply scrolloff: adjust the list offset so the selected item
// stays at least `scrolloff` lines from the top/bottom edge.
if let Some(sel) = selected_list_index {
    let visible_height = list_area.height.saturating_sub(2) as usize;
    if visible_height > 0 {
        let scrolloff = scrolloff.min(visible_height / 2);
        // Desired: sel should be >= offset + scrolloff
        //          sel should be <= offset + visible_height - 1 - scrolloff
        let current_offset = *state.offset_mut();
        let mut offset = current_offset;

        if sel < offset + scrolloff {
            offset = sel.saturating_sub(scrolloff);
        } else if sel + scrolloff >= offset + visible_height {
            offset = (sel + scrolloff + 1).saturating_sub(visible_height);
        }
        *state.offset_mut() = offset;
    }
}

frame.render_stateful_widget(list, list_area, &mut state);
```

Remove the standalone `let scrolloff` and `let visible_height` lines added before the `if items.is_empty()` check — they've been moved into the stateful widget block.

- [ ] **Step 2: Run tests and verify visually**

Run: `cargo test && cargo clippy`
Expected: all pass.

Run: `cargo run -- .` and navigate a directory with many files to verify scrolloff behavior visually.

- [ ] **Step 3: Commit**

```bash
git add src/browser/widget.rs
git commit -m "feat: add scrolloff (5-line buffer) to browser list

Cursor stays at least 5 lines from the top/bottom edge of the
visible list, keeping surrounding context visible during navigation."
```

---

### Task 5: Mouse support (click + scroll)

Enable mouse capture in the terminal. Handle click-to-select and scroll in Browse mode browser panel. Store the browser layout rect on `App` so the mouse handler can accurately determine click targets.

**Files:**
- Modify: `src/main.rs` — enable/disable mouse capture
- Modify: `src/app.rs` — store `browser_rect`, handle `Event::Mouse` events

- [ ] **Step 1: Enable mouse capture in `src/main.rs`**

Update imports and terminal setup:
```rust
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
```

Replace the existing `execute!(stdout, EnterAlternateScreen)?;` line:
```rust
execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
```

Update `restore_terminal()`:
```rust
fn restore_terminal() -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
```

- [ ] **Step 2: Store browser layout rect on `App`**

In `src/app.rs`:

1. Add field to `App` struct:
```rust
browser_rect: Rect,
```

2. Initialize in `App::new`:
```rust
browser_rect: Rect::default(),
```

3. In `draw()`, after computing `layout`, store the browser rect:
```rust
let layout = compute_layout(area, self.config.browser.panel_width, self.show_browser);
self.browser_rect = layout.browser;
```

- [ ] **Step 3: Handle mouse events in `src/app.rs`**

Add mouse event imports at the top:
```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind, MouseButton};
```

In `handle_event`, extend the match to handle `Event::Mouse`:
```rust
Event::Mouse(mouse) => {
    self.handle_mouse_event(mouse);
}
```

Add the mouse handler method to `App`:
```rust
fn handle_mouse_event(&mut self, mouse: &crossterm::event::MouseEvent) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if self.mode == AppMode::Browse && self.show_browser {
                let br = self.browser_rect;
                // Check if click is within the browser panel area.
                if mouse.column >= br.x
                    && mouse.column < br.x + br.width
                    && mouse.row >= br.y
                    && mouse.row < br.y + br.height
                {
                    // The list content starts at br.y (no top border).
                    // The block has Borders::RIGHT only, so no top/bottom offset.
                    let clicked_row = (mouse.row - br.y) as usize;
                    let total = if let Some(ref matches) = self.search_matches {
                        matches.len()
                    } else {
                        self.navigator.entries.len()
                    };
                    if clicked_row < total {
                        if let Some(ref matches) = self.search_matches {
                            if clicked_row < matches.len() {
                                self.navigator.selected = matches[clicked_row];
                            }
                        } else {
                            self.navigator.selected = clicked_row;
                        }
                        self.preview_selected_file();
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if self.mode == AppMode::Browse {
                self.navigator.move_up();
                self.preview_selected_file();
            }
        }
        MouseEventKind::ScrollDown => {
            if self.mode == AppMode::Browse {
                self.navigator.move_down();
                self.preview_selected_file();
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 4: Run tests and clippy**

Run: `cargo test && cargo clippy`
Expected: all pass.

- [ ] **Step 5: Test manually**

Run: `cargo run -- .` and verify:
- Clicking on a file in the browser selects it
- Scroll wheel moves the selection up/down
- Mouse works in Browse mode only

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/app.rs
git commit -m "feat: add mouse support (click to select, scroll to navigate)

Enable crossterm mouse capture. Store browser layout rect for accurate
click detection. Scroll wheel moves selection up/down in Browse mode."
```

---

### Task 6: Copy path to clipboard (`y` in Browse mode)

Press `y` to copy the selected file's full path to the system clipboard, with a status bar confirmation.

**Files:**
- Modify: `Cargo.toml` — add `arboard = "3"` as a direct dependency
- Modify: `src/app.rs` — handle `y` key, add clipboard helper

- [ ] **Step 1: Add `arboard` dependency**

In `Cargo.toml`, add to `[dependencies]`:
```toml
arboard = "3"
```

Run: `cargo check` to verify dependency resolves.

- [ ] **Step 2: Implement `y` key handler in Browse mode**

In `src/app.rs`, add to `handle_browse_key` match block:
```rust
KeyCode::Char('y') => {
    if let Some(entry) = self.navigator.selected_entry() {
        let full_path = self.navigator.current_dir.join(&entry.name);
        let path_str = full_path.display().to_string();
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&path_str)) {
            Ok(()) => self.set_status_message(format!("Copied: {path_str}")),
            Err(err) => self.set_status_message(format!("Copy failed: {err}")),
        }
    }
}
```

Add import at top of `src/app.rs` (not needed — use full path `arboard::Clipboard`).

- [ ] **Step 3: Write test**

```rust
#[test]
fn yank_path_does_not_panic_without_selection() {
    let tmp = TempDir::new().unwrap();
    // Create an empty subdirectory (no .md files) so entries is empty
    let empty = tmp.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    let mut app = App::new(Config::default(), empty);
    app.data_dir = tmp.path().join("app-data");

    // Pressing 'y' with no entries should not panic
    let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
    app.handle_browse_key(y_key);
    // No assertion needed — just verifying no panic
}
```

- [ ] **Step 4: Run tests and clippy**

Run: `cargo test && cargo clippy`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/app.rs
git commit -m "feat: copy file path to clipboard with 'y' key (Browse mode)

Press y to copy the selected file's absolute path to the system
clipboard. Shows confirmation in the status bar."
```
