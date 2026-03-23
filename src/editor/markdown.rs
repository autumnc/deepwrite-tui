//! Markdown syntax highlighting using regex patterns.

use ratatui::style::{Modifier, Style};
use regex::Regex;

use crate::theme::Theme;

/// A highlighted range in the editor with a style.
#[derive(Debug, Clone)]
pub struct HighlightRange {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
    pub style: Style,
}

/// Markdown syntax highlighter powered by regex patterns.
pub struct MarkdownHighlighter {
    // Inline patterns
    re_bold: Regex,
    re_italic: Regex,
    re_inline_code: Regex,
    re_image: Regex,
    re_link: Regex,
    re_strikethrough: Regex,
    // Line-level patterns
    re_heading: Regex,
    re_block_quote: Regex,
    re_unordered_list: Regex,
    re_ordered_list: Regex,
    re_task_list: Regex,
    re_horizontal_rule: Regex,
    re_fenced_code_start: Regex,
}

impl Default for MarkdownHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownHighlighter {
    /// Create a new highlighter.
    pub fn new() -> Self {
        Self {
            re_bold: Regex::new(r"\*\*(.+?)\*\*").unwrap(),
            re_italic: Regex::new(r"\*([^*]+)\*").unwrap(),
            re_inline_code: Regex::new(r"`([^`]+)`").unwrap(),
            re_image: Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap(),
            re_link: Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap(),
            re_strikethrough: Regex::new(r"~~(.+?)~~").unwrap(),
            re_heading: Regex::new(r"^(#{1,6})\s").unwrap(),
            re_block_quote: Regex::new(r"^>\s?").unwrap(),
            re_unordered_list: Regex::new(r"^(\s*)([-*+])\s").unwrap(),
            re_ordered_list: Regex::new(r"^(\s*)(\d+[.)]) ").unwrap(),
            re_task_list: Regex::new(r"^(\s*)- \[([ xX])\] ").unwrap(),
            re_horizontal_rule: Regex::new(r"^(---+|\*\*\*+|___+)\s*$").unwrap(),
            re_fenced_code_start: Regex::new(r"^```").unwrap(),
        }
    }

    /// Parse markdown source text and return highlight ranges.
    pub fn parse(&mut self, source: &str, theme: &Theme) -> Vec<HighlightRange> {
        let mut ranges = Vec::new();
        let lines: Vec<&str> = source.split('\n').collect();
        let mut in_code_block = false;

        for (row, line) in lines.iter().enumerate() {
            let line_char_len = char_col(line, line.len());

            // Track fenced code blocks
            if self.re_fenced_code_start.is_match(line) {
                // The fence line itself is styled as code
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: 0,
                    end_row: row,
                    end_col: line_char_len,
                    style: Style::default().fg(theme.md_code),
                });
                in_code_block = !in_code_block;
                continue;
            }

