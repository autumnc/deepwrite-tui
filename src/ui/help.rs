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
 l / → / ⏎   Enter dir / Open    Ctrl+F       Cycle focus mode
 a           Create file/dir     Ctrl+O       Outline
 r           Rename              Ctrl+B       Bold
 d           Delete              Ctrl+I / T   Italic
 cc          Copy path           Ctrl+U       Strikethrough
 /           Search/filter       Ctrl+K       Insert link
 .           Toggle hidden       Ctrl+1..6    Heading level
 Ctrl+E      Toggle browser      F1..F6       Heading fallback
 q           Quit                Ctrl+Y       Redo
                                 Ctrl+A       Select all
                                 Ctrl+Z       Undo
                                 Ctrl+C       Copy
                                 Ctrl+X       Cut
                                 Ctrl+V       Paste

 Press ? or Esc to close";

/// Render the help overlay across the full screen.
pub fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.bg).fg(theme.fg));

    let paragraph = Paragraph::new(HELP_TEXT)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
