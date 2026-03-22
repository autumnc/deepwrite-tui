//! Word and character counting with proper Unicode support.

use unicode_segmentation::UnicodeSegmentation;

/// Count the number of words in a string using Unicode word boundaries.
pub fn count_words(text: &str) -> usize {
    text.unicode_words().count()
}

/// Count the number of user-perceived characters (grapheme clusters).
pub fn count_chars(text: &str) -> usize {
    text.graphemes(true).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_words() {
        assert_eq!(count_words("Hello world"), 2);
    }

    #[test]
    fn empty_string() {
        assert_eq!(count_words(""), 0);
        assert_eq!(count_chars(""), 0);
    }

    #[test]
    fn char_count_ascii() {
        assert_eq!(count_chars("Hello"), 5);
    }

    #[test]
    fn char_count_cjk() {
        assert_eq!(count_chars("\u{4f60}\u{597d}"), 2);
    }
}