            if in_code_block {
                // Everything inside a fenced code block is code-styled
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: 0,
                    end_row: row,
                    end_col: line_char_len,
                    style: Style::default().fg(theme.md_code),
                });
                continue;
            }

            // Horizontal rule (full line)
            if self.re_horizontal_rule.is_match(line) {
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: 0,
                    end_row: row,
                    end_col: line_char_len,
                    style: Style::default().fg(theme.md_muted),
                });
                continue;
            }

            // Heading (full line)
            if self.re_heading.is_match(line) {
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: 0,
                    end_row: row,
                    end_col: line_char_len,
                    style: Style::default()
                        .fg(theme.md_heading)
                        .add_modifier(Modifier::BOLD),
                });
                continue;
            }

            // Block quote marker
            if let Some(m) = self.re_block_quote.find(line) {
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: 0,
                    end_row: row,
                    end_col: m.end(),
                    style: Style::default().fg(theme.md_muted),
                });
                // Continue to process inline patterns in the rest of the line
            }

            // Task list (must check before unordered list since it's more specific)
            if let Some(m) = self.re_task_list.find(line) {
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: 0,
                    end_row: row,
                    end_col: m.end(),
                    style: Style::default().fg(theme.md_muted),
                });
            } else if let Some(m) = self.re_unordered_list.find(line) {
                // Unordered list marker
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: 0,
                    end_row: row,
                    end_col: m.end(),
                    style: Style::default().fg(theme.md_muted),
                });
            } else if let Some(m) = self.re_ordered_list.find(line) {
                // Ordered list marker
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: 0,
                    end_row: row,
                    end_col: m.end(),
                    style: Style::default().fg(theme.md_muted),
                });
            }

            // Inline patterns
            // Bold: **text** — collect positions for overlap check with italic
            let mut bold_ranges: Vec<(usize, usize)> = Vec::new();
            for m in self.re_bold.find_iter(line) {
                bold_ranges.push((m.start(), m.end()));
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: char_col(line, m.start()),
                    end_row: row,
                    end_col: char_col(line, m.end()),
                    style: Style::default().add_modifier(Modifier::BOLD),
                });
            }

            // Italic: *text* — search in segments of the line not covered by bold
            // Build a list of "safe" byte ranges to search for italic
            {
                let mut search_start = 0usize;
                let mut segments: Vec<(usize, usize)> = Vec::new();
                let mut sorted_bolds = bold_ranges.clone();
                sorted_bolds.sort_by_key(|&(s, _)| s);
                for &(bs, be) in &sorted_bolds {
                    if search_start < bs {
                        segments.push((search_start, bs));
                    }
                    search_start = be;
                }
                if search_start < line.len() {
                    segments.push((search_start, line.len()));
                }

                for (seg_start, seg_end) in segments {
                    let segment = &line[seg_start..seg_end];
                    for m in self.re_italic.find_iter(segment) {
                        ranges.push(HighlightRange {
                            start_row: row,
                            start_col: char_col(line, seg_start + m.start()),
                            end_row: row,
                            end_col: char_col(line, seg_start + m.end()),
                            style: Style::default().add_modifier(Modifier::ITALIC),
                        });
                    }
                }
            }

            // Images: ![alt](url) — collect positions to avoid double-matching as links
            let mut image_ranges: Vec<(usize, usize)> = Vec::new();
            for m in self.re_image.find_iter(line) {
                image_ranges.push((m.start(), m.end()));
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: char_col(line, m.start()),
                    end_row: row,
                    end_col: char_col(line, m.end()),
                    style: Style::default()
                        .fg(theme.md_link)
                        .add_modifier(Modifier::UNDERLINED),
                });
            }

            // Links: [text](url) — skip matches that overlap with image matches
            for m in self.re_link.find_iter(line) {
                let overlaps_image = image_ranges
                    .iter()
                    .any(|&(is, ie)| m.start() < ie && m.end() > is);
                if !overlaps_image {
                    ranges.push(HighlightRange {
                        start_row: row,
                        start_col: char_col(line, m.start()),
                        end_row: row,
                        end_col: char_col(line, m.end()),
                        style: Style::default()
                            .fg(theme.md_link)
                            .add_modifier(Modifier::UNDERLINED),
                    });
                }
            }

            // Inline code: `code`
            for m in self.re_inline_code.find_iter(line) {
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: char_col(line, m.start()),
                    end_row: row,
                    end_col: char_col(line, m.end()),
                    style: Style::default().fg(theme.md_code),
                });
            }

            // Strikethrough: ~~text~~
            for m in self.re_strikethrough.find_iter(line) {
                ranges.push(HighlightRange {
                    start_row: row,
                    start_col: char_col(line, m.start()),
                    end_row: row,
                    end_col: char_col(line, m.end()),
                    style: Style::default()
                        .fg(theme.md_muted)
                        .add_modifier(Modifier::CROSSED_OUT),
                });
            }
        }

        ranges
    }
}

