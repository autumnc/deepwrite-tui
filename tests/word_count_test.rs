//! Integration tests for word and character counting.

use deepwrite::editor::word_count::{count_chars, count_words};

#[test]
fn english_two_words() {
    assert_eq!(count_words("Hello world"), 2);
}

#[test]
fn chinese_characters() {
    // unicode_words splits CJK characters as individual words
    let count = count_words("\u{4f60}\u{597d}\u{4e16}\u{754c}");
    // Each CJK ideograph is its own "word" per UAX#29
    assert!(
        count >= 1,
        "Expected at least 1 word for CJK text, got {count}"
    );
}

#[test]
fn mixed_english_chinese() {
    let count = count_words("Hello \u{4f60}\u{597d} world");
    // "Hello", CJK chars, "world" — exact count depends on UAX#29
    assert!(
        count >= 2,
        "Expected at least 2 words for mixed text, got {count}"
    );
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

#[test]
fn char_count_emoji() {
    // Family emoji (grapheme cluster) should count as 1 character
    assert_eq!(
        count_chars("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"),
        1
    );
}
