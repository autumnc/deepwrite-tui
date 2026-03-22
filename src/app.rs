use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{prelude::*, widgets::Block};

use deepwrite::browser::actions;
use deepwrite::browser::entries::EntryKind;
use deepwrite::browser::navigator::Navigator;
use deepwrite::browser::widget::{render_browser_with_prompt, BrowserPromptInfo};
use deepwrite::config::Config;
use deepwrite::editor::centered_editor_area;
use deepwrite::editor::focus::FocusMode;
use deepwrite::editor::formatting;
use deepwrite::editor::word_count;
use deepwrite::editor::EditorWrapper;
use deepwrite::services::auto_save::AutoSave;
use deepwrite::services::file_io;
use deepwrite::services::file_watcher::FileWatcher;
use deepwrite::theme::Theme;
use deepwrite::ui::help::render_help;
use deepwrite::ui::layout::compute_layout;
use deepwrite::ui::status_bar::render_status_bar;

/// The current interaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Browse,
    Edit,
}

/// Active prompt overlay in the browser panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserPrompt {
    None,
    Create(String),
    Rename(String),
    Delete,
    Search(String),
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
    pub pending_external_change: bool,
    browser_visibility_before_focus: bool,
    status_message: Option<StatusMessage>,
    last_conflict_backup_content: Option<String>,
    data_dir: PathBuf,
    browser_rect: Rect,
}

