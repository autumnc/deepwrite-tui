use deepwrite::editor::sentence::{cursor_to_byte_offset, find_all_sentences, find_sentence_at};

#[test]
fn test_english_sentences() {
    let text = "Hello world. This is a test. Another sentence here.";
    // cursor at offset 15 → "This is a test."
    let range = find_sentence_at(text, 15);
    assert_eq!(&text[range.start..range.end], "This is a test.");
}

#[test]
fn test_chinese_sentences() {
    let text = "你好世界。這是一個測試。另一個句子。";
    // cursor in second sentence → "這是一個測試。"
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
    assert_eq!(&text[range.start..range.end], text);
}

#[test]
fn test_cursor_at_end() {
    let text = "First. Second.";
    let range = find_sentence_at(text, text.len());
    assert_eq!(&text[range.start..range.end], "Second.");
}
