//! Markdown formatting helpers: bold, italic, links, headings.
//!
//! These functions operate on text strings and are designed to be used
//! with the editor's selection and line manipulation APIs.

/// Wrap `text` with `marker` on both sides.
///
/// # Example
/// ```ignore
/// wrap_selection("hello", "**") // → "**hello**"
/// ```
pub fn wrap_selection(text: &str, marker: &str) -> String {
    format!("{marker}{text}{marker}")
}

/// If `text` is already wrapped with `marker`, return the inner content.
/// Otherwise return `None`.
///
/// # Example
/// ```ignore
/// unwrap_if_wrapped("**hello**", "**") // → Some("hello")
/// unwrap_if_wrapped("hello", "**")     // → None
/// ```
pub fn unwrap_if_wrapped(text: &str, marker: &str) -> Option<String> {
    if text.len() >= marker.len() * 2 && text.starts_with(marker) && text.ends_with(marker) {
        Some(text[marker.len()..text.len() - marker.len()].to_string())
    } else {
        None
    }
}

/// Create a Markdown link template.
///
/// If `selected_text` is empty, produces `[](url)`.
/// Otherwise produces `[selected_text](url)`.
pub fn link_template(selected_text: &str) -> String {
    format!("[{selected_text}](url)")
}

/// Toggle a heading level on a line.
///
/// - If the line has no heading, add `level` hashes.
/// - If the line has the same heading level, remove it.
/// - If the line has a different heading level, change to `level`.
///
/// # Example
/// ```ignore
/// toggle_heading("Some text", 2)     // → "## Some text"
/// toggle_heading("## Some text", 2)  // → "Some text"
/// toggle_heading("## Some text", 3)  // → "### Some text"
/// ```
pub fn toggle_heading(line: &str, level: usize) -> String {
    let level = level.clamp(1, 6);

    // Detect existing heading level.
    let trimmed = line.trim_start();
    let hash_count = trimmed.chars().take_while(|&c| c == '#').count();

    if hash_count > 0 && trimmed.len() > hash_count {
        // Check that hashes are followed by a space (valid heading).
        let after_hashes = &trimmed[hash_count..];
        if let Some(content) = after_hashes.strip_prefix(' ') {
            let content = content.to_string();
            if hash_count == level {
                // Same level — remove heading.
                return content;
            }
            // Different level — change to requested level.
            let hashes = "#".repeat(level);
            return format!("{hashes} {content}");
        }
    }

    // No existing heading — add one.
    let hashes = "#".repeat(level);
    format!("{hashes} {line}")
}

/// Toggle an inline marker (bold, italic, strikethrough) on `text`.
///
/// If the text is already wrapped, unwrap it; otherwise wrap it.
pub fn toggle_marker(text: &str, marker: &str) -> String {
    if let Some(inner) = unwrap_if_wrapped(text, marker) {
        inner
    } else {
        wrap_selection(text, marker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold_wrap() {
        assert_eq!(wrap_selection("hello", "**"), "**hello**");
    }

    #[test]
    fn test_bold_unwrap() {
        assert_eq!(
            unwrap_if_wrapped("**hello**", "**"),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_bold_unwrap_not() {
        assert_eq!(unwrap_if_wrapped("hello", "**"), None);
    }

    #[test]
    fn test_italic_wrap() {
        assert_eq!(wrap_selection("hello", "*"), "*hello*");
    }

    #[test]
    fn test_link_no_selection() {
        assert_eq!(link_template(""), "[](url)");
    }

    #[test]
    fn test_link_with_selection() {
        assert_eq!(link_template("click here"), "[click here](url)");
    }

    #[test]
    fn test_heading_add() {
        assert_eq!(toggle_heading("Some text", 2), "## Some text");
    }

    #[test]
    fn test_heading_remove() {
        assert_eq!(toggle_heading("## Some text", 2), "Some text");
    }

    #[test]
    fn test_heading_change() {
        assert_eq!(toggle_heading("## Some text", 3), "### Some text");
    }

    #[test]
    fn test_toggle_marker_wrap() {
        assert_eq!(toggle_marker("hello", "**"), "**hello**");
    }

    #[test]
    fn test_toggle_marker_unwrap() {
        assert_eq!(toggle_marker("**hello**", "**"), "hello");
    }

    #[test]
    fn test_strikethrough_wrap() {
        assert_eq!(wrap_selection("hello", "~~"), "~~hello~~");
    }

    #[test]
    fn test_strikethrough_unwrap() {
        assert_eq!(
            unwrap_if_wrapped("~~hello~~", "~~"),
            Some("hello".to_string())
        );
    }
}
