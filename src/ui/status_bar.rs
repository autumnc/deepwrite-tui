//! Status bar rendering.

use ratatui::{prelude::*, widgets::Paragraph};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

/// Render the status bar into the given area.
///
/// - Left: filename (or mode hint)
/// - Center: focus mode label (if active)
/// - Right: word count + char count
pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    filename: &str,
    word_count: usize,
    char_count: usize,
    focus_label: &str,
    theme: &Theme,
) {
    let style = Style::default()
        .fg(theme.status_bar_fg)
        .bg(theme.status_bar_bg);

    let left = filename.to_string();
    let center = focus_label.to_string();
    let right = format!("{word_count}W  {char_count}C");
    let width = area.width as usize;
    if width == 0 {
        return;
    }

    let left = truncate_to_width(&left, width);
    let center = truncate_to_width(&center, width);
    let right = truncate_to_width(&right, width);

    frame.render_widget(Paragraph::new(" ".repeat(width)).style(style), area);

    let right_width = display_width(&right).min(width) as u16;
    let right_area = Rect::new(
        area.x + area.width.saturating_sub(right_width),
        area.y,
        right_width,
        area.height,
    );
    if !right.is_empty() && right_area.width > 0 {
        frame.render_widget(
            Paragraph::new(right)
                .style(style)
                .alignment(Alignment::Right),
            right_area,
        );
    }

    let remaining_width = area.width.saturating_sub(right_width);
    if remaining_width == 0 {
        return;
    }

    if center.is_empty() {
        let left_area = Rect::new(area.x, area.y, remaining_width, area.height);
        frame.render_widget(Paragraph::new(left).style(style), left_area);
        return;
    }

    let center_width = display_width(&center).min(remaining_width as usize) as u16;
    let center_x = area.x + remaining_width.saturating_sub(center_width) / 2;
    let left_area = Rect::new(area.x, area.y, center_x.saturating_sub(area.x), area.height);
    let center_area = Rect::new(center_x, area.y, center_width, area.height);

    if left_area.width > 0 {
        frame.render_widget(Paragraph::new(left).style(style), left_area);
    }
    if center_area.width > 0 {
        frame.render_widget(
            Paragraph::new(center)
                .style(style)
                .alignment(Alignment::Center),
            center_area,
        );
    }
}

fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    let mut truncated = String::new();
    let mut current_width = 0;

    for grapheme in text.graphemes(true) {
        let grapheme_width = display_width(grapheme);
        if current_width + grapheme_width > max_width {
            break;
        }
        truncated.push_str(grapheme);
        current_width += grapheme_width;
    }

    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_to_width_respects_wide_characters() {
        assert_eq!(truncate_to_width("檔案.md", 4), "檔案");
        assert_eq!(display_width("檔案"), 4);
    }

    #[test]
    fn truncate_to_width_keeps_full_graphemes() {
        assert_eq!(truncate_to_width("a👨‍👩‍👧‍👦b", 3), "a👨‍👩‍👧‍👦");
    }
}
