use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::{prelude::*, widgets::Block};

use deepwrite::browser::actions;
use deepwrite::browser::entries::EntryKind;
use deepwrite::browser::navigator::Navigator;
use deepwrite::browser::widget::{
    browser_content_area, browser_scroll_offset, render_browser_with_prompt, split_browser_area,
    BrowserPromptInfo,
};
use deepwrite::config::Config;
use deepwrite::editor::centered_editor_area;
use deepwrite::editor::focus::FocusMode;
use deepwrite::editor::formatting;
use deepwrite::editor::word_count;
use deepwrite::editor::EditorWrapper;
use deepwrite::services::auto_save::AutoSave;
use deepwrite::services::file_io;
use deepwrite::services::file_watcher::FileWatcher;
use deepwrite::services::update_checker;
use deepwrite::theme::Theme;
use deepwrite::ui::help::render_help;

/// Map Zhuyin (Bopomofo) characters back to their ASCII key equivalents.
/// When a CJK input method is active on macOS, pressing Ctrl+F sends
/// Ctrl+ㄑ instead of Ctrl+F. This table reverses the standard Zhuyin
/// keyboard layout so Ctrl shortcuts work regardless of input method.
fn normalize_zhuyin(c: char) -> char {
    // Standard Zhuyin (大千/Dachen) keyboard layout mapping
    match c {
        // Row 1 (number row): 1 2 _ _ 5 _ _ 8 9 0 - =
        'ㄅ' => '1',
        'ㄉ' => '2',
        'ㄓ' => '5',
        'ㄚ' => '8',
        'ㄞ' => '9',
        'ㄢ' => '0',
        'ㄦ' => '-',
        // Row 2 (qwerty): q w e r t y u i o p
        'ㄆ' => 'q',
        'ㄊ' => 'w',
        'ㄍ' => 'e',
        'ㄐ' => 'r',
        'ㄔ' => 't',
        'ㄗ' => 'y',
        'ㄧ' => 'u',
        'ㄛ' => 'i',
        'ㄟ' => 'o',
        'ㄣ' => 'p',
        // Row 3 (asdf): a s d f g h j k l ;
        'ㄇ' => 'a',
        'ㄋ' => 's',
        'ㄎ' => 'd',
        'ㄑ' => 'f',
        'ㄕ' => 'g',
        'ㄘ' => 'h',
        'ㄨ' => 'j',
        'ㄜ' => 'k',
        'ㄠ' => 'l',
        'ㄤ' => ';',
        // Row 4 (zxcv): z x c v b n m , . /
        'ㄈ' => 'z',
        'ㄌ' => 'x',
        'ㄏ' => 'c',
        'ㄒ' => 'v',
        'ㄖ' => 'b',
        'ㄙ' => 'n',
        'ㄩ' => 'm',
        'ㄝ' => ',',
        'ㄡ' => '.',
        'ㄥ' => '/',
        // Full-width punctuation
        '，' => ',',
        '。' => '.',
        '／' => '/',
        '；' => ';',
        other => other,
    }
}

/// Normalize a KeyEvent: map Zhuyin characters back to ASCII.
/// In Edit mode, only applies when Ctrl is held (so normal typing isn't affected).
fn normalize_key(key: KeyEvent) -> KeyEvent {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            let normalized = normalize_zhuyin(c);
            if normalized != c {
                return KeyEvent::new(KeyCode::Char(normalized), key.modifiers);
            }
        }
    }
    key
}

/// Normalize a KeyEvent for Browse mode: always map Zhuyin to ASCII,
/// since Browse mode uses single-key shortcuts (j, k, ., etc.) that
/// don't involve typing text.
fn normalize_browse_key(key: KeyEvent) -> KeyEvent {
    if let KeyCode::Char(c) = key.code {
        let normalized = normalize_zhuyin(c);
        if normalized != c {
            return KeyEvent::new(KeyCode::Char(normalized), key.modifiers);
        }
    }
    key
}
use deepwrite::ui::layout::compute_layout;
use deepwrite::ui::status_bar::render_status_bar;

/// The current interaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Browse,
    Edit,
}

/// A text input buffer with cursor position for prompts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptInput {
    pub text: String,
    /// Cursor position in characters (not bytes).
    pub cursor: usize,
}

impl PromptInput {
    fn new(text: String) -> Self {
        let cursor = text.chars().count();
        Self { text, cursor }
    }

    fn empty() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    fn insert(&mut self, c: char) {
        let byte_idx = self
            .text
            .char_indices()
            .nth(self.cursor)
            .map_or(self.text.len(), |(i, _)| i);
        self.text.insert(byte_idx, c);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let byte_idx = self
                .text
                .char_indices()
                .nth(self.cursor)
                .map_or(self.text.len(), |(i, _)| i);
            self.text.remove(byte_idx);
        }
    }

    fn delete(&mut self) {
        let len = self.text.chars().count();
        if self.cursor < len {
            let byte_idx = self
                .text
                .char_indices()
                .nth(self.cursor)
                .map_or(self.text.len(), |(i, _)| i);
            self.text.remove(byte_idx);
        }
    }

    fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_right(&mut self) {
        let len = self.text.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }
}

/// Active prompt overlay in the browser panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserPrompt {
    None,
    Create(PromptInput),
    Rename(PromptInput),
    Delete,
    Search(PromptInput),
}

#[derive(Debug, Clone)]
struct StatusMessage {
    text: String,
    expires_at: Instant,
}

impl StatusMessage {
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            expires_at: Instant::now() + Duration::from_secs(5),
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Top-level application state.
pub struct App {
    pub mode: AppMode,
    pub config: Config,
    pub theme: Theme,
    pub show_browser: bool,
    pub show_help: bool,
    pub should_quit: bool,
    pub navigator: Navigator,
    pub editor: EditorWrapper,
    pub auto_save: AutoSave,
    pub file_watcher: Option<FileWatcher>,
    pub current_filename: String,
    pub focus_mode: FocusMode,
    pub prompt: BrowserPrompt,
    /// Filtered entry indices during search; None when not searching.
    pub search_matches: Option<Vec<usize>>,
    pub editor_line_width: u16,
    editor_render_width: u16,
    pub pending_external_change: bool,
    browser_visibility_before_focus: bool,
    status_message: Option<StatusMessage>,
    last_conflict_backup_content: Option<String>,
    data_dir: PathBuf,
    browser_content_rect: Rect,
    browser_scroll_offset: usize,
    update_check_rx: Option<mpsc::Receiver<update_checker::UpdateCheckResult>>,
    pub update_available: Option<String>,
    /// True when `c` was pressed and we're waiting for the second key.
    pending_c_prefix: bool,
}

impl App {
    /// Create a new App from the given configuration, rooted in `start_dir`.
    pub fn new(config: Config, start_dir: std::path::PathBuf) -> Self {
        let update_check_rx = if config.updates.check_on_startup {
            update_checker::check_for_updates()
        } else {
            None
        };
        Self::new_with_update_receiver(config, start_dir, update_check_rx)
    }

