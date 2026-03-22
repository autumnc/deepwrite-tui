//! Status bar rendering.

use ratatui::{prelude::*, widgets::Paragraph};

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

    // Build the three sections
    let left = filename.to_string();
    let center = focus_label.to_string();
    let right = format!("{word_count}W  {char_count}C");

    let width = area.width as usize;

    // Calculate padding to distribute the three parts across the width
    let left_len = left.len();
    let center_len = center.len();
    let right_len = right.len();

    let line = if width < left_len + center_len + right_len + 4 {
        // Not enough room — just concatenate with spaces
        format!("{left}  {center}  {right}")
    } else if center.is_empty() {
        // No center label — left-justify filename, right-justify counts
        let padding = width.saturating_sub(left_len + right_len);
        format!("{left}{:>pad$}", right, pad = padding)
    } else {
        // Position center text in the middle of the bar
        let mid = width / 2;
        let center_start = mid.saturating_sub(center_len / 2);
        let left_pad = center_start.saturating_sub(left_len);
        let right_pad = width
            .saturating_sub(center_start + center_len)
            .saturating_sub(right_len);
        format!(
            "{left}{}{center}{}{}",
            " ".repeat(left_pad),
            " ".repeat(right_pad),
            right,
        )
    };

    // Truncate to width
    let display: String = line.chars().take(width).collect();

    let para = Paragraph::new(display).style(style);
    frame.render_widget(para, area);
}
