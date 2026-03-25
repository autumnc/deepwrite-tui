//! Focus mode: state machine, paragraph detection, sentence detection, and dimming support.

use super::sentence::{byte_range_to_rows, cursor_to_byte_offset, find_sentence_at};

/// The focus mode determines which distraction-free features are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusMode {
    Off,
    Sentence,
    Paragraph,
    Typewriter,
}

impl FocusMode {
    /// Parse a focus mode from config, defaulting to Off for unknown values.
    pub fn from_config(value: &str) -> Self {
        match value {
            "sentence" => Self::Sentence,
            "paragraph" => Self::Paragraph,
            "typewriter" => Self::Typewriter,
            _ => Self::Off,
        }
    }

    /// Cycle to the next focus mode: Off -> Sentence -> Paragraph -> Typewriter -> Off.
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Sentence,
            Self::Sentence => Self::Paragraph,
            Self::Paragraph => Self::Typewriter,
            Self::Typewriter => Self::Off,
        }
    }

    /// Whether this mode dims text outside the active range.
    pub fn has_dimming(self) -> bool {
        matches!(self, Self::Sentence | Self::Paragraph)
    }

    /// Whether this mode uses typewriter scrolling (cursor always centered).
    pub fn has_typewriter(self) -> bool {
        matches!(self, Self::Typewriter)
    }

    /// A human-readable label for the status bar. Empty string for Off.
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "",
            Self::Sentence => "Focus: Sentence",
            Self::Paragraph => "Focus: Paragraph",
            Self::Typewriter => "Focus: Typewriter",
        }
    }
}

/// A range of rows that should be highlighted (not dimmed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusRange {
    pub start_row: usize,
    pub end_row: usize,
}

/// Returns the indentation width if the line starts with a supported Markdown
/// list marker, otherwise `None`.
fn list_marker_indent(line: &str) -> Option<usize> {
    let indent = line.chars().take_while(|c| *c == ' ').count();

    // Avoid treating 4-space indented code blocks as lists.
    if indent > 3 {
        return None;
    }

    let trimmed = &line[indent..];
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return Some(indent);
    }

    let digits = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && trimmed[digits..].starts_with(". ") {
        return Some(indent);
    }

    None
}

/// Returns true if the given row is inside a fenced code block.
fn is_inside_fenced_code(lines: &[&str], row: usize) -> bool {
    let mut inside = false;
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("```") {
            inside = !inside;
        }
        if i == row {
            return inside;
        }
    }
    inside
}

/// Find the scope for sentence-mode focus at the given cursor row.
///
/// For list entries, returns only the single list item block (marker + continuation
/// lines). For fenced code blocks, returns just the cursor row. For normal prose,
/// falls back to `find_paragraph_at_cursor`.
fn find_sentence_scope_at_cursor(text: &str, cursor_row: usize) -> Option<FocusRange> {
    let lines: Vec<&str> = text.split('\n').collect();

    if cursor_row >= lines.len() {
        return None;
    }

    if lines[cursor_row].trim().is_empty() {
        return None;
    }

    // Inside fenced code: each line is its own unit.
    if is_inside_fenced_code(&lines, cursor_row) {
        return Some(FocusRange {
            start_row: cursor_row,
            end_row: cursor_row,
        });
    }

    // Try to find the list marker anchor for the cursor row.
    let anchor = if list_marker_indent(&lines[cursor_row]).is_some() {
        // Cursor is directly on a list marker line.
        Some(cursor_row)
    } else {
        // Scan upward within the same contiguous non-blank block to find the
        // nearest preceding list marker row.
        let mut row = cursor_row;
        let mut found = None;
        while row > 0 {
            row -= 1;
            if lines[row].trim().is_empty() {
                break;
            }
            if is_inside_fenced_code(&lines, row) {
                break;
            }
            if list_marker_indent(&lines[row]).is_some() {
                found = Some(row);
                break;
            }
        }
        found
    };

    match anchor {
        Some(anchor_row) => {
            // Scan downward from the anchor, including continuation lines.
            let mut end_row = anchor_row;
            for r in (anchor_row + 1)..lines.len() {
                if lines[r].trim().is_empty() {
                    break;
                }
                if list_marker_indent(&lines[r]).is_some() {
                    // New sibling list marker — stop.
                    break;
                }
                end_row = r;
            }
            Some(FocusRange {
                start_row: anchor_row,
                end_row,
            })
        }
        None => {
            // No list marker found — fall back to paragraph detection.
            find_paragraph_at_cursor(text, cursor_row)
        }
    }
}