    fn new_with_update_receiver(
        config: Config,
        start_dir: std::path::PathBuf,
        update_check_rx: Option<mpsc::Receiver<update_checker::UpdateCheckResult>>,
    ) -> Self {
        let theme = Theme::from_config(&config.theme.mode, config.focus.opacity);
        let show_hidden = config.browser.show_hidden;
        let editor_line_width = config.editor.line_width;
        let navigator = Navigator::new(&start_dir, show_hidden);
        let editor = EditorWrapper::new();
        let auto_save = AutoSave::new(config.editor.auto_save_delay_ms);
        let focus_mode = FocusMode::from_config(&config.focus.mode);
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("deepwrite");

        let mut app = Self {
            mode: AppMode::Browse,
            config,
            theme,
            show_browser: true,
            show_help: false,
            should_quit: false,
            navigator,
            editor,
            auto_save,
            file_watcher: None,
            current_filename: String::new(),
            focus_mode,
            prompt: BrowserPrompt::None,
            search_matches: None,
            editor_line_width,
            editor_render_width: editor_line_width,
            pending_external_change: false,
            browser_visibility_before_focus: true,
            status_message: None,
            last_conflict_backup_content: None,
            data_dir,
            browser_content_rect: Rect::default(),
            browser_scroll_offset: 0,
            update_check_rx,
            update_available: None,
            pending_c_prefix: false,
        };
        app.preview_selected_file();
        app
    }

    fn poll_update_check_result(&mut self) {
        if let Some(rx) = self.update_check_rx.take() {
            match rx.try_recv() {
                Ok(result) => {
                    if result.is_newer {
                        self.update_available = Some(result.latest_version);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.update_check_rx = Some(rx);
                }
                Err(mpsc::TryRecvError::Disconnected) => {}
            }
        }
    }

    /// Run the main event loop.
    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> anyhow::Result<()> {
        while !self.should_quit {
            self.prune_status_message();
            terminal.draw(|frame| self.draw(frame))?;

            // Poll-based event loop: check for input every 100ms
            if event::poll(Duration::from_millis(100))? {
                let ev = event::read()?;
                self.handle_event(&ev)?;
            }

            self.poll_update_check_result();

            // Auto-save check
            if self.config.editor.auto_save && self.auto_save.should_save() {
                let content = self.editor.get_content();
                if self.pending_external_change {
                    if let Err(err) = self.backup_conflict_copy_if_needed(&content) {
                        self.set_status_message(format!("Failed to back up local changes: {err}"));
                    }
                    self.auto_save.last_edit = None;
                } else if let Err(err) = self.auto_save.save(&content) {
                    self.set_status_message(format!("Save failed: {err}"));
                }
            }

            // File watcher check
            let mut changed_paths = Vec::new();
            if let Some(ref mut watcher) = self.file_watcher {
                match watcher.poll_changed() {
                    Ok(Some(path)) => changed_paths.push(path),
                    Ok(None) => {}
                    Err(err) => self.set_status_message(format!("File watch failed: {err}")),
                }
            }
            for path in changed_paths {
                self.handle_external_change(&path);
            }
        }

        // Save any pending changes before exiting
        let content = self.editor.get_content();
        if self.auto_save.dirty {
            if self.auto_save.path.is_some() {
                let _ = self.persist_content(&content, true);
            } else if !content.trim().is_empty() {
                // Untitled document with content — save to unsaved directory
                let _ = self.save_untitled(&content);
            }
        }

        Ok(())
    }

    /// Draw the full UI.
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let layout = compute_layout(area, self.config.browser.ratio, self.show_browser);

        // Fill background
        let bg_block = Block::default().style(self.theme.base_style());
        frame.render_widget(bg_block, area);

        // Browser panel
        if self.show_browser {
            let prompt_visible = self.prompt != BrowserPrompt::None;
            self.sync_browser_viewport(layout.browser, prompt_visible);
            let prompt_info = match &self.prompt {
                BrowserPrompt::None => None,
                BrowserPrompt::Create(ref pi) => Some(BrowserPromptInfo {
                    label: "Create: ",
                    input: &pi.text,
                    cursor: pi.cursor,
                }),
                BrowserPrompt::Rename(ref pi) => Some(BrowserPromptInfo {
                    label: "Rename: ",
                    input: &pi.text,
                    cursor: pi.cursor,
                }),
                BrowserPrompt::Delete => Some(BrowserPromptInfo {
                    label: "Delete? (y/n): ",
                    input: "",
                    cursor: 0,
                }),
                BrowserPrompt::Search(ref pi) => Some(BrowserPromptInfo {
                    label: "/",
                    input: &pi.text,
                    cursor: pi.cursor,
                }),
            };
            let visible = self.search_matches.as_deref();
            render_browser_with_prompt(
                frame,
                layout.browser,
                &self.navigator,
                &self.theme,
                prompt_info,
                visible,
            );
        } else {
            self.browser_content_rect = Rect::default();
            self.browser_scroll_offset = 0;
        }

        // Editor panel — centered content area
        let editor_area = centered_editor_area(layout.editor, self.editor_line_width);
        self.editor_render_width = editor_area.width;
        self.editor
            .render(frame, editor_area, &self.theme, self.focus_mode);

        // Status bar
        let content = self.editor.get_content();
        let wc = word_count::count_words(&content);
        let cc = word_count::count_chars(&content);

        let display_name = if self.current_filename.is_empty() {
            match self.mode {
                AppMode::Browse => "BROWSE".to_string(),
                AppMode::Edit => "EDIT".to_string(),
            }
        } else {
            self.current_filename.clone()
        };

        let center_label = self
            .status_message
            .as_ref()
            .map(|message| message.text.as_str())
            .unwrap_or_else(|| {
                if self.mode == AppMode::Edit {
                    self.focus_mode.label()
                } else {
                    ""
                }
            });

        render_status_bar(
            frame,
            layout.status_bar,
            &display_name,
            wc,
            cc,
            center_label,
            &self.theme,
            self.update_available.as_deref(),
        );

        // Help overlay
        if self.show_help {
            render_help(frame, area, &self.theme);
        }
    }

    fn set_status_message(&mut self, text: impl Into<String>) {
        self.status_message = Some(StatusMessage::new(text));
    }

    fn prune_status_message(&mut self) {
        if self
            .status_message
            .as_ref()
            .is_some_and(StatusMessage::is_expired)
        {
            self.status_message = None;
        }
    }

    fn toggle_browser_visibility(&mut self) {
        self.show_browser = !self.show_browser;
        self.browser_visibility_before_focus = self.show_browser;
    }

    fn set_focus_mode(&mut self, focus_mode: FocusMode) {
        if self.focus_mode == FocusMode::Off && focus_mode != FocusMode::Off {
            self.browser_visibility_before_focus = self.show_browser;
            self.show_browser = false;
        } else if self.focus_mode != FocusMode::Off && focus_mode == FocusMode::Off {
            self.show_browser = self.browser_visibility_before_focus;
        }

        self.focus_mode = focus_mode;
        self.editor.update_highlights(&self.theme, self.focus_mode);
    }

