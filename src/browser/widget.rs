use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::theme::Theme;

use super::entries::EntryKind;
use super::navigator::Navigator;

pub const BROWSER_SCROLL_OFF: usize = 5;

/// A prompt label and input buffer for rendering at the bottom of the browser.
pub struct BrowserPromptInfo<'a> {
    pub label: &'a str,
    pub input: &'a str,
}

/// Split the browser panel into the list area and optional prompt area.
pub fn split_browser_area(area: Rect, has_prompt: bool) -> (Rect, Option<Rect>) {
    if has_prompt {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    }
}

/// Return the inner list content area, excluding the title row and right border.
pub fn browser_content_area(list_area: Rect) -> Rect {
    Rect::new(
        list_area.x,
        list_area.y.saturating_add(1),
        list_area.width.saturating_sub(1),
        list_area.height.saturating_sub(1),
    )
}

/// Compute the list offset needed to keep the selection within the scrolloff window.
pub fn browser_scroll_offset(list_area: Rect, selected_list_index: Option<usize>) -> usize {
    let visible_height = browser_content_area(list_area).height as usize;
    if visible_height == 0 {
        return 0;
    }

    let Some(sel) = selected_list_index else {
        return 0;
    };

    let scrolloff = BROWSER_SCROLL_OFF.min(visible_height / 2);
    let mut offset = 0;

    if sel < offset + scrolloff {
        offset = sel.saturating_sub(scrolloff);
    } else if sel + scrolloff >= offset + visible_height {
        offset = (sel + scrolloff + 1).saturating_sub(visible_height);
    }

    offset
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
    let (list_area, prompt_area) = split_browser_area(area, prompt.is_some());

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
        let msg = Paragraph::new("No Markdown files.\nPress a to create one.")
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
        *state.offset_mut() = browser_scroll_offset(list_area, selected_list_index);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_content_area_excludes_title_row_and_border() {
        let list_area = Rect::new(2, 3, 20, 8);
        assert_eq!(browser_content_area(list_area), Rect::new(2, 4, 19, 7));
    }

    #[test]
    fn browser_scroll_offset_respects_scrolloff_window() {
        let list_area = Rect::new(0, 0, 20, 11);
        assert_eq!(browser_scroll_offset(list_area, Some(10)), 6);
    }
}
