//! Editor integration: wraps edtui with a non-modal keymap and
//! centered content rendering.

pub mod focus;
pub mod formatting;
pub mod keymap;
pub mod markdown;
pub mod sentence;
pub mod word_count;

use crossterm::event::Event;
use edtui::{
    EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Highlight, Index2, Lines,
};
use ratatui::prelude::*;

use crate::theme::Theme;

use self::focus::{find_paragraph_at_cursor, find_sentence_at_cursor, FocusMode, FocusRange};
use self::keymap::deepwrite_keymap;
use self::markdown::MarkdownHighlighter;

/// Wraps edtui state and event handler into a single struct that the
/// rest of the app can interact with.
pub struct EditorWrapper {
    pub state: EditorState,
    pub handler: EditorEventHandler,
    highlighter: MarkdownHighlighter,
}

impl Default for EditorWrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorWrapper {
    /// Create a new editor with our custom non-modal keymap.
    ///
    /// The editor starts in Insert mode so the user can type immediately.
    pub fn new() -> Self {
        let key_handler = deepwrite_keymap();
        let handler = EditorEventHandler::new(key_handler);

        let mut state = EditorState::default();
        // Force Insert mode so the user can type right away.
        state.mode = EditorMode::Insert;

        Self {
            state,
            handler,
            highlighter: MarkdownHighlighter::new(),
        }
    }

    /// Load text content into the editor, replacing whatever was there.
    pub fn load_content(&mut self, content: &str) {
        self.state = EditorState::new(Lines::from(content));
        // Re-enter Insert mode after loading new content.
        self.state.mode = EditorMode::Insert;
    }

