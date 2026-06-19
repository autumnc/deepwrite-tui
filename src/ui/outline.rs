//! Outline panel rendering.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::editor::outline::OutlineState;
use crate::theme::Theme;

/// Render the outline panel into the given area.
pub fn render_outline(
    frame: &mut Frame,
    area: Rect,
    outline: &OutlineState,
    current_idx: Option<usize>,
    theme: &Theme,
) {
    let border_style = if outline.focused {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.dimmed_fg)
    };

    let block = Block::default()
        .title(" Outline ")
        .borders(Borders::LEFT)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg).fg(theme.fg));

    if outline.headings.is_empty() {
        let msg = Paragraph::new("No headings")
            .style(Style::default().fg(theme.dimmed_fg).bg(theme.bg))
            .block(block);
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = outline
        .headings
        .iter()
        .enumerate()
        .map(|(i, heading)| {
            let indent = "  ".repeat(heading.level.saturating_sub(1));
            let display = format!("{indent}{}", heading.text);
            let is_current = current_idx == Some(i);

            let style = if is_current {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                Style::default().fg(theme.heading_color(heading.level))
            };

            ListItem::new(Line::from(Span::styled(display, style)))
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(theme.browser_selected_bg)
            .fg(theme.browser_selected_fg),
    );

    let mut state = ListState::default();
    state.select(Some(outline.selected));

    frame.render_stateful_widget(list, area, &mut state);
}
