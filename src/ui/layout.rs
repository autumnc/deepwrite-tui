use ratatui::prelude::*;

/// The computed rectangles for each panel in the application layout.
#[derive(Debug, Clone)]
pub struct PanelLayout {
    pub browser: Rect,
    pub editor: Rect,
    pub status_bar: Rect,
}

/// Compute the layout for the application given the total area.
///
/// - Status bar is always 1 line at the bottom.
/// - If `show_browser` is true, the browser panel gets `browser_width` columns
///   on the left and the editor takes the rest.
/// - If `show_browser` is false, the editor takes the full width and the browser
///   rect is `Rect::default()` (zero-sized).
pub fn compute_layout(area: Rect, browser_width: u16, show_browser: bool) -> PanelLayout {
    // Split vertically: main content area + status bar (1 line)
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let status_bar = vertical[1];

    if show_browser {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(browser_width), Constraint::Min(1)])
            .split(main_area);

        PanelLayout {
            browser: horizontal[0],
            editor: horizontal[1],
            status_bar,
        }
    } else {
        PanelLayout {
            browser: Rect::default(),
            editor: main_area,
            status_bar,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_with_browser() {
        let area = Rect::new(0, 0, 100, 40);
        let layout = compute_layout(area, 30, true);

        assert_eq!(layout.status_bar.height, 1);
        assert_eq!(layout.browser.width, 30);
        assert_eq!(layout.editor.width, 70);
        assert_eq!(layout.browser.height, 39);
        assert_eq!(layout.editor.height, 39);
    }

    #[test]
    fn test_layout_without_browser() {
        let area = Rect::new(0, 0, 100, 40);
        let layout = compute_layout(area, 30, false);

        assert_eq!(layout.status_bar.height, 1);
        assert_eq!(layout.browser, Rect::default());
        assert_eq!(layout.editor.width, 100);
        assert_eq!(layout.editor.height, 39);
    }
}