impl App {
    /// Create a new App from the given configuration, rooted in `start_dir`.
    pub fn new(config: Config, start_dir: std::path::PathBuf) -> Self {
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

        Self {
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
            pending_external_change: false,
            browser_visibility_before_focus: true,
            status_message: None,
            last_conflict_backup_content: None,
            data_dir,
            browser_rect: Rect::default(),
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
            if let Some(ref watcher) = self.file_watcher {
                while let Ok(path) = watcher.rx.try_recv() {
                    changed_paths.push(path);
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
        let layout = compute_layout(area, self.config.browser.panel_width, self.show_browser);
        self.browser_rect = layout.browser;

        // Fill background
        let bg_block = Block::default().style(self.theme.base_style());
        frame.render_widget(bg_block, area);

        // Browser panel
        if self.show_browser {
            let prompt_info = match &self.prompt {
                BrowserPrompt::None => None,
                BrowserPrompt::Create(buf) => Some(BrowserPromptInfo {
                    label: "Create: ",
                    input: buf,
                }),
                BrowserPrompt::Rename(buf) => Some(BrowserPromptInfo {
                    label: "Rename: ",
                    input: buf,
                }),
                BrowserPrompt::Delete => Some(BrowserPromptInfo {
                    label: "Delete? (y/n): ",
                    input: "",
                }),
                BrowserPrompt::Search(buf) => Some(BrowserPromptInfo {
                    label: "/",
                    input: buf,
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
        }

        // Editor panel — centered content area
        let editor_area = centered_editor_area(layout.editor, self.editor_line_width);
        self.editor
            .render(frame, editor_area, &self.theme, self.focus_mode);

        // Help overlay
        if self.show_help {
            render_help(frame, area, &self.theme);
        }

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
        );
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
                        // The block has Borders::RIGHT + a title row, so list
                        // content starts at br.y + 1. Ignore clicks on the title.
                        if mouse.row <= br.y {
                            return;
                        }
                        let clicked_row = (mouse.row - br.y - 1) as usize;
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

        // If a prompt is active, route keys to the prompt handler.
        if self.prompt != BrowserPrompt::None {
            self.handle_prompt_key(key);
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
            KeyCode::Char('.') => self.navigator.toggle_hidden(),
            // ── File browser actions ──
            KeyCode::Char('a') => {
                self.prompt = BrowserPrompt::Create(String::new());
            }
            KeyCode::Char('r') => {
                if let Some(entry) = self.navigator.selected_entry() {
                    self.prompt = BrowserPrompt::Rename(entry.name.clone());
                }
            }
            KeyCode::Char('d') => {
                if self.navigator.selected_entry().is_some() {
                    self.prompt = BrowserPrompt::Delete;
                }
            }
            KeyCode::Char('/') => {
                self.prompt = BrowserPrompt::Search(String::new());
                self.search_matches = Some(self.navigator.filter_entries(""));
            }
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
                    BrowserPrompt::Create(buf)
                    | BrowserPrompt::Rename(buf)
                    | BrowserPrompt::Search(buf) => {
                        buf.pop();
                    }
                    _ => {}
                }
                // Update search matches live.
                if let BrowserPrompt::Search(ref query) = self.prompt {
                    let matches = self.navigator.filter_entries(query);
                    // Move selection to first match.
                    if let Some(&first) = matches.first() {
                        self.navigator.selected = first;
                    }
                    self.search_matches = Some(matches);
                }
            }
            KeyCode::Char(c) => {
                match &mut self.prompt {
                    BrowserPrompt::Create(buf)
                    | BrowserPrompt::Rename(buf)
                    | BrowserPrompt::Search(buf) => {
                        buf.push(c);
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
                // Update search matches live.
                if let BrowserPrompt::Search(ref query) = self.prompt {
                    let matches = self.navigator.filter_entries(query);
                    if let Some(&first) = matches.first() {
                        self.navigator.selected = first;
                    }
                    self.search_matches = Some(matches);
                }
            }
            _ => {}
        }
    }

    /// Execute the confirmed prompt action.
    fn confirm_prompt(&mut self) {
        let prompt = std::mem::replace(&mut self.prompt, BrowserPrompt::None);
        match prompt {
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
            BrowserPrompt::Rename(new_name) => {
                if let Some(entry) = self.navigator.selected_entry() {
                    let old_name = entry.name.clone();
                    if !new_name.trim().is_empty() && new_name != old_name {
                        let _ = actions::rename_entry(
                            &self.navigator.current_dir,
                            &old_name,
                            &new_name,
                        );
                        self.navigator.refresh();
                    }
                }
            }
            BrowserPrompt::Delete => {
                if let Some(entry) = self.navigator.selected_entry() {
                    let name = entry.name.clone();
                    let _ = actions::delete_entry(&self.navigator.current_dir, &name);
                    self.navigator.refresh();
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

        // Ctrl+D cycles focus mode
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('d') {
            self.set_focus_mode(self.focus_mode.cycle());
            return;
        }

        // ── Formatting Shortcuts ──────────────────────────────────

        // Ctrl+1 through Ctrl+6: toggle heading level on current line
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            let heading_level = match key.code {
                KeyCode::Char('1') => Some(1),
                KeyCode::Char('2') => Some(2),
                KeyCode::Char('3') => Some(3),
                KeyCode::Char('4') => Some(4),
                KeyCode::Char('5') => Some(5),
                KeyCode::Char('6') => Some(6),
                _ => None,
            };
            if let Some(level) = heading_level {
                self.apply_heading_toggle(level);
                return;
            }
        }

        // Ctrl+B: toggle bold
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
            self.apply_inline_format("**");
            return;
        }

        // Ctrl+I: toggle italic
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('i') {
            self.apply_inline_format("*");
            return;
        }

        // Ctrl+K: insert/wrap link
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('k') {
            self.apply_link_format();
            return;
        }

        // Ctrl+U: toggle strikethrough
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('u') {
            self.apply_inline_format("~~");
            return;
        }

        // Esc: exit edit mode in one press (also turns off Focus Mode)
        if key.code == KeyCode::Esc {
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

            // Always return to Browse mode
            self.mode = AppMode::Browse;
            self.show_browser = self.browser_visibility_before_focus;
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

    // ── Formatting helpers ──────────────────────────────────────

    /// Toggle a heading level on the current line.
    ///
    /// Reads the current line from the editor, applies `toggle_heading`,
    /// and replaces the line in-place.
    fn apply_heading_toggle(&mut self, level: usize) {
        use edtui::RowIndex;

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
    use crossterm::event::KeyEvent;
    use tempfile::TempDir;

    fn test_app(root: &TempDir) -> App {
        let mut app = App::new(Config::default(), root.path().to_path_buf());
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

        let app = App::new(config, tmp.path().to_path_buf());
        assert_eq!(app.editor_line_width, 64);
        assert_eq!(app.auto_save.delay, Duration::from_millis(1500));
        assert_eq!(app.focus_mode, FocusMode::Sentence);
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
        assert_eq!(app.prompt, BrowserPrompt::Create(String::new()));
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
        let mut app = App::new(Config::default(), empty);
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
}
