//! Sentence boundary detection for mixed English/Chinese text.
//!
//! This module identifies sentence boundaries using punctuation terminators:
//! - English: `.` `!` `?` followed by whitespace or end of text
//! - Chinese: `。` `！` `？` `；`

/// A byte range representing a single sentence within a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentenceRange {
    /// Byte offset of the first character in the sentence.
    pub start: usize,
    /// Byte offset one past the last character in the sentence (exclusive).
    pub end: usize,
}

/// Returns `true` if `ch` is a Chinese sentence terminator.
fn is_chinese_terminator(ch: char) -> bool {
    matches!(ch, '\u{3002}' | '\u{FF01}' | '\u{FF1F}' | '\u{FF1B}')
    // 。！？；
}

/// Returns `true` if `ch` is an English sentence terminator (`.` `!` `?`).
fn is_english_terminator(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?')
}

/// Find all sentence ranges in `text`.
///
/// Rules:
/// - Chinese terminators (`。！？；`) end a sentence immediately.
/// - English terminators (`.!?`) end a sentence only when followed by
///   whitespace or end-of-text.
/// - Sentences start after the previous terminator, with leading whitespace
///   trimmed.
/// - If no terminator is found, the entire text is one sentence.
pub fn find_all_sentences(text: &str) -> Vec<SentenceRange> {
    if text.is_empty() {
        return vec![];
    }

    let mut sentences = Vec::new();
    let mut sentence_start: Option<usize> = None;
    let mut chars = text.char_indices().peekable();

    while let Some((byte_idx, ch)) = chars.next() {
        // Skip leading whitespace for the current sentence.
        if sentence_start.is_none() {
            if ch.is_whitespace() {
                continue;
            }
            sentence_start = Some(byte_idx);
        }

        if is_chinese_terminator(ch) {
            // Chinese terminators end the sentence immediately (including the
            // terminator character itself).
            let end = byte_idx + ch.len_utf8();
            if let Some(start) = sentence_start {
                sentences.push(SentenceRange { start, end });
            }
            sentence_start = None;
        } else if is_english_terminator(ch) {
            // English terminators only end a sentence if followed by
            // whitespace or end-of-text.
            let end = byte_idx + ch.len_utf8();
            let terminates = match chars.peek() {
                None => true, // end of text
                Some(&(_, next_ch)) => next_ch.is_whitespace(),
            };
            if terminates {
                if let Some(start) = sentence_start {
                    sentences.push(SentenceRange { start, end });
                }
                sentence_start = None;
            }
        }
    }

    // If there is remaining text that was never terminated, it forms its own
    // sentence.
    if let Some(start) = sentence_start {
        sentences.push(SentenceRange {
            start,
            end: text.len(),
        });
    }

    // If no sentences were found at all (e.g. whitespace-only text), treat
    // the whole text as one sentence.
    if sentences.is_empty() {
        sentences.push(SentenceRange {
            start: 0,
            end: text.len(),
        });
    }

    sentences
}

/// Find the sentence that contains the given `byte_offset`.
///
/// If the offset falls between sentences (in whitespace), the next sentence
/// is returned. If the offset is past the last sentence, the last sentence
/// is returned.
pub fn find_sentence_at(text: &str, byte_offset: usize) -> SentenceRange {
    let sentences = find_all_sentences(text);

    // Should always have at least one entry thanks to the fallback in
    // `find_all_sentences`.
    if sentences.is_empty() {
        return SentenceRange {
            start: 0,
            end: text.len(),
        };
    }

    for s in &sentences {
        if byte_offset < s.end {
            return s.clone();
        }
    }

    // Past the last sentence — return the last one.
    sentences.last().unwrap().clone()
}

/// Convert a (row, col) cursor position to a byte offset within the full
/// editor text content.
///
/// `col` is measured in **characters** (not bytes), matching how edtui
/// stores cursor positions.
pub fn cursor_to_byte_offset(text: &str, row: usize, col: usize) -> usize {
    let mut byte_offset = 0;

    for (current_row, line) in text.split('\n').enumerate() {
        if current_row == row {
            // Walk `col` characters into this line.
            let char_offset = line
                .char_indices()
                .nth(col)
                .map(|(idx, _)| idx)
                .unwrap_or(line.len());
            return byte_offset + char_offset;
        }
        // +1 for the '\n' separator
        byte_offset += line.len() + 1;
    }

    // If the row is past the end, return end of text.
    text.len()
}

