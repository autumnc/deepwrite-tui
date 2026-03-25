# List Item Focus Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Sentence focus mode highlight only the current Markdown list entry block when the cursor is on a list, without changing Paragraph mode behavior.

**Architecture:** Keep `find_paragraph_at_cursor` in `src/editor/focus.rs` as the shared blank-line paragraph primitive used by Paragraph mode. Add a new Sentence-only scope helper in `src/editor/focus.rs` and call it from `find_sentence_at_cursor`. That helper should:
- Fall back to `find_paragraph_at_cursor` for normal prose.
- Detect Markdown list markers for the current line or its continuation lines.
- Return the current list entry block for Sentence mode only.
- Ignore list-like prefixes inside fenced code blocks.

**Non-goals / scope limits:** This change does not implement full CommonMark list-item parsing. Blank-line-separated subparagraphs inside a list item, and complex nested-list semantics, remain out of scope for this fix. The supported scope is: the list marker line plus its directly attached continuation lines within the same contiguous block.

**Tech Stack:** Rust, existing `focus.rs` and `sentence.rs` modules

---

## File Map

| Action | File | Purpose |
|--------|------|---------|
| Modify | `src/editor/focus.rs` | Add sentence-only list scope helpers and tests |
| No change | `src/editor/sentence.rs` | Sentence detector stays punctuation-based |

---

### Task 1: Add tests that lock down the intended behavior

**Files:**
- Modify: `src/editor/focus.rs` (test module at bottom)

- [ ] **Step 1: Add Sentence mode tests that currently fail**

Add tests covering the new behavior:

```rust
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
```

- [ ] **Step 2: Add guard tests for the regressions called out in review**

Add tests that should stay green before and after the implementation:

```rust
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
```

Notes:
- The first test protects Paragraph mode semantics.
- The second test protects against false positives from list-like prefixes inside fenced code blocks.

- [ ] **Step 3: Run focused tests to confirm the failure shape**

Run:

```bash
cargo test --lib find_sentence_at_cursor_highlights_single
```

Expected:
- The new Sentence mode list tests fail on `main` because the current implementation scopes to the whole blank-line paragraph.
- The Paragraph-mode regression test should already pass.

- [ ] **Step 4: Commit the test changes**

```bash
git add src/editor/focus.rs
git commit -m "test: cover sentence focus behavior for markdown lists"
```

---

### Task 2: Implement sentence-only list scope detection

**Files:**
- Modify: `src/editor/focus.rs`

- [ ] **Step 1: Add a helper that detects Markdown list markers conservatively**

Add a private helper above `find_sentence_at_cursor`, for example:

```rust
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
```

Why this shape:
- It keeps the heuristic local to Sentence mode.
- It avoids the `trim_start()` false positive that would classify indented code as a list.

- [ ] **Step 2: Add a fenced-code guard**

Add a small helper that marks which rows are inside fenced code blocks, for example by toggling on lines that start with `` ``` ``. This helper can stay private to `focus.rs`.

Expected behavior:
- If the cursor row is inside a fenced code block, Sentence mode should fall back to the existing paragraph path.
- List-like content inside code fences must not trigger list scoping.

- [ ] **Step 3: Add a Sentence-only scope helper**

Add a helper such as:

```rust
fn find_sentence_scope_at_cursor(text: &str, cursor_row: usize) -> Option<FocusRange>
```

Behavior:
1. Split `text` into lines.
2. Return `None` if `cursor_row` is out of bounds or on a blank line.
3. Build or query the fenced-code mask. If `cursor_row` is inside fenced code, return `find_paragraph_at_cursor(text, cursor_row)`.
4. Try to locate the current list entry block:
   - If the cursor row is a list marker row, use it as the anchor.
   - Otherwise, scan upward within the same contiguous non-blank block to find the nearest preceding list marker row.
   - Stop the upward scan at blank lines or fenced-code boundaries.
5. If no list marker anchor is found, return `find_paragraph_at_cursor(text, cursor_row)`.
6. Once anchored to a list marker row, scan downward and include directly attached continuation lines.
7. Stop before:
   - a blank line,
   - a fenced-code boundary,
   - or a new sibling list marker.

Important limitation:
- Do not document this helper as full list-item parsing.
- For this change, a "list entry block" means the marker line plus directly attached continuation lines in the same contiguous block.

- [ ] **Step 4: Wire Sentence mode to the new helper**

Update `find_sentence_at_cursor` so Step 1 becomes:

```rust
let para = match find_sentence_scope_at_cursor(text, cursor_row) {
    Some(p) => p,
    None => {
        return FocusRange {
            start_row: cursor_row,
            end_row: cursor_row,
        };
    }
};
```

Do not change `find_paragraph_at_cursor`.

- [ ] **Step 5: Run targeted tests**

Run:

```bash
cargo test --lib focus
cargo test --lib find_sentence_at_cursor
```

Expected:
- New list-focused Sentence mode tests pass.
- Existing Paragraph mode tests remain green because `find_paragraph_at_cursor` is unchanged.

- [ ] **Step 6: Commit**

```bash
git add src/editor/focus.rs
git commit -m "feat: scope sentence focus to markdown list entry blocks"
```

---

### Task 3: Verify broader regressions and document the constraint

**Files:**
- Modify: `docs/superpowers/plans/2026-03-25-list-item-focus-mode.md` only if implementation details changed

- [ ] **Step 1: Run the full relevant suite**

Run:

```bash
cargo test --workspace
```

Expected:
- All tests pass.
- No regressions in existing focus-mode behavior outside list handling.

- [ ] **Step 2: Sanity check the intended edge cases**

Verify these cases are covered by tests or explicit comments:
- Paragraph mode still treats a contiguous list as one blank-line paragraph.
- Sentence mode falls back to paragraph behavior for non-list prose.
- Fenced code blocks containing `- ` or `1. ` are not treated as Markdown lists.
- Blank-line-separated subparagraphs inside a list item are still out of scope and not claimed as supported behavior.

- [ ] **Step 3: Commit any final doc/test adjustments**

```bash
git add src/editor/focus.rs docs/superpowers/plans/2026-03-25-list-item-focus-mode.md
git commit -m "test: lock down list sentence focus regressions"
```