/// Find the sentence at the cursor position and return the row range that
/// should remain bright (not dimmed).
///
/// Strategy: first find the scope containing the cursor (list entry or
/// paragraph), then do sentence detection only within that scope. This
/// prevents headings, list items, and other non-sentence Markdown elements
/// from being merged into one giant "sentence" spanning multiple paragraphs.
pub fn find_sentence_at_cursor(text: &str, cursor_row: usize, cursor_col: usize) -> FocusRange {
    // Step 1: Find the scope containing the cursor (list entry or paragraph)
    let para = match find_sentence_scope_at_cursor(text, cursor_row) {
        Some(p) => p,
        None => {
            // Cursor on a blank line — just highlight that single line
            return FocusRange {
                start_row: cursor_row,
                end_row: cursor_row,
            };
        }
    };

    // Step 2: Extract the paragraph text so manually wrapped prose still
    // counts as one sentence span.
    let lines: Vec<&str> = text.split('\n').collect();
    let para_text: String = lines[para.start_row..=para.end_row].join("\n");

    // Step 3: Calculate cursor position relative to paragraph start.
    let cursor_row_in_para = cursor_row - para.start_row;
    let byte_offset_in_para = cursor_to_byte_offset(&para_text, cursor_row_in_para, cursor_col);

    // Step 4: Find sentence within the paragraph text.
    let sentence = find_sentence_at(&para_text, byte_offset_in_para);

    // Step 5: Convert byte range back to rows (relative to paragraph).
    let (sent_start_row_rel, sent_end_row_rel) =
        byte_range_to_rows(&para_text, sentence.start, sentence.end);

    // Step 6: Convert back to absolute row indices.
    FocusRange {
        start_row: para.start_row + sent_start_row_rel,
        end_row: para.start_row + sent_end_row_rel,
    }
}