    /// Extract the current text as a `String`.
    pub fn get_content(&self) -> String {
        self.state
            .lines
            .iter_row()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Re-parse the editor content and apply syntax highlights.
    ///
    /// When `focus_mode` has dimming enabled (Sentence or Paragraph), text
    /// outside the active range is dimmed using `theme.dimmed_fg`.
    pub fn update_highlights(&mut self, theme: &Theme, focus_mode: FocusMode) {
        let content = self.get_content();
        let ranges = self.highlighter.parse(&content, theme);
        let total_rows = self.state.lines.len();

        self.state.clear_highlights();

        // Determine the active (non-dimmed) focus range, if applicable.
        let active_range: Option<FocusRange> = if focus_mode == FocusMode::Paragraph {
            let cursor_row = self.state.cursor.row;
            find_paragraph_at_cursor(&content, cursor_row)
        } else if focus_mode == FocusMode::Sentence {
            Some(find_sentence_at_cursor(
                &content,
                self.state.cursor.row,
                self.state.cursor.col,
            ))
        } else if focus_mode == FocusMode::Line {
            let row = self.state.cursor.row;
            Some(FocusRange {
                start_row: row,
                end_row: row,
            })
        } else {
            None
        };

        if let Some(focus) = active_range {
            // edtui uses first-match for highlights, so we must NOT use a
            // full-document dim + bright override approach. Instead, add dim
            // highlights only for regions OUTSIDE the active range.

            let dimmed_style = Style::default().fg(theme.dimmed_fg);

            // Dim region BEFORE the active range
            if focus.start_row > 0 {
                self.state.add_highlight(Highlight::new(
                    Index2::new(0, 0),
                    Index2::new(focus.start_row - 1, usize::MAX),
                    dimmed_style,
                ));
            }

            // Dim region AFTER the active range
            if focus.end_row + 1 < total_rows {
                self.state.add_highlight(Highlight::new(
                    Index2::new(focus.end_row + 1, 0),
                    Index2::new(total_rows.saturating_sub(1), usize::MAX),
                    dimmed_style,
                ));
            }

            // Add syntax highlights ONLY within the active range
            for r in &ranges {
                // Convert exclusive end to edtui inclusive end
                let (end_row, end_col) = if r.end_col > 0 {
                    (r.end_row, r.end_col - 1)
                } else if r.end_row > 0 {
                    (r.end_row - 1, usize::MAX)
                } else {
                    continue;
                };

                if r.start_row > end_row || (r.start_row == end_row && r.start_col > end_col) {
                    continue;
                }

                // Only add syntax highlights that overlap the active range
                if end_row < focus.start_row || r.start_row > focus.end_row {
                    continue;
                }

                self.state.add_highlight(Highlight::new(
                    Index2::new(r.start_row, r.start_col),
                    Index2::new(end_row, end_col),
                    r.style,
                ));
            }
        } else {
            // No focus dimming — apply all syntax highlights normally
            for r in &ranges {
                let (end_row, end_col) = if r.end_col > 0 {
                    (r.end_row, r.end_col - 1)
                } else if r.end_row > 0 {
                    (r.end_row - 1, usize::MAX)
                } else {
                    continue;
                };

                if r.start_row > end_row || (r.start_row == end_row && r.start_col > end_col) {
                    continue;
                }

                self.state.add_highlight(Highlight::new(
                    Index2::new(r.start_row, r.start_col),
                    Index2::new(end_row, end_col),
                    r.style,
                ));
            }
        }
    }

    /// Render the editor into the given area using our theme colors.
    ///
    /// When `focus_mode` is Typewriter, the viewport is adjusted so the
    /// cursor row is vertically centered in the view.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, focus_mode: FocusMode) {
        // Typewriter mode: center the cursor row vertically by adjusting
        // the viewport offset before edtui's own render logic runs.
        if focus_mode.has_typewriter() && area.height > 0 {
            let cursor_row = self.state.cursor.row;
            let half_height = (area.height as usize) / 2;
            let target_y = cursor_row.saturating_sub(half_height);
            let (offset_x, _) = self.state.viewport_offset();
            self.state.set_viewport_offset(offset_x, target_y);
        }

        let editor_theme = EditorTheme::default()
            .base(Style::default().bg(theme.bg).fg(theme.fg))
            .cursor_style(Style::default().bg(theme.accent).fg(theme.bg))
            .selection_style(Style::default().bg(theme.accent).fg(theme.bg))
            .hide_status_line();

        let view = EditorView::new(&mut self.state)
            .theme(editor_theme)
            .wrap(true);

        frame.render_widget(view, area);

        // Position the terminal cursor at the editor's cursor so that the
        // user sees a blinking caret in the right place.
        if let Some(pos) = self.state.cursor_screen_position() {
            frame.set_cursor_position(pos);
        }
    }

    /// Pass a crossterm event through to edtui's event handler.
    pub fn handle_event(&mut self, event: Event) {
        self.handler.on_event(event, &mut self.state);
    }
}

// ── Content centering helper ────────────────────────────────────────

/// Given the editor panel's `Rect` and a desired line width, return a
/// centered `Rect` that fits within the panel. If the panel is narrower
/// than `line_width`, the full panel width is used.
pub fn centered_editor_area(panel: Rect, line_width: u16) -> Rect {
    if panel.width <= line_width {
        panel
    } else {
        let padding = (panel.width - line_width) / 2;
        Rect::new(panel.x + padding, panel.y, line_width, panel.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn sentence_focus_does_not_keep_heading_bright() {
        let mut editor = EditorWrapper::new();
        editor.load_content("# Heading\n\nFirst sentence. Second sentence.");
        editor.state.cursor.row = 2;
        editor.state.cursor.col = 2;

        let theme = Theme::dark();
        editor.update_highlights(&theme, FocusMode::Sentence);

        let dimmed = Style::default().fg(theme.dimmed_fg);
        assert!(
            editor
                .state
                .highlights
                .iter()
                .any(|highlight| highlight.style == dimmed && highlight.contains_row(0)),
            "expected heading row to be dimmed in sentence focus mode"
        );
    }
}
