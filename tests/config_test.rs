use deepwrite::config::Config;

#[test]
fn test_default_config() {
    let cfg = Config::default();

    // Editor defaults
    assert_eq!(cfg.editor.line_width, 72);
    assert!(cfg.editor.auto_save);
    assert_eq!(cfg.editor.auto_save_delay_ms, 500);

    // Focus defaults
    assert_eq!(cfg.focus.mode, "off");
    assert_eq!(cfg.focus.opacity, 30);

    // Theme defaults
    assert_eq!(cfg.theme.mode, "system");

    // Browser defaults
    assert!(!cfg.browser.show_hidden);
    assert_eq!(cfg.browser.ratio, [1, 3]);
}

#[test]
fn test_parse_partial_toml() {
    let toml_str = r#"
[editor]
line_width = 80

[focus]
mode = "sentence"
"#;

    let cfg = Config::from_toml_str(toml_str).expect("should parse partial TOML");

    // Overridden values
    assert_eq!(cfg.editor.line_width, 80);
    assert_eq!(cfg.focus.mode, "sentence");

    // Defaults for everything else
    assert!(cfg.editor.auto_save);
    assert_eq!(cfg.editor.auto_save_delay_ms, 500);
    assert_eq!(cfg.focus.opacity, 30);
    assert_eq!(cfg.theme.mode, "system");
    assert!(!cfg.browser.show_hidden);
    assert_eq!(cfg.browser.ratio, [1, 3]);
}

#[test]
fn test_invalid_toml_returns_error() {
    let bad_toml = "this is [[[not valid toml";
    let result = Config::from_toml_str(bad_toml);
    assert!(result.is_err(), "invalid TOML should return an error");
}
