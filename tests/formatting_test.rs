use deepwrite::editor::formatting::{
    link_template, toggle_heading, unwrap_if_wrapped, wrap_selection,
};

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
