# Changelog

## 0.4.0 (2026-06-07)

### Added

- **High-contrast theme** — optimized for `#101010` background, configurable via `theme.mode = "high_contrast"`
- **Heading hierarchy colors** — H1-H6 each use distinct colors (warm-to-cool gradient) for visual distinction
- **`==highlight==` syntax** — highlight text with reversed background color (highlighter pen effect)
- **`<u>underline</u>` syntax** — underline text support
- **Line focus mode** — highlights only the current editing line, dims everything else, cursor stays vertically centered. Cycle via `Ctrl+F`
- **Ctrl+H shortcut** — toggle highlight formatting
- **Ctrl+D shortcut** — toggle strikethrough formatting

### Changed
- **Ctrl+U** now toggles underline (was strikethrough)
- **Block quotes** now render with italic text and distinct color from body text
- **Paste** now inserts at cursor position instead of before it
- **Esc in Visual mode** first exits selection mode (back to Insert), second Esc returns to Browse
- Focus mode cycle extended: Off → Sentence → Paragraph → Typewriter → Line → Off

### Fixed
- Shift+Arrow text selection compatibility with Linux console / fbterm
- Ctrl+number keys not working in fbterm (documented workaround: use F1-F6)

## 0.3.0 (2026-05-01)

Initial public release.
