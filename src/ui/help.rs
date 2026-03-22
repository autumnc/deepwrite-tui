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