    fn handle_external_change(&mut self, path: &Path) {
        if self.auto_save.path.as_deref() != Some(path) {
            return;
        }

        match file_io::load_file(path) {
            Ok(content) => {
                if content == self.auto_save.last_save_content {
                    return;
                }

                if self.auto_save.dirty {
                    self.pending_external_change = true;
                    let local_content = self.editor.get_content();
                    match self.backup_conflict_copy_if_needed(&local_content) {
                        Ok(Some(path)) => self.set_status_message(format!(
                            "External change detected; local copy backed up to {}",
                            path.display()
                        )),
                        Ok(None) => self.set_status_message(
                            "External change detected; local edits kept in buffer",
                        ),
                        Err(err) => self.set_status_message(format!(
                            "External change detected, but backup failed: {err}"
                        )),
                    }
                    return;
                }

                self.editor.load_content(&content);
                self.editor.update_highlights(&self.theme, self.focus_mode);
                self.auto_save.last_save_content = content;
                self.auto_save.dirty = false;
                self.auto_save.last_edit = None;
                self.pending_external_change = false;
                self.last_conflict_backup_content = None;
                self.set_status_message("Reloaded external changes");
            }
            Err(err) => self.set_status_message(format!("Failed to reload file: {err}")),
        }
    }

    fn persist_content(&mut self, content: &str, force: bool) -> anyhow::Result<()> {
        if self.pending_external_change && self.auto_save.path.is_some() {
            if let Some(path) = self.backup_conflict_copy_if_needed(content)? {
                self.set_status_message(format!(
                    "External changes pending; local copy saved to {}",
                    path.display()
                ));
            } else {
                self.set_status_message(
                    "External changes pending; refusing to overwrite the on-disk file",
                );
            }
            return Ok(());
        }

        if self.auto_save.path.is_some() {
            if force {
                self.auto_save.force_save(content)?;
            } else {
                self.auto_save.save(content)?;
            }
        } else if !content.trim().is_empty() {
            self.save_untitled(content)?;
            self.auto_save.last_save_content = content.to_string();
            self.auto_save.dirty = false;
            self.auto_save.last_edit = None;
        }

        self.pending_external_change = false;
        self.last_conflict_backup_content = None;
        Ok(())
    }

    fn backup_conflict_copy_if_needed(&mut self, content: &str) -> anyhow::Result<Option<PathBuf>> {
        if self.auto_save.path.is_none()
            || content == self.last_conflict_backup_content.as_deref().unwrap_or("")
        {
            return Ok(None);
        }

        let conflicts_dir = self.data_dir.join("conflicts");
        fs::create_dir_all(&conflicts_dir)?;

        let original_path = self.auto_save.path.as_ref().expect("checked above");
        let stem = original_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| "document".to_string());
        let extension = original_path
            .extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .filter(|ext| !ext.is_empty())
            .unwrap_or_else(|| "md".to_string());

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let file_name = format!("{stem}-conflict-{timestamp}.{extension}");
        let conflict_path = conflicts_dir.join(file_name);
        file_io::save_file(&conflict_path, content)?;
        self.last_conflict_backup_content = Some(content.to_string());
        Ok(Some(conflict_path))
    }