/// Convert a byte offset range to a (start_row, end_row) pair.
///
/// This scans the text to determine which lines the range spans.
pub fn byte_range_to_rows(text: &str, start_byte: usize, end_byte: usize) -> (usize, usize) {
    let mut start_row = 0;
    let mut end_row = 0;
    let mut byte_offset = 0;

    for (row_idx, line) in text.split('\n').enumerate() {
        let line_end = byte_offset + line.len(); // exclusive, before the '\n'

        if byte_offset <= start_byte && start_byte <= line_end {
            start_row = row_idx;
        }
        if byte_offset < end_byte && end_byte <= line_end + 1 {
            // +1 to include the '\n' boundary
            end_row = row_idx;
        }

        byte_offset = line_end + 1; // skip past '\n'
    }

    // If end_byte is at the very end of text (no trailing newline), make sure
    // end_row is set to the last line.
    if end_byte >= text.len() && !text.is_empty() {
        let total_lines = text.split('\n').count();
        if total_lines > 0 {
            end_row = total_lines - 1;
        }
    }

    (start_row, end_row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_sentences() {
        let text = "Hello world. This is a test. Another sentence here.";
        let range = find_sentence_at(text, 15);
        assert_eq!(&text[range.start..range.end], "This is a test.");
    }

    #[test]
    fn test_chinese_sentences() {
        let text = "你好世界。這是一個測試。另一個句子。";
        // Find the byte offset inside the second sentence.
        // "你好世界。" is 5 chars * 3 bytes = 15 bytes. Second sentence starts at 15.
        let offset = cursor_to_byte_offset(text, 0, 6); // 6th char = '是'
        let range = find_sentence_at(text, offset);
        assert_eq!(&text[range.start..range.end], "這是一個測試。");
    }

    #[test]
    fn test_mixed_language() {
        let text = "Hello world。這是中文。More English.";
        let sentences = find_all_sentences(text);
        assert_eq!(sentences.len(), 3);
        assert_eq!(&text[sentences[0].start..sentences[0].end], "Hello world。");
        assert_eq!(&text[sentences[1].start..sentences[1].end], "這是中文。");
        assert_eq!(&text[sentences[2].start..sentences[2].end], "More English.");
    }

    #[test]
    fn test_exclamation_and_question() {
        let text = "Really? Yes! OK.";
        let sentences = find_all_sentences(text);
        assert_eq!(sentences.len(), 3);
        assert_eq!(&text[sentences[0].start..sentences[0].end], "Really?");
        assert_eq!(&text[sentences[1].start..sentences[1].end], "Yes!");
        assert_eq!(&text[sentences[2].start..sentences[2].end], "OK.");
    }

    #[test]
    fn test_chinese_punctuation() {
        let text = "你好！真的嗎？是的。";
        let sentences = find_all_sentences(text);
        assert_eq!(sentences.len(), 3);
        assert_eq!(&text[sentences[0].start..sentences[0].end], "你好！");
        assert_eq!(&text[sentences[1].start..sentences[1].end], "真的嗎？");
        assert_eq!(&text[sentences[2].start..sentences[2].end], "是的。");
    }

    #[test]
    fn test_single_sentence() {
        let text = "Just one sentence.";
        let range = find_sentence_at(text, 5);
        assert_eq!(&text[range.start..range.end], "Just one sentence.");
    }

    #[test]
    fn test_cursor_at_end() {
        let text = "First. Second.";
        let range = find_sentence_at(text, text.len());
        assert_eq!(&text[range.start..range.end], "Second.");
    }

    #[test]
    fn test_cursor_to_byte_offset_ascii() {
        let text = "Hello\nWorld";
        assert_eq!(cursor_to_byte_offset(text, 0, 0), 0);
        assert_eq!(cursor_to_byte_offset(text, 0, 5), 5);
        assert_eq!(cursor_to_byte_offset(text, 1, 0), 6);
        assert_eq!(cursor_to_byte_offset(text, 1, 3), 9);
    }

    #[test]
    fn test_cursor_to_byte_offset_unicode() {
        let text = "你好世界";
        // Each Chinese char is 3 bytes.
        assert_eq!(cursor_to_byte_offset(text, 0, 0), 0);
        assert_eq!(cursor_to_byte_offset(text, 0, 1), 3);
        assert_eq!(cursor_to_byte_offset(text, 0, 2), 6);
    }

    #[test]
    fn test_byte_range_to_rows() {
        let text = "Hello world. This is a test.\nAnother sentence.";
        assert_eq!(byte_range_to_rows(text, 0, 12), (0, 0));
        assert_eq!(byte_range_to_rows(text, 29, 47), (1, 1));
    }
}
