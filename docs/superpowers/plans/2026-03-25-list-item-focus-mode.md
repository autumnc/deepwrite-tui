# List Item Focus Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Sentence focus mode highlight only the current list item (not the entire bullet list) when the cursor is on a Markdown list.

**Architecture:** Modify `find_paragraph_at_cursor` in `src/editor/focus.rs` to treat list item prefixes (`- `, `* `, `+ `, `1. `) as paragraph boundaries. Each list item becomes its own "paragraph", so sentence detection scopes to that single item. No changes to `sentence.rs` — the sentence detector stays punctuation-based; we just feed it smaller chunks.

**Tech Stack:** Rust, existing `focus.rs` and `sentence.rs` modules

---

## File Map

| Action | File | Purpose |
|--------|------|---------|
| Modify | `src/editor/focus.rs:111-137` | Update paragraph detection to split on list items |
| Modify | `src/editor/focus.rs` (tests) | Add tests for list item paragraph splitting |

---

### Task 1: Add failing tests for list item paragraph detection

**Files:**
- Modify: `src/editor/focus.rs` (test module at bottom)

- [ ] **Step 1: Write failing test — bullet list items are separate paragraphs**

Add to the `#[cfg(test)] mod tests` block in `src/editor/focus.rs`:

```rust
#[test]
fn find_paragraph_splits_bullet_list_items() {
    let source = "### Bug Fixes\n\n- Fix alpha\n- Fix beta\n- Fix gamma\n";

    // Row 2: "- Fix alpha" — should be its own paragraph
    let result = find_paragraph_at_cursor(source, 2).unwrap();
    assert_eq!(result.start_row, 2);
    assert_eq!(result.end_row, 2);

    // Row 3: "- Fix beta" — should be its own paragraph
    let result = find_paragraph_at_cursor(source, 3).unwrap();
    assert_eq!(result.start_row, 3);
    assert_eq!(result.end_row, 3);
}

#[test]
fn find_paragraph_splits_numbered_list_items() {
    let source = "Steps:\n\n1. First step\n2. Second step\n3. Third step\n";

    let result = find_paragraph_at_cursor(source, 2).unwrap();
    assert_eq!(result.start_row, 2);
    assert_eq!(result.end_row, 2);

    let result = find_paragraph_at_cursor(source, 3).unwrap();
    assert_eq!(result.start_row, 3);
    assert_eq!(result.end_row, 3);
}

#[test]
fn find_paragraph_keeps_multiline_list_item_together() {
    // A list item that wraps onto the next line (continuation is indented)
    let source = "- This is a long item that\n  continues on the next line\n- Second item\n";

    // Row 0-1 should be one paragraph (continuation line starts with spaces, not `- `)
    let result = find_paragraph_at_cursor(source, 0).unwrap();
    assert_eq!(result.start_row, 0);
    assert_eq!(result.end_row, 1);

    // Row 2 should be its own paragraph
    let result = find_paragraph_at_cursor(source, 2).unwrap();
    assert_eq!(result.start_row, 2);
    assert_eq!(result.end_row, 2);
}

#[test]
fn find_paragraph_prose_unchanged() {
    // Regular prose without list markers should still group by blank lines
    let source = "First line of paragraph.\nSecond line of paragraph.\n\nNew paragraph.\n";

    let result = find_paragraph_at_cursor(source, 0).unwrap();
    assert_eq!(result.start_row, 0);
    assert_eq!(result.end_row, 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib find_paragraph_splits`

Expected: FAIL — bullet list rows 2 and 3 return `start_row=2, end_row=4` (the whole list) instead of individual items.

- [ ] **Step 3: Commit failing tests**

```bash
git add src/editor/focus.rs
git commit -m "test: add failing tests for list item paragraph detection"
```

---

### Task 2: Implement list-aware paragraph detection

**Files:**
- Modify: `src/editor/focus.rs:111-137`

- [ ] **Step 1: Add a helper to detect list item prefixes**

Add above `find_paragraph_at_cursor`:

```rust
/// Returns true if the line starts with a Markdown list marker (`- `, `* `, `+ `, or `1. `).
fn is_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("+ ")
    {
        return true;
    }
    // Numbered list: one or more digits followed by `. `
    let after_digits = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
    if after_digits.len() < trimmed.len() && after_digits.starts_with(". ") {
        return true;
    }
    false
}
```

- [ ] **Step 2: Update `find_paragraph_at_cursor` to split on list items**

Replace the upward and downward search loops in `find_paragraph_at_cursor`:

```rust
pub fn find_paragraph_at_cursor(text: &str, cursor_row: usize) -> Option<FocusRange> {
    let lines: Vec<&str> = text.split('\n').collect();

    if cursor_row >= lines.len() {
        return None;
    }

    if lines[cursor_row].trim().is_empty() {
        return None;
    }

    let cursor_is_list_item = is_list_item(lines[cursor_row]);

    // Search upward for the start of the paragraph
    let mut start_row = cursor_row;
    while start_row > 0 {
        let prev = lines[start_row - 1];
        // Stop at blank lines (always a boundary)
        if prev.trim().is_empty() {
            break;
        }
        if cursor_is_list_item {
            // Cursor is on a list item line — don't go past it
            break;
        }
        if is_list_item(prev) {
            // Cursor is on a continuation line and prev is the parent
            // list marker — include it, then stop
            start_row -= 1;
            break;
        }
        start_row -= 1;
    }

    // Search downward for the end of the paragraph
    let mut end_row = cursor_row;
    while end_row + 1 < lines.len() {
        let next = lines[end_row + 1];
        // Stop at blank lines
        if next.trim().is_empty() {
            break;
        }
        // If the next line is a new list item, stop
        if is_list_item(next) {
            break;
        }
        end_row += 1;
    }

    Some(FocusRange { start_row, end_row })
}
```

Key behavior:
- **Bullet/numbered list items**: each `- ` / `* ` / `1. ` line starts a new paragraph
- **Continuation lines** (indented, no marker): belong to the list item above them (upward search includes the parent marker)
- **Downward search**: always stops at the next list marker (a new `- ` always starts a new unit, regardless of cursor type)
- **Regular prose**: unchanged — groups by blank lines as before

- [ ] **Step 3: Run all tests**

Run: `cargo test --lib focus`

Expected: all new and existing tests pass.

- [ ] **Step 4: Also run the sentence focus integration test**

Run: `cargo test --lib find_sentence_at_cursor`

Expected: PASS — existing sentence-within-paragraph tests still work.

- [ ] **Step 5: Commit**

```bash
git add src/editor/focus.rs
git commit -m "feat: treat list items as separate paragraphs in focus mode"
```

---

### Task 3: Add sentence-level focus test for bullet lists

**Files:**
- Modify: `src/editor/focus.rs` (test module)

- [ ] **Step 1: Write test verifying sentence focus on a bullet list**

```rust
#[test]
fn find_sentence_at_cursor_highlights_single_list_item() {
    let source = "### Bug Fixes\n\n- Fix alpha\n- Fix beta\n- Fix gamma\n";

    // Cursor on row 3 ("- Fix beta"), col 4
    let focus = find_sentence_at_cursor(source, 3, 4);
    assert_eq!(focus.start_row, 3);
    assert_eq!(focus.end_row, 3);
}

#[test]
fn find_sentence_at_cursor_multiline_list_item() {
    let source = "- This is a long item that\n  continues here\n- Second item\n";

    // Cursor on continuation line (row 1)
    let focus = find_sentence_at_cursor(source, 1, 5);
    assert_eq!(focus.start_row, 0);
    assert_eq!(focus.end_row, 1);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --lib find_sentence_at_cursor`

Expected: PASS.

- [ ] **Step 3: Run full test suite**

Run: `cargo test --workspace`

Expected: all tests pass, no regressions.

- [ ] **Step 4: Commit**

```bash
git add src/editor/focus.rs
git commit -m "test: verify sentence focus scopes to individual list items"
```
