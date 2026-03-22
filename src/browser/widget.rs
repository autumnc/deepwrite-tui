use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::theme::Theme;

use super::entries::EntryKind;
use super::navigator::Navigator;

/// A prompt label and input buffer for rendering at the bottom of the browser.
pub struct BrowserPromptInfo<'a> {
    pub label: &'a str,
    pub input: &'a str,
}

/// Render the file browser panel into the given area.
///
/// If `prompt` is `Some`, an input line is rendered at the bottom of the panel.
/// If `visible_indices` is `Some`, only those entries are shown (search filter).
pub fn render_browser(frame: &mut Frame, area: Rect, nav: &Navigator, theme: &Theme) {
    render_browser_with_prompt(frame, area, nav, theme, None, None);
}

/// Full browser renderer with optional prompt overlay and search filter.
pub fn render_browser_with_prompt(
    frame: &mut Frame,
    area: Rect,
    nav: &Navigator,
    theme: &Theme,
    prompt: Option<BrowserPromptInfo>,
    visible_indices: Option<&[usize]>,
) {
    // Split area: if there is a prompt, reserve the bottom line for it.
    let (list_area, prompt_area) = if prompt.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // Build the directory title (last component of current_dir, or the full path if root)
    let dir_name = nav
        .current_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| nav.current_dir.display().to_string());

    let block = Block::default()
        .title(format!(" {} ", dir_name))
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme.dimmed_fg))
        .style(Style::default().bg(theme.bg).fg(theme.fg));

    // Determine which entries to show and which index maps to the selected one.
    let (items, selected_list_index): (Vec<ListItem>, Option<usize>) =
        if let Some(indices) = visible_indices {
            let items: Vec<ListItem> = indices
                .iter()
                .map(|&i| {
                    let entry = &nav.entries[i];
                    let display = match entry.kind {
                        EntryKind::Directory => format!("{}/", entry.name),
                        EntryKind::File => entry.name.clone(),
                    };
                    let style = match entry.kind {
                        EntryKind::Directory => Style::default().fg(theme.browser_dir),
                        EntryKind::File => Style::default().fg(theme.fg),
                    };
                    ListItem::new(display).style(style)
                })
                .collect();
            let sel = indices.iter().position(|&i| i == nav.selected);
            (items, sel)
        } else {
            let items: Vec<ListItem> = nav
                .entries
                .iter()
                .map(|entry| {
                    let display = match entry.kind {
                        EntryKind::Directory => format!("{}/", entry.name),
                        EntryKind::File => entry.name.clone(),
                    };
                    let style = match entry.kind {
                        EntryKind::Directory => Style::default().fg(theme.browser_dir),
                        EntryKind::File => Style::default().fg(theme.fg),
                    };
                    ListItem::new(display).style(style)
                })
                .collect();
            let sel = if nav.entries.is_empty() {
                None
            } else {
                Some(nav.selected)
            };
            (items, sel)
        };

    // If the directory is empty, show a helpful message instead of an empty list.
    if items.is_empty() {
        let empty_block = block;
        let msg = Paragraph::new("No Markdown files.\nPress n to create one.")
            .style(Style::default().fg(theme.dimmed_fg).bg(theme.bg))
            .block(empty_block);
        frame.render_widget(msg, list_area);
    } else {
        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(theme.browser_selected_bg)
                .fg(theme.browser_selected_fg),
        );

        let mut state = ListState::default();
        state.select(selected_list_index);

        frame.render_stateful_widget(list, list_area, &mut state);
    }

    // Render prompt line if present.
    if let (Some(prompt_info), Some(p_area)) = (prompt, prompt_area) {
        let text = format!("{}{}", prompt_info.label, prompt_info.input);
        let prompt_widget =
            Paragraph::new(text).style(Style::default().bg(theme.browser_selected_bg).fg(theme.fg));
        frame.render_widget(prompt_widget, p_area);
    }
}