/// Find the paragraph containing the given cursor row using blank-line detection.
///
/// A paragraph is a group of consecutive non-empty lines separated by blank lines.
/// Returns the start_row and end_row of the paragraph containing the cursor,
/// or `None` if the cursor is on a blank line.
pub fn find_paragraph_at_cursor(text: &str, cursor_row: usize) -> Option<FocusRange> {
    let lines: Vec<&str> = text.split('\n').collect();

    // If cursor is beyond the text, return None
    if cursor_row >= lines.len() {
        return None;
    }

    // If the cursor is on a blank line, no paragraph
    if lines[cursor_row].trim().is_empty() {
        return None;
    }

    // Search upward for the start of the paragraph
    let mut start_row = cursor_row;
    while start_row > 0 && !lines[start_row - 1].trim().is_empty() {
        start_row -= 1;
    }

    // Search downward for the end of the paragraph
    let mut end_row = cursor_row;
    while end_row + 1 < lines.len() && !lines[end_row + 1].trim().is_empty() {
        end_row += 1;
    }

    Some(FocusRange { start_row, end_row })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_mode_cycle() {
        assert_eq!(FocusMode::Off.cycle(), FocusMode::Sentence);
        assert_eq!(FocusMode::Sentence.cycle(), FocusMode::Paragraph);
        assert_eq!(FocusMode::Paragraph.cycle(), FocusMode::Typewriter);
        assert_eq!(FocusMode::Typewriter.cycle(), FocusMode::Off);
    }

    #[test]
    fn focus_mode_dimming() {
        assert!(!FocusMode::Off.has_dimming());
        assert!(FocusMode::Sentence.has_dimming());
        assert!(FocusMode::Paragraph.has_dimming());
        assert!(!FocusMode::Typewriter.has_dimming());
    }

    #[test]
    fn focus_mode_typewriter() {
        assert!(!FocusMode::Off.has_typewriter());
        assert!(!FocusMode::Sentence.has_typewriter());
        assert!(!FocusMode::Paragraph.has_typewriter());
        assert!(FocusMode::Typewriter.has_typewriter());
    }

    #[test]
    fn focus_mode_labels() {
        assert_eq!(FocusMode::Off.label(), "");
        assert_eq!(FocusMode::Sentence.label(), "Focus: Sentence");
        assert_eq!(FocusMode::Paragraph.label(), "Focus: Paragraph");
        assert_eq!(FocusMode::Typewriter.label(), "Focus: Typewriter");
    }

    #[test]
    fn focus_mode_from_config() {
        assert_eq!(FocusMode::from_config("sentence"), FocusMode::Sentence);
        assert_eq!(FocusMode::from_config("paragraph"), FocusMode::Paragraph);
        assert_eq!(FocusMode::from_config("typewriter"), FocusMode::Typewriter);
        assert_eq!(FocusMode::from_config("unknown"), FocusMode::Off);
    }

    #[test]
    fn find_paragraph_basic() {
        let source = "# Heading\n\nFirst paragraph.\n\nSecond paragraph.\n";

        // Row 0 is the heading — it is a non-blank line, so it forms its own paragraph
        let result = find_paragraph_at_cursor(source, 0);
        assert!(
            result.is_some(),
            "Heading line is a non-blank line paragraph"
        );
        let range = result.unwrap();
        assert_eq!(range.start_row, 0);
        assert_eq!(range.end_row, 0);

        // Row 1 is blank
        let result = find_paragraph_at_cursor(source, 1);
        assert!(result.is_none(), "Blank line should not be a paragraph");

        // Row 2 is the first paragraph
        let result = find_paragraph_at_cursor(source, 2);
        assert!(result.is_some(), "Expected paragraph at row 2");
        let range = result.unwrap();
        assert_eq!(range.start_row, 2);
        assert_eq!(range.end_row, 2);

        // Row 4 is the second paragraph
        let result = find_paragraph_at_cursor(source, 4);
        assert!(result.is_some(), "Expected paragraph at row 4");
        let range = result.unwrap();
        assert_eq!(range.start_row, 4);
        assert_eq!(range.end_row, 4);
    }

    #[test]
    fn find_paragraph_multiline() {
        let source = "Line one\nLine two\nLine three\n\nAnother paragraph.\n";

        // Row 0 should find paragraph spanning rows 0-2
        let result = find_paragraph_at_cursor(source, 0);
        assert!(result.is_some());
        let range = result.unwrap();
        assert_eq!(range.start_row, 0);
        assert_eq!(range.end_row, 2);

        // Row 1 should find the same paragraph
        let result = find_paragraph_at_cursor(source, 1);
        assert!(result.is_some());
        let range = result.unwrap();
        assert_eq!(range.start_row, 0);
        assert_eq!(range.end_row, 2);

        // Row 4 is the second paragraph
        let result = find_paragraph_at_cursor(source, 4);
        assert!(result.is_some());
        let range = result.unwrap();
        assert_eq!(range.start_row, 4);
        assert_eq!(range.end_row, 4);
    }

    #[test]
    fn find_paragraph_blank_line_returns_none() {
        let source = "Hello\n\nWorld\n";
        let result = find_paragraph_at_cursor(source, 1);
        assert!(result.is_none());
    }

    #[test]
    fn find_sentence_at_cursor_stays_within_paragraph() {
        let source = "# Heading\n\nFirst sentence. Second sentence.\n";
        let focus = find_sentence_at_cursor(source, 2, 2);
        assert_eq!(focus.start_row, 2);
        assert_eq!(focus.end_row, 2);
    }

    #[test]
    fn find_sentence_at_cursor_keeps_multiline_sentence_together() {
        let source =
            "This is one sentence split\nacross two lines without a blank break.\n\nNext paragraph.\n";

        let focus = find_sentence_at_cursor(source, 0, 5);
        assert_eq!(focus.start_row, 0);
        assert_eq!(focus.end_row, 1);

        let focus = find_sentence_at_cursor(source, 1, 5);
        assert_eq!(focus.start_row, 0);
        assert_eq!(focus.end_row, 1);
    }

    #[test]
    fn find_sentence_at_cursor_highlights_single_bullet_item() {
        let source = "### Bug Fixes\n\n- Fix alpha\n- Fix beta\n- Fix gamma\n";

        let focus = find_sentence_at_cursor(source, 3, 4);
        assert_eq!(focus.start_row, 3);
        assert_eq!(focus.end_row, 3);
    }

    #[test]
    fn find_sentence_at_cursor_highlights_single_numbered_item() {
        let source = "Steps:\n\n1. First step\n2. Second step\n3. Third step\n";

        let focus = find_sentence_at_cursor(source, 3, 4);
        assert_eq!(focus.start_row, 3);
        assert_eq!(focus.end_row, 3);
    }

    #[test]
    fn find_sentence_at_cursor_keeps_list_continuation_lines_together() {
        let source = "- This is a long item that\n  continues on the next line\n- Second item\n";

        let focus = find_sentence_at_cursor(source, 1, 5);
        assert_eq!(focus.start_row, 0);
        assert_eq!(focus.end_row, 1);
    }

    #[test]
    fn find_paragraph_still_treats_a_list_as_one_blank_line_delimited_paragraph() {
        let source = "- Fix alpha\n- Fix beta\n- Fix gamma\n";

        let result = find_paragraph_at_cursor(source, 1).unwrap();
        assert_eq!(result.start_row, 0);
        assert_eq!(result.end_row, 2);
    }

    #[test]
    fn find_sentence_at_cursor_does_not_treat_fenced_code_as_a_list() {
        let source = "```md\n- not a list item for focus\n- still code\n```\n";

        let focus = find_sentence_at_cursor(source, 1, 3);
        assert_eq!(focus.start_row, 1);
        assert_eq!(focus.end_row, 1);
    }
}