    /// Handle a single event.
    fn handle_event(&mut self, event: &Event) -> anyhow::Result<()> {
        match event {
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return Ok(());
                }

                match self.mode {
                    AppMode::Browse => self.handle_browse_key(*key),
                    AppMode::Edit => self.handle_edit_key(*key, event.clone()),
                }
            }
            Event::Mouse(mouse) => {
                self.handle_mouse_event(mouse);
            }
            // Terminal resize: ratatui redraws automatically on the next
            // `terminal.draw()` call. No state invalidation needed — just
            // let the event pass through so the next draw picks up the new size.
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    fn handle_mouse_event(&mut self, mouse: &crossterm::event::MouseEvent) {
        if self.mode == AppMode::Edit {
            self.editor.handle_event(Event::Mouse(*mouse));
            self.editor.update_highlights(&self.theme, self.focus_mode);
            return;
        }

        if !self.show_browser || self.show_help || self.prompt != BrowserPrompt::None {
            return;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.mouse_in_browser_content(mouse) {
                    let clicked_row = (mouse.row - self.browser_content_rect.y) as usize;
                    let visible_index = self.browser_scroll_offset + clicked_row;
                    self.select_visible_entry(visible_index);
                }
            }
            MouseEventKind::ScrollUp => {
                if self.mouse_in_browser_content(mouse) {
                    self.navigator.move_up();
                    self.preview_selected_file();
                }
            }
            MouseEventKind::ScrollDown => {
                if self.mouse_in_browser_content(mouse) {
                    self.navigator.move_down();
                    self.preview_selected_file();
                }
            }
            _ => {}
        }
    }

    fn sync_browser_viewport(&mut self, area: Rect, prompt_visible: bool) {
        let (list_area, _) = split_browser_area(area, prompt_visible);
        self.browser_content_rect = browser_content_area(list_area);
        self.browser_scroll_offset = browser_scroll_offset(list_area, self.selected_list_index());
    }

    fn selected_list_index(&self) -> Option<usize> {
        if let Some(matches) = &self.search_matches {
            matches
                .iter()
                .position(|&index| index == self.navigator.selected)
        } else if self.navigator.entries.is_empty() {
            None
        } else {
            Some(self.navigator.selected)
        }
    }

    fn mouse_in_browser_content(&self, mouse: &crossterm::event::MouseEvent) -> bool {
        let rect = self.browser_content_rect;
        rect.width > 0
            && rect.height > 0
            && mouse.column >= rect.x
            && mouse.column < rect.x + rect.width
            && mouse.row >= rect.y
            && mouse.row < rect.y + rect.height
    }

    fn select_visible_entry(&mut self, visible_index: usize) {
        let selected = if let Some(matches) = &self.search_matches {
            matches.get(visible_index).copied()
        } else if visible_index < self.navigator.entries.len() {
            Some(visible_index)
        } else {
            None
        };

        if let Some(selected) = selected {
            self.navigator.selected = selected;
            self.preview_selected_file();
        }
    }

    /// Preview the currently selected file in the editor panel (read-only, no auto-save setup).
    /// Called as user navigates in Browse mode so the right panel updates live.
    fn preview_selected_file(&mut self) {
        if let Some(entry) = self.navigator.selected_entry() {
            if entry.kind == EntryKind::File {
                let path = self.navigator.current_dir.join(&entry.name);
                if let Ok(content) = file_io::load_file(&path) {
                    self.editor.load_content(&content);
                    self.editor.update_highlights(&self.theme, FocusMode::Off);
                    self.current_filename = entry.name.clone();
                }
            } else {
                // Directory selected — clear the editor preview
                self.editor.load_content("");
                self.current_filename = String::new();
            }
        } else {
            self.editor.load_content("");
            self.current_filename = String::new();
        }
    }

    fn select_browser_entry(&mut self, name: &str, kind: EntryKind) {
        let _ = self.navigator.select_entry_named(name, kind);
    }

    fn created_entry_name(name: &str) -> String {
        if name.ends_with(".md") || name.ends_with(".txt") {
            name.to_string()
        } else {
            format!("{name}.md")
        }
    }

    /// Open a file: load content, set up auto-save path, and create a file watcher.
    pub fn open_file(&mut self, path: &Path) {
        if let Err(err) = self.open_file_impl(path) {
            self.set_status_message(format!("Failed to open {}: {err}", path.display()));
        }
    }

    fn open_file_impl(&mut self, path: &Path) -> anyhow::Result<()> {
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            env::current_dir().unwrap_or_else(|_| ".".into()).join(path)
        };

        if !abs_path.exists() {
            let Some(parent) = abs_path.parent() else {
                anyhow::bail!("Invalid path");
            };
            anyhow::ensure!(
                parent.exists(),
                "Parent directory does not exist: {}",
                parent.display()
            );
            file_io::save_file(&abs_path, "")?;
            if parent == self.navigator.current_dir {
                self.navigator.refresh();
            }
            self.set_status_message(format!("Created {}", abs_path.display()));
        }

        let content = file_io::load_file(&abs_path)?;
        self.editor.load_content(&content);
        self.editor.update_highlights(&self.theme, self.focus_mode);

        self.current_filename = abs_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        self.auto_save.path = Some(abs_path.clone());
        self.auto_save.last_save_content = content;
        self.auto_save.dirty = false;
        self.auto_save.last_edit = None;
        self.pending_external_change = false;
        self.last_conflict_backup_content = None;

        match FileWatcher::new(&abs_path) {
            Ok(watcher) => self.file_watcher = Some(watcher),
            Err(err) => {
                self.file_watcher = None;
                self.set_status_message(format!("Opened file, but watcher failed: {err}"));
            }
        }

        self.mode = AppMode::Edit;
        if self.focus_mode != FocusMode::Off {
            self.browser_visibility_before_focus = self.show_browser;
            self.show_browser = false;
        }

        Ok(())
    }

    /// Handle key events in Browse mode.
    fn handle_browse_key(&mut self, key: KeyEvent) {
        // Prompts capture text input verbatim; do not rewrite IME characters.
        if self.prompt != BrowserPrompt::None {
            self.handle_prompt_key(key);
            return;
        }

        let key = normalize_browse_key(key);

        // Help screen intercepts all keys when visible
        if self.show_help {
            match key.code {
                KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.show_help = false;
                }
                _ => {}
            }
            return;
        }

        // Handle `c` prefix sequence (cc = copy path)
        if self.pending_c_prefix {
            self.pending_c_prefix = false;
            if key.code == KeyCode::Char('c') {
                if let Some(entry) = self.navigator.selected_entry() {
                    let full_path = self.navigator.current_dir.join(&entry.name);
                    let path_str = full_path.display().to_string();
                    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&path_str)) {
                        Ok(()) => self.set_status_message(format!("Copied: {path_str}")),
                        Err(err) => self.set_status_message(format!("Copy failed: {err}")),
                    }
                }
            }
            // Any other key after `c` just cancels the prefix
            return;
        }

        // Ctrl+E toggles browser panel in any mode
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
            self.toggle_browser_visibility();
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigator.move_up();
                self.preview_selected_file();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigator.move_down();
                self.preview_selected_file();
            }
            KeyCode::Right | KeyCode::Enter | KeyCode::Char('l') => {
                if let Some(entry) = self.navigator.selected_entry() {
                    if entry.kind == EntryKind::Directory {
                        self.navigator.enter_selected();
                        self.preview_selected_file();
                    } else {
                        // Selected a file — load its content and switch to Edit mode
                        let path = self.navigator.current_dir.join(&entry.name);
                        self.open_file(&path);
                    }
                }
            }
            KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => {
                self.navigator.go_up();
                self.preview_selected_file();
            }
            KeyCode::Char('.') => {
                self.navigator.toggle_hidden();
                self.preview_selected_file();
            }
            // ── File browser actions ──
            KeyCode::Char('a') => {
                self.prompt = BrowserPrompt::Create(PromptInput::empty());
            }
            KeyCode::Char('r') => {
                if let Some(entry) = self.navigator.selected_entry() {
                    self.prompt = BrowserPrompt::Rename(PromptInput::new(entry.name.clone()));
                }
            }
            KeyCode::Char('d') => {
                if self.navigator.selected_entry().is_some() {
                    self.prompt = BrowserPrompt::Delete;
                }
            }
            KeyCode::Char('/') => {
                self.prompt = BrowserPrompt::Search(PromptInput::empty());
                self.search_matches = Some(self.navigator.filter_entries(""));
            }
            KeyCode::Char('c') => {
                self.pending_c_prefix = true;
                self.set_status_message("c-");
                // wait for the next key
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            _ => {}
        }
    }

    /// Handle key events directed at the active browser prompt.
    fn handle_prompt_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                // Cancel prompt
                self.prompt = BrowserPrompt::None;
                self.search_matches = None;
            }
            KeyCode::Enter => {
                self.confirm_prompt();
            }
            KeyCode::Backspace => {
                match &mut self.prompt {
                    BrowserPrompt::Create(pi)
                    | BrowserPrompt::Rename(pi)
                    | BrowserPrompt::Search(pi) => {
                        pi.backspace();
                    }
                    _ => {}
                }
                self.update_search_matches();
            }
            KeyCode::Delete => {
                match &mut self.prompt {
                    BrowserPrompt::Create(pi)
                    | BrowserPrompt::Rename(pi)
                    | BrowserPrompt::Search(pi) => {
                        pi.delete();
                    }
                    _ => {}
                }
                self.update_search_matches();
            }
            KeyCode::Left => match &mut self.prompt {
                BrowserPrompt::Create(pi)
                | BrowserPrompt::Rename(pi)
                | BrowserPrompt::Search(pi) => {
                    pi.move_left();
                }
                _ => {}
            },
            KeyCode::Right => match &mut self.prompt {
                BrowserPrompt::Create(pi)
                | BrowserPrompt::Rename(pi)
                | BrowserPrompt::Search(pi) => {
                    pi.move_right();
                }
                _ => {}
            },
            KeyCode::Char(c) => {
                match &mut self.prompt {
                    BrowserPrompt::Create(pi)
                    | BrowserPrompt::Rename(pi)
                    | BrowserPrompt::Search(pi) => {
                        pi.insert(c);
                    }
                    BrowserPrompt::Delete => {
                        // Only y/Y confirms
                        if c == 'y' || c == 'Y' {
                            self.confirm_prompt();
                            return;
                        } else if c == 'n' || c == 'N' {
                            self.prompt = BrowserPrompt::None;
                            return;
                        }
                    }
                    BrowserPrompt::None => {}
                }
                self.update_search_matches();
            }
            _ => {}
        }
    }

    /// Update search matches when the search prompt text changes.
    fn update_search_matches(&mut self) {
        if let BrowserPrompt::Search(ref pi) = self.prompt {
            let matches = self.navigator.filter_entries(&pi.text);
            if let Some(&first) = matches.first() {
                self.navigator.selected = first;
            }
            self.search_matches = Some(matches);
        }
    }

    /// Execute the confirmed prompt action.
    fn confirm_prompt(&mut self) {
        let prompt = std::mem::replace(&mut self.prompt, BrowserPrompt::None);
        match prompt {
            BrowserPrompt::Create(pi) => {
                let trimmed = pi.text.trim();
                if !trimmed.is_empty() {
                    if trimmed.ends_with('/') {
                        let dir_name = trimmed.trim_end_matches('/');
                        match actions::create_directory(&self.navigator.current_dir, dir_name) {
                            Ok(()) => {
                                self.navigator.refresh();
                                self.select_browser_entry(dir_name, EntryKind::Directory);
                                self.preview_selected_file();
                                self.set_status_message(format!("Created directory: {dir_name}"));
                            }
                            Err(err) => {
                                self.set_status_message(format!("Create failed: {err}"));
                            }
                        }
                    } else {
                        match actions::create_file(&self.navigator.current_dir, trimmed) {
                            Ok(()) => {
                                let file_name = Self::created_entry_name(trimmed);
                                self.navigator.refresh();
                                self.select_browser_entry(&file_name, EntryKind::File);
                                self.preview_selected_file();
                                self.set_status_message(format!("Created file: {file_name}"));
                            }
                            Err(err) => {
                                self.set_status_message(format!("Create failed: {err}"));
                            }
                        }
                    }
                }
            }
            BrowserPrompt::Rename(pi) => {
                let trimmed_new_name = pi.text.trim().to_string();
                if let Some(entry) = self.navigator.selected_entry().cloned() {
                    let old_name = entry.name;
                    let kind = entry.kind;
                    if !trimmed_new_name.is_empty() && trimmed_new_name != old_name {
                        match actions::rename_entry(
                            &self.navigator.current_dir,
                            &old_name,
                            &trimmed_new_name,
                        ) {
                            Ok(()) => {
                                self.navigator.refresh();
                                self.select_browser_entry(&trimmed_new_name, kind);
                                self.preview_selected_file();
                                self.set_status_message(format!(
                                    "Renamed {old_name} -> {trimmed_new_name}"
                                ));
                            }
                            Err(err) => {
                                self.set_status_message(format!("Rename failed: {err}"));
                            }
                        }
                    }
                }
            }
            BrowserPrompt::Delete => {
                if let Some(entry) = self.navigator.selected_entry().cloned() {
                    let name = entry.name;
                    match actions::delete_entry(&self.navigator.current_dir, &name) {
                        Ok(()) => {
                            self.navigator.refresh();
                            self.preview_selected_file();
                            self.set_status_message(format!("Deleted {name}"));
                        }
                        Err(err) => {
                            self.set_status_message(format!("Delete failed: {err}"));
                        }
                    }
                }
            }
            BrowserPrompt::Search(_) => {
                // On Enter, accept current selection and exit search.
                self.search_matches = None;
            }
            BrowserPrompt::None => {}
        }
    }

    /// Handle key events in Edit mode.
    ///
    /// We intercept Esc, Ctrl+E, and Ctrl+S before passing the event to edtui.
    fn handle_edit_key(&mut self, key: KeyEvent, event: Event) {
        let key = normalize_key(key);

        // Ctrl+E toggles browser panel
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
            self.toggle_browser_visibility();
            return;
        }

        // Ctrl+S saves immediately
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            let content = self.editor.get_content();
            if let Err(err) = self.persist_content(&content, true) {
                self.set_status_message(format!("Save failed: {err}"));
            }
            return;
        }

        // Ctrl+X: cut selected text while staying in insert mode.
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('x')
            && self.editor.state.selection.is_some()
        {
            use edtui::actions::{CopySelection, DeleteSelection, Execute, SwitchMode};

            let selection = self.editor.state.selection.clone();
            CopySelection.execute(&mut self.editor.state);
            self.editor.state.selection = selection;
            DeleteSelection.execute(&mut self.editor.state);
            SwitchMode(edtui::EditorMode::Insert).execute(&mut self.editor.state);

            self.editor.update_highlights(&self.theme, self.focus_mode);
            self.auto_save.mark_edited();
            return;
        }

        // Ctrl+F cycles focus mode
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f') {
            self.set_focus_mode(self.focus_mode.cycle());
            return;
        }

        // ── Formatting Shortcuts ──────────────────────────────────

        // Ctrl+1 through Ctrl+6: toggle heading level on current line
        let heading_level = if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('1') => Some(1),
                KeyCode::Char('2') => Some(2),
                KeyCode::Char('3') => Some(3),
                KeyCode::Char('4') => Some(4),
                KeyCode::Char('5') => Some(5),
                KeyCode::Char('6') => Some(6),
                _ => None,
            }
        } else {
            match key.code {
                KeyCode::F(1) => Some(1),
                KeyCode::F(2) => Some(2),
                KeyCode::F(3) => Some(3),
                KeyCode::F(4) => Some(4),
                KeyCode::F(5) => Some(5),
                KeyCode::F(6) => Some(6),
                _ => None,
            }
        };
        if let Some(level) = heading_level {
            self.apply_heading_toggle(level);
            return;
        }

        // Ctrl+B: toggle bold
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
            self.apply_inline_format("**");
            return;
        }

        // Ctrl+I: toggle italic. Ctrl+T is a portable fallback on terminals
        // that still collapse Ctrl+I into Tab.
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('i') | KeyCode::Char('t'))
        {
            self.apply_inline_format("*");
            return;
        }

        // Ctrl+K: insert/wrap link
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('k') {
            self.apply_link_format();
            return;
        }

        // Ctrl+H: toggle highlight
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('h') {
            self.apply_inline_format("==");
            return;
        }

        // Ctrl+U: toggle underline
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('u') {
            self.apply_inline_format_pair("<u>", "</u>");
            return;
        }

        // Ctrl+D: toggle strikethrough
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('d') {
            self.apply_inline_format("~~");
            return;
        }

        // Esc: exit Visual mode first, then exit edit mode on second press
        if key.code == KeyCode::Esc {
            if self.editor.state.mode == edtui::EditorMode::Visual {
                use edtui::actions::{Execute, SwitchMode};
                self.editor.state.selection = None;
                SwitchMode(edtui::EditorMode::Insert).execute(&mut self.editor.state);
                self.editor.update_highlights(&self.theme, self.focus_mode);
                return;
            }

            // Turn off focus mode if active
            if self.focus_mode != FocusMode::Off {
                self.set_focus_mode(FocusMode::Off);
            }

            // Force-save before leaving edit mode if dirty
            if self.auto_save.dirty {
                let content = self.editor.get_content();
                if let Err(err) = self.persist_content(&content, true) {
                    self.set_status_message(format!("Save failed: {err}"));
                }
            }

            // Return to Browse mode
            self.mode = AppMode::Browse;
            self.show_browser = self.browser_visibility_before_focus;
            return;
        }

        if self.handle_wrapped_vertical_move(key) {
            return;
        }

        // Everything else goes to the editor
        let content_before = self.editor.get_content();
        self.editor.handle_event(event);

        // Re-highlight after the edit
        self.editor.update_highlights(&self.theme, self.focus_mode);

        // Only mark dirty when content actually changes.
        if self.editor.get_content() != content_before {
            self.auto_save.mark_edited();
        }
    }

    fn wrapped_movement_width(&self) -> usize {
        usize::from(self.editor_render_width.max(1))
    }

    fn handle_wrapped_vertical_move(&mut self, key: KeyEvent) -> bool {
        if !matches!(key.code, KeyCode::Up | KeyCode::Down) {
            return false;
        }

        if !(key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT) {
            return false;
        }

        use edtui::actions::{Execute, SwitchMode};

        if key.modifiers == KeyModifiers::SHIFT
            && self.editor.state.mode != edtui::EditorMode::Visual
        {
            SwitchMode(edtui::EditorMode::Visual).execute(&mut self.editor.state);
        }

        let width = self.wrapped_movement_width();
        if key.code == KeyCode::Up {
            edtui::actions::MoveUpWrapped { width }.execute(&mut self.editor.state);
        } else {
            edtui::actions::MoveDownWrapped { width }.execute(&mut self.editor.state);
        }

        self.editor.update_highlights(&self.theme, self.focus_mode);
        true
    }

    // ── Formatting helpers ──────────────────────────────────────

    /// Toggle a heading level on the current line.
    ///
    /// Reads the current line from the editor, applies `toggle_heading`,
    /// and replaces the line in-place.
    fn apply_heading_toggle(&mut self, level: usize) {
        use edtui::RowIndex;

        self.editor.state.checkpoint();
        let row = self.editor.state.cursor.row;

        // Read the current line content.
        let line: String = match self.editor.state.lines.get(RowIndex::new(row)) {
            Some(chars) => chars.iter().collect(),
            None => return,
        };

        let new_line = formatting::toggle_heading(&line, level);
        let new_chars: Vec<char> = new_line.chars().collect();
        let new_len = new_chars.len();

        // Replace the row content.
        if let Some(row_mut) = self.editor.state.lines.get_mut(RowIndex::new(row)) {
            row_mut.clear();
            row_mut.extend(new_chars.iter());
        }

        // Clamp cursor column to the new line length.
        if self.editor.state.cursor.col >= new_len {
            self.editor.state.cursor.col = new_len.saturating_sub(1);
        }

        self.editor.update_highlights(&self.theme, self.focus_mode);
        self.auto_save.mark_edited();
    }

    /// Toggle an inline formatting marker (bold, italic, strikethrough)
    /// on the selected text, or insert the marker pair at cursor if no
    /// selection is active.
    fn apply_inline_format(&mut self, marker: &str) {
        self.editor.state.checkpoint();
        if let Some(ref selection) = self.editor.state.selection.clone() {
            // There is a selection — get the text, toggle, and replace.
            let selected_lines = selection.copy_from(&self.editor.state.lines);
            let selected_text = selected_lines.to_string();
            let new_text = formatting::toggle_marker(&selected_text, marker);

            // Delete the selection.
            let start = selection.start();
            let _ = selection.extract_from(&mut self.editor.state.lines);
            self.editor.state.cursor = start;
            self.editor.state.selection = None;

            // Insert the new text at cursor.
            for ch in new_text.chars() {
                use edtui::actions::Execute;
                use edtui::actions::InsertChar;
                InsertChar(ch).execute(&mut self.editor.state);
            }
        } else {
            // No selection — insert marker pair and place cursor between them.
            use edtui::actions::Execute;
            use edtui::actions::InsertChar;
            for ch in marker.chars() {
                InsertChar(ch).execute(&mut self.editor.state);
            }
            for ch in marker.chars() {
                InsertChar(ch).execute(&mut self.editor.state);
            }
            // Move cursor back by marker length so it sits between the markers.
            let marker_len = marker.len();
            self.editor.state.cursor.col = self.editor.state.cursor.col.saturating_sub(marker_len);
        }

        self.editor.state.mode = edtui::EditorMode::Insert;
        self.editor.update_highlights(&self.theme, self.focus_mode);
        self.auto_save.mark_edited();
    }

    fn apply_inline_format_pair(&mut self, open: &str, close: &str) {
        self.editor.state.checkpoint();
        if let Some(ref selection) = self.editor.state.selection.clone() {
            let selected_lines = selection.copy_from(&self.editor.state.lines);
            let selected_text = selected_lines.to_string();
            let new_text = formatting::toggle_marker_pair(&selected_text, open, close);

            let start = selection.start();
            let _ = selection.extract_from(&mut self.editor.state.lines);
            self.editor.state.cursor = start;
            self.editor.state.selection = None;

            for ch in new_text.chars() {
                use edtui::actions::Execute;
                use edtui::actions::InsertChar;
                InsertChar(ch).execute(&mut self.editor.state);
            }
        } else {
            use edtui::actions::Execute;
            use edtui::actions::InsertChar;
            for ch in open.chars() {
                InsertChar(ch).execute(&mut self.editor.state);
            }
            for ch in close.chars() {
                InsertChar(ch).execute(&mut self.editor.state);
            }
            let close_len = close.chars().count();
            self.editor.state.cursor.col = self.editor.state.cursor.col.saturating_sub(close_len);
        }

        self.editor.state.mode = edtui::EditorMode::Insert;
        self.editor.update_highlights(&self.theme, self.focus_mode);
        self.auto_save.mark_edited();
    }

    /// Save an untitled document to `~/.local/share/deepwrite/unsaved/{timestamp}.md`.
    fn save_untitled(&self, content: &str) -> anyhow::Result<()> {
        let data_dir = self.data_dir.join("unsaved");
        fs::create_dir_all(&data_dir)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let filename = format!("{}.md", timestamp);
        let path = data_dir.join(filename);
        file_io::save_file(&path, content)?;
        Ok(())
    }

    /// Insert a Markdown link template at the cursor or wrap the selection.
    fn apply_link_format(&mut self) {
        self.editor.state.checkpoint();
        if let Some(ref selection) = self.editor.state.selection.clone() {
            let selected_lines = selection.copy_from(&self.editor.state.lines);
            let selected_text = selected_lines.to_string();
            let link_text = formatting::link_template(&selected_text);

            // Delete the selection.
            let start = selection.start();
            let _ = selection.extract_from(&mut self.editor.state.lines);
            self.editor.state.cursor = start;
            self.editor.state.selection = None;

            // Insert the link template.
            use edtui::actions::Execute;
            use edtui::actions::InsertChar;
            for ch in link_text.chars() {
                InsertChar(ch).execute(&mut self.editor.state);
            }
        } else {
            // No selection — insert empty link template [](url).
            let link_text = formatting::link_template("");
            use edtui::actions::Execute;
            use edtui::actions::InsertChar;
            for ch in link_text.chars() {
                InsertChar(ch).execute(&mut self.editor.state);
            }
            // Place cursor inside the brackets: [|](url)
            // link_text is "[](url)" = 7 chars, cursor is after all, move back 6
            self.editor.state.cursor.col = self.editor.state.cursor.col.saturating_sub(6);
        }

        self.editor.state.mode = edtui::EditorMode::Insert;
        self.editor.update_highlights(&self.theme, self.focus_mode);
        self.auto_save.mark_edited();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, MouseEvent};
    use ratatui::{backend::TestBackend, Terminal};
    use std::sync::mpsc;
    use tempfile::TempDir;

    fn test_app(root: &TempDir) -> App {
        let mut app =
            App::new_with_update_receiver(Config::default(), root.path().to_path_buf(), None);
        app.data_dir = root.path().join("app-data");
        app
    }

    #[test]
    fn app_uses_editor_and_focus_config_values() {
        let tmp = TempDir::new().unwrap();
        let config = Config::from_toml_str(
            r#"
            [editor]
            line_width = 64
            auto_save = false
            auto_save_delay_ms = 1500

            [focus]
            mode = "sentence"
            opacity = 10
            "#,
        )
        .unwrap();

        let app = App::new_with_update_receiver(config, tmp.path().to_path_buf(), None);
        assert_eq!(app.editor_line_width, 64);
        assert_eq!(app.editor_render_width, 64);
        assert_eq!(app.auto_save.delay, Duration::from_millis(1500));
        assert_eq!(app.focus_mode, FocusMode::Sentence);
    }

    #[test]
    fn draw_tracks_actual_editor_render_width() {
        let tmp = TempDir::new().unwrap();
        let mut app = test_app(&tmp);
        app.show_browser = false;

        let backend = TestBackend::new(20, 6);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();

        assert_eq!(app.editor_render_width, 20);
    }

    #[test]
    fn poll_update_check_result_sets_update_available_when_newer() {
        let tmp = TempDir::new().unwrap();
        let (tx, rx) = mpsc::channel();
        let mut app =
            App::new_with_update_receiver(Config::default(), tmp.path().to_path_buf(), Some(rx));

        tx.send(update_checker::UpdateCheckResult {
            latest_version: "0.2.0".to_string(),
            is_newer: true,
        })
        .unwrap();

        app.poll_update_check_result();

        assert_eq!(app.update_available.as_deref(), Some("0.2.0"));
        assert!(app.update_check_rx.is_none());
    }

    #[test]
    fn poll_update_check_result_requeues_empty_receiver() {
        let tmp = TempDir::new().unwrap();
        let (_tx, rx) = mpsc::channel();
        let mut app =
            App::new_with_update_receiver(Config::default(), tmp.path().to_path_buf(), Some(rx));

        app.poll_update_check_result();

        assert!(app.update_available.is_none());
        assert!(app.update_check_rx.is_some());
    }

    #[test]
    fn poll_update_check_result_ignores_non_newer_version() {
        let tmp = TempDir::new().unwrap();
        let (tx, rx) = mpsc::channel();
        let mut app =
            App::new_with_update_receiver(Config::default(), tmp.path().to_path_buf(), Some(rx));

        tx.send(update_checker::UpdateCheckResult {
            latest_version: "0.1.0".to_string(),
            is_newer: false,
        })
        .unwrap();

        app.poll_update_check_result();

        assert!(app.update_available.is_none());
        assert!(app.update_check_rx.is_none());
    }

    #[test]
    fn open_file_creates_missing_path() {
        let tmp = TempDir::new().unwrap();
        let mut app = test_app(&tmp);
        let path = tmp.path().join("new-file.md");

        app.open_file(&path);

        assert!(path.exists());
        assert_eq!(app.mode, AppMode::Edit);
        assert_eq!(app.current_filename, "new-file.md");
    }

    #[test]
    fn cursor_motion_does_not_mark_document_dirty() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "Hello").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);
        app.auto_save.dirty = false;

        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        app.handle_edit_key(key, Event::Key(key));

        assert!(!app.auto_save.dirty);
    }

    #[test]
    fn wrapped_vertical_movement_uses_last_render_width() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "ABCDE").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);
        app.editor_render_width = 3;

        let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        app.handle_edit_key(right, Event::Key(right));

        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.handle_edit_key(down, Event::Key(down));

        assert_eq!(app.editor.state.cursor, edtui::Index2::new(0, 4));
    }

    #[test]
    fn shift_down_starts_visual_selection_with_wrapped_motion() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "ABCDE").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);
        app.editor_render_width = 3;

        let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        app.handle_edit_key(right, Event::Key(right));

        let shift_down = KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT);
        app.handle_edit_key(shift_down, Event::Key(shift_down));

        let selection = app
            .editor
            .state
            .selection
            .as_ref()
            .expect("expected selection after Shift+Down");
        assert_eq!(app.editor.state.mode, edtui::EditorMode::Visual);
        assert_eq!(app.editor.state.cursor, edtui::Index2::new(0, 4));
        assert_eq!(selection.start, edtui::Index2::new(0, 1));
        assert_eq!(selection.end, edtui::Index2::new(0, 4));
    }

    #[test]
    fn ctrl_x_cuts_selected_text() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "Hello").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);

        let select = KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT);
        app.handle_edit_key(select, Event::Key(select));
        app.handle_edit_key(select, Event::Key(select));

        let cut = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        app.handle_edit_key(cut, Event::Key(cut));

        assert_eq!(app.editor.get_content(), "lo");
        assert_eq!(app.editor.state.mode, edtui::EditorMode::Insert);
        assert!(app.auto_save.dirty);
    }

    #[test]
    fn external_change_reloads_when_buffer_is_clean() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "Before").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);

        fs::write(&file_path, "After").unwrap();
        app.handle_external_change(&file_path);

        assert_eq!(app.editor.get_content(), "After");
        assert!(!app.pending_external_change);
    }

    #[test]
    fn external_change_while_dirty_creates_conflict_backup() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "Before").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);
        app.editor.load_content("Local edits");
        app.auto_save.mark_edited();

        fs::write(&file_path, "External edits").unwrap();
        app.handle_external_change(&file_path);

        assert_eq!(app.editor.get_content(), "Local edits");
        assert!(app.pending_external_change);
        assert_eq!(
            app.last_conflict_backup_content.as_deref(),
            Some("Local edits")
        );

        let conflict_dir = app.data_dir.join("conflicts");
        let conflict_files: Vec<_> = fs::read_dir(conflict_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(conflict_files.len(), 1);
        assert_eq!(
            fs::read_to_string(&conflict_files[0]).unwrap(),
            "Local edits"
        );
    }

    #[test]
    fn persist_content_does_not_overwrite_when_external_change_is_pending() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "On disk").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);
        app.editor.load_content("Local edits");
        app.auto_save.mark_edited();
        app.pending_external_change = true;

        app.persist_content("Local edits", true).unwrap();

        assert_eq!(fs::read_to_string(&file_path).unwrap(), "On disk");
        let conflict_dir = app.data_dir.join("conflicts");
        assert_eq!(fs::read_dir(conflict_dir).unwrap().count(), 1);
    }

    #[test]
    fn create_prompt_without_slash_creates_file() {
        let tmp = TempDir::new().unwrap();
        let mut app = test_app(&tmp);
        let a_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        app.handle_browse_key(a_key);
        assert_eq!(app.prompt, BrowserPrompt::Create(PromptInput::empty()));
        for c in "notes".chars() {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            app.handle_prompt_key(key);
        }
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_prompt_key(enter);
        assert!(tmp.path().join("notes.md").exists());
        assert_eq!(app.prompt, BrowserPrompt::None);
    }

    #[test]
    fn yank_path_does_not_panic_without_selection() {
        let tmp = TempDir::new().unwrap();
        let empty = tmp.path().join("empty");
        std::fs::create_dir(&empty).unwrap();
        let mut app = App::new_with_update_receiver(Config::default(), empty, None);
        app.data_dir = tmp.path().join("app-data");

        // Pressing 'y' with no entries should not panic
        let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        app.handle_browse_key(y_key);
    }

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

    #[test]
    fn mouse_click_uses_scroll_offset_for_visible_entries() {
        let tmp = TempDir::new().unwrap();
        for i in 0..20 {
            fs::write(tmp.path().join(format!("note-{i:02}.md")), "").unwrap();
        }

        let mut app = test_app(&tmp);
        app.navigator.selected = 10;
        app.sync_browser_viewport(Rect::new(0, 0, 20, 11), false);

        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: app.browser_content_rect.x,
            row: app.browser_content_rect.y + 2,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse_event(&click);

        assert_eq!(app.browser_scroll_offset, 6);
        assert_eq!(app.navigator.selected, 8);
    }

    #[test]
    fn mouse_scroll_ignores_events_outside_browser_content() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("note.md"), "").unwrap();

        let mut app = test_app(&tmp);
        app.navigator.selected = 0;
        app.sync_browser_viewport(Rect::new(0, 0, 20, 6), false);

        let scroll = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: app.browser_content_rect.x + app.browser_content_rect.width,
            row: app.browser_content_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse_event(&scroll);

        assert_eq!(app.navigator.selected, 0);
    }

    #[test]
    fn mouse_input_is_ignored_while_help_or_prompt_is_visible() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.md"), "").unwrap();
        fs::write(tmp.path().join("b.md"), "").unwrap();

        let mut app = test_app(&tmp);
        app.sync_browser_viewport(Rect::new(0, 0, 20, 6), false);

        let scroll = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: app.browser_content_rect.x,
            row: app.browser_content_rect.y,
            modifiers: KeyModifiers::NONE,
        };

        app.show_help = true;
        app.handle_mouse_event(&scroll);
        assert_eq!(app.navigator.selected, 0);

        app.show_help = false;
        app.prompt = BrowserPrompt::Search(PromptInput::empty());
        app.handle_mouse_event(&scroll);
        assert_eq!(app.navigator.selected, 0);
    }

    #[test]
    fn app_previews_first_file_on_launch() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("alpha.md"), "Alpha").unwrap();
        fs::write(tmp.path().join("beta.md"), "Beta").unwrap();

        let app = test_app(&tmp);

        assert_eq!(app.current_filename, "alpha.md");
        assert_eq!(app.editor.get_content(), "Alpha");
    }

    #[test]
    fn duplicate_create_sets_status_message() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("notes.md"), "").unwrap();

        let mut app = test_app(&tmp);
        app.prompt = BrowserPrompt::Create(PromptInput::new("notes".to_string()));
        app.confirm_prompt();

        let message = app
            .status_message
            .as_ref()
            .expect("expected status message")
            .text
            .clone();
        assert!(message.contains("Create failed"));
    }

    #[test]
    fn toggle_hidden_preserves_previewed_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".secret.md"), "Secret").unwrap();
        fs::write(tmp.path().join("alpha.md"), "Alpha").unwrap();
        fs::write(tmp.path().join("visible.md"), "Visible").unwrap();

        let mut app = test_app(&tmp);
        app.navigator.selected = app
            .navigator
            .entries
            .iter()
            .position(|entry| entry.name == "visible.md")
            .expect("expected visible file");
        app.preview_selected_file();
        assert_eq!(app.current_filename, "visible.md");

        let dot = KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE);
        app.handle_browse_key(dot);

        assert_eq!(app.current_filename, "visible.md");
        assert_eq!(app.editor.get_content(), "Visible");
    }

    #[test]
    fn ctrl_t_applies_italic_fallback_shortcut() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "Hello").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);

        let move_right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        for _ in 0..5 {
            app.handle_edit_key(move_right, Event::Key(move_right));
        }

        let italic = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
        app.handle_edit_key(italic, Event::Key(italic));

        assert_eq!(app.editor.get_content(), "Hello**");
    }

    #[test]
    fn browse_prompt_keeps_zhuyin_input_verbatim() {
        let tmp = TempDir::new().unwrap();
        let mut app = test_app(&tmp);
        app.prompt = BrowserPrompt::Create(PromptInput::empty());

        let key = KeyEvent::new(KeyCode::Char('ㄅ'), KeyModifiers::NONE);
        app.handle_browse_key(key);

        assert_eq!(
            app.prompt,
            BrowserPrompt::Create(PromptInput::new("ㄅ".to_string()))
        );
    }

    #[test]
    fn function_keys_toggle_heading_levels() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "Hello").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);

        let heading = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        app.handle_edit_key(heading, Event::Key(heading));

        assert_eq!(app.editor.get_content(), "# Hello");
    }

    #[test]
    fn formatting_shortcuts_undo_as_single_action() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("note.md");
        fs::write(&file_path, "Hello").unwrap();

        let mut app = test_app(&tmp);
        app.open_file(&file_path);

        let move_right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        for _ in 0..5 {
            app.handle_edit_key(move_right, Event::Key(move_right));
        }

        let bold = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        app.handle_edit_key(bold, Event::Key(bold));
        assert_eq!(app.editor.get_content(), "Hello****");

        let undo = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
        app.handle_edit_key(undo, Event::Key(undo));

        assert_eq!(app.editor.get_content(), "Hello");
    }
}