fn char_col(line: &str, byte_idx: usize) -> usize {
    line[..byte_idx].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Main test: parse markdown and verify all highlight categories are found.
    #[test]
    fn discover_highlight_categories() {
        let source = r#"# Heading 1

## Heading 2

Normal paragraph with **bold** and *italic* text.

- list item one
- list item two
  - nested item
1. ordered item
2. second item

> block quote text
> more quoted text

`inline code` and a [link](https://example.com)

![alt text](image.png)

```rust
fn main() {}
```

---

~~strikethrough~~

- [ ] unchecked task
- [x] checked task
"#;

        let theme = Theme::dark();
        let mut hl = MarkdownHighlighter::new();
        let ranges = hl.parse(source, &theme);

        assert!(
            !ranges.is_empty(),
            "Expected highlight ranges from markdown"
        );

        // Verify we have heading highlights
        let has_heading = ranges.iter().any(|r| {
            r.style
                == Style::default()
                    .fg(theme.md_heading)
                    .add_modifier(Modifier::BOLD)
        });
        assert!(has_heading, "Expected heading highlight");

        // Verify we have bold highlights
        let has_bold = ranges
            .iter()
            .any(|r| r.style == Style::default().add_modifier(Modifier::BOLD));
        assert!(has_bold, "Expected bold highlight");

        // Verify we have italic highlights
        let has_italic = ranges
            .iter()
            .any(|r| r.style == Style::default().add_modifier(Modifier::ITALIC));
        assert!(has_italic, "Expected italic highlight");

        // Verify we have code highlights
        let has_code = ranges
            .iter()
            .any(|r| r.style == Style::default().fg(theme.md_code));
        assert!(has_code, "Expected code highlight");

        // Verify we have link highlights
        let has_link = ranges.iter().any(|r| {
            r.style
                == Style::default()
                    .fg(theme.md_link)
                    .add_modifier(Modifier::UNDERLINED)
        });
        assert!(has_link, "Expected link highlight");

        // Verify we have muted highlights (list markers, etc.)
        let has_muted = ranges
            .iter()
            .any(|r| r.style == Style::default().fg(theme.md_muted));
        assert!(has_muted, "Expected muted highlight");
    }

    #[test]
    fn fenced_code_block_contents_highlighted() {
        let source = "```\nfoo\nbar\n```\n";
        let theme = Theme::dark();
        let mut hl = MarkdownHighlighter::new();
        let ranges = hl.parse(source, &theme);

        let code_style = Style::default().fg(theme.md_code);
        // All 4 lines (``` foo bar ```) should be code-styled
        let code_rows: Vec<usize> = ranges
            .iter()
            .filter(|r| r.style == code_style)
            .map(|r| r.start_row)
            .collect();
        assert!(
            code_rows.contains(&0),
            "Opening fence should be code-styled"
        );
        assert!(code_rows.contains(&1), "Code line 1 should be code-styled");
        assert!(code_rows.contains(&2), "Code line 2 should be code-styled");
        assert!(
            code_rows.contains(&3),
            "Closing fence should be code-styled"
        );
    }

    #[test]
    fn strikethrough_detected() {
        let source = "Some ~~deleted~~ text\n";
        let theme = Theme::dark();
        let mut hl = MarkdownHighlighter::new();
        let ranges = hl.parse(source, &theme);

        let has_strikethrough = ranges.iter().any(|r| {
            r.style
                == Style::default()
                    .fg(theme.md_muted)
                    .add_modifier(Modifier::CROSSED_OUT)
        });
        assert!(has_strikethrough, "Expected strikethrough highlight");
    }

    #[test]
    fn horizontal_rule_detected() {
        let source = "---\n";
        let theme = Theme::dark();
        let mut hl = MarkdownHighlighter::new();
        let ranges = hl.parse(source, &theme);

        let has_hr = ranges
            .iter()
            .any(|r| r.style == Style::default().fg(theme.md_muted) && r.start_row == 0);
        assert!(has_hr, "Expected horizontal rule highlight");
    }

    #[test]
    fn unicode_prefix_uses_character_columns() {
        let source = "你好 **bold**";
        let theme = Theme::dark();
        let mut hl = MarkdownHighlighter::new();
        let ranges = hl.parse(source, &theme);

        let bold = ranges
            .iter()
            .find(|r| r.style == Style::default().add_modifier(Modifier::BOLD))
            .expect("expected bold highlight");

        assert_eq!(bold.start_col, 3);
        assert_eq!(bold.end_col, 11);
    }
}
