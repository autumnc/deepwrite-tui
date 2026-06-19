use ratatui::prelude::*;

/// The computed rectangles for each panel in the application layout.
#[derive(Debug, Clone)]
pub struct PanelLayout {
    pub browser: Rect,
    pub editor: Rect,
    pub outline: Rect,
    pub status_bar: Rect,
}

/// Compute the layout for the application given the total area.
///
/// - Status bar is always 1 line at the bottom.
/// - If `show_browser` is true, the browser and editor panels are split
///   according to `ratio` (e.g. `[1, 3]` = browser 1/4, editor 3/4).
/// - If `show_outline` is true, the outline panel takes ~20% of the width
///   on the right side, and the editor gets the remaining space.
/// - If both browser and outline are shown, the layout is:
///   `[browser | editor | outline]`.
pub fn compute_layout(
    area: Rect,
    ratio: [u32; 2],
    show_browser: bool,
    show_outline: bool,
) -> PanelLayout {
    // Split vertically: main content area + status bar (1 line)
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let status_bar = vertical[1];

    match (show_browser, show_outline) {
        (true, true) => {
            let total = ratio[0] + ratio[1];
            let outline_width = (main_area.width as u32 / 5).max(20) as u16;
            let outline_width = outline_width.min(main_area.width / 3);
            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(ratio[0], total),
                    Constraint::Min(1),
                    Constraint::Length(outline_width),
                ])
                .split(main_area);

            PanelLayout {
                browser: horizontal[0],
                editor: horizontal[1],
                outline: horizontal[2],
                status_bar,
            }
        }
        (true, false) => {
            let total = ratio[0] + ratio[1];
            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(ratio[0], total),
                    Constraint::Ratio(ratio[1], total),
                ])
                .split(main_area);

            PanelLayout {
                browser: horizontal[0],
                editor: horizontal[1],
                outline: Rect::default(),
                status_bar,
            }
        }
        (false, true) => {
            let outline_width = (main_area.width as u32 / 5).max(20) as u16;
            let outline_width = outline_width.min(main_area.width / 3);
            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(outline_width)])
                .split(main_area);

            PanelLayout {
                browser: Rect::default(),
                editor: horizontal[0],
                outline: horizontal[1],
                status_bar,
            }
        }
        (false, false) => PanelLayout {
            browser: Rect::default(),
            editor: main_area,
            outline: Rect::default(),
            status_bar,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_with_browser_ratio() {
        let area = Rect::new(0, 0, 100, 40);
        let layout = compute_layout(area, [1, 3], true, false);

        assert_eq!(layout.status_bar.height, 1);
        assert_eq!(layout.browser.width, 25);
        assert_eq!(layout.editor.width, 75);
        assert_eq!(layout.browser.height, 39);
        assert_eq!(layout.editor.height, 39);
        assert_eq!(layout.outline, Rect::default());
    }

    #[test]
    fn test_layout_with_equal_ratio() {
        let area = Rect::new(0, 0, 100, 40);
        let layout = compute_layout(area, [1, 1], true, false);

        assert_eq!(layout.browser.width, 50);
        assert_eq!(layout.editor.width, 50);
    }

    #[test]
    fn test_layout_without_browser() {
        let area = Rect::new(0, 0, 100, 40);
        let layout = compute_layout(area, [1, 3], false, false);

        assert_eq!(layout.status_bar.height, 1);
        assert_eq!(layout.browser, Rect::default());
        assert_eq!(layout.editor.width, 100);
        assert_eq!(layout.editor.height, 39);
    }

    #[test]
    fn test_layout_with_outline_only() {
        let area = Rect::new(0, 0, 100, 40);
        let layout = compute_layout(area, [1, 3], false, true);

        assert_eq!(layout.status_bar.height, 1);
        assert_eq!(layout.browser, Rect::default());
        assert_eq!(layout.outline.width, 20); // 100/5 = 20
        assert_eq!(layout.editor.width, 80);
        assert_eq!(layout.editor.height, 39);
    }

    #[test]
    fn test_layout_with_browser_and_outline() {
        let area = Rect::new(0, 0, 100, 40);
        let layout = compute_layout(area, [1, 3], true, true);

        assert_eq!(layout.browser.width, 25);
        assert_eq!(layout.outline.width, 20);
        assert!(layout.editor.width > 0);
    }
}
