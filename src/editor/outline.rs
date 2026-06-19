//! Outline panel: heading extraction and state management.

use regex::Regex;

/// A single heading extracted from the document.
#[derive(Debug, Clone)]
pub struct Heading {
    /// Heading level 1–6.
    pub level: usize,
    /// Heading text content (without the leading `#` markers and space).
    pub text: String,
    /// 0-based row index in the editor buffer.
    pub row: usize,
}

/// State for the outline panel.
#[derive(Debug, Clone, Default)]
pub struct OutlineState {
    /// All headings extracted from the current document.
    pub headings: Vec<Heading>,
    /// Index of the currently selected heading in the list.
    pub selected: usize,
    /// Whether the outline panel is visible.
    pub visible: bool,
    /// Whether the outline panel has keyboard focus.
    pub focused: bool,
}

impl OutlineState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Extract all headings from the editor content string.
pub fn extract_headings(content: &str) -> Vec<Heading> {
    let re = Regex::new(r"^(#{1,6})\s(.*)$").unwrap();
    let mut headings = Vec::new();

    for (row, line) in content.split('\n').enumerate() {
        if let Some(caps) = re.captures(line) {
            let hashes = caps.get(1).unwrap().as_str();
            let level = hashes.len().min(6);
            let text = caps.get(2).unwrap().as_str().trim().to_string();
            headings.push(Heading { level, text, row });
        }
    }

    headings
}

/// Find the index of the heading closest to (but not after) the cursor row.
pub fn current_heading_index(headings: &[Heading], cursor_row: usize) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, heading) in headings.iter().enumerate() {
        if heading.row <= cursor_row {
            best = Some(i);
        } else {
            break;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_headings_basic() {
        let content = "# Title\n\n## Section 1\nSome text\n### Subsection\n## Section 2";
        let headings = extract_headings(content);
        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].text, "Title");
        assert_eq!(headings[0].row, 0);
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].text, "Section 1");
        assert_eq!(headings[1].row, 2);
        assert_eq!(headings[2].level, 3);
        assert_eq!(headings[2].text, "Subsection");
        assert_eq!(headings[2].row, 4);
        assert_eq!(headings[3].level, 2);
        assert_eq!(headings[3].text, "Section 2");
        assert_eq!(headings[3].row, 5);
    }

    #[test]
    fn extract_headings_ignores_non_headings() {
        let content = "Not a heading\n#Real heading (no space)\n# Heading\n";
        let headings = extract_headings(content);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "Heading");
    }

    #[test]
    fn extract_headings_empty() {
        let headings = extract_headings("no headings here\njust text");
        assert!(headings.is_empty());
    }

    #[test]
    fn current_heading_finds_closest() {
        let content = "# Title\n\n## Section 1\nSome text\n### Subsection\n## Section 2";
        let headings = extract_headings(content);

        assert_eq!(current_heading_index(&headings, 0), Some(0));
        assert_eq!(current_heading_index(&headings, 1), Some(0));
        assert_eq!(current_heading_index(&headings, 2), Some(1));
        assert_eq!(current_heading_index(&headings, 3), Some(1));
        assert_eq!(current_heading_index(&headings, 4), Some(2));
        assert_eq!(current_heading_index(&headings, 5), Some(3));
    }

    #[test]
    fn current_heading_before_first() {
        let headings = vec![Heading {
            level: 1,
            text: "Late".into(),
            row: 5,
        }];
        assert_eq!(current_heading_index(&headings, 3), None);
    }
}
