# Visual Line Movement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Up/Down arrow keys move by visual (wrapped) lines instead of logical lines in the editor, so cursor movement feels natural with soft-wrapped text.

**Architecture:** Add `MoveUpWrapped` and `MoveDownWrapped` actions to edtui's motion module. These use `LineWrapper::wrap_line()` and `chars_width()` (already exist) to compute how many visual rows a logical line occupies, then translate cursor position between logical coordinates (row, col) and visual row offsets. Deepwrite's keymap swaps `MoveUp`/`MoveDown` for the wrapped variants.

**Tech Stack:** Rust, edtui (forked crate at `crates/edtui/`)

---

### Task 1: Add `MoveDownWrapped` action to edtui

**Files:**
- Modify: `crates/edtui/src/actions/motion.rs` — add `MoveDownWrapped` struct + `Execute` impl
- Modify: `crates/edtui/src/actions.rs` — re-export `MoveDownWrapped`

- [ ] **Step 1: Write failing test for `MoveDownWrapped` — basic case**

In `crates/edtui/src/actions/motion.rs`, in the `mod tests` section, add:

```rust
#[test]
fn test_move_down_wrapped_within_same_line() {
    // "ABCDEFGHIJ" with width=5 wraps to:
    // visual row 0: "ABCDE"
    // visual row 1: "FGHIJ"
    let mut state = EditorState::new(Lines::from("ABCDEFGHIJ"));
    state.mode = EditorMode::Insert;
    state.cursor = Index2::new(0, 2); // on 'C', visual row 0

    MoveDownWrapped { width: 5 }.execute(&mut state);

    // Should move to visual row 1, same visual column → col 7 (= 5 + 2)
    assert_eq!(state.cursor, Index2::new(0, 7));
}
```

- [ ] **Step 2: Write failing test — wrapping to next logical line**

```rust
#[test]
fn test_move_down_wrapped_to_next_logical_line() {
    // "ABCDE" (5 chars) + "FGH" (3 chars), width=5
    // visual row 0: "ABCDE" (line 0, full)
    // visual row 1: "FGH"   (line 1)
    let mut state = EditorState::new(Lines::from("ABCDE\nFGH"));
    state.mode = EditorMode::Insert;
    state.cursor = Index2::new(0, 2); // on 'C'

    MoveDownWrapped { width: 5 }.execute(&mut state);

    // Next visual row is line 1, col clamped to min(2, 3) = 2
    assert_eq!(state.cursor, Index2::new(1, 2));
}
```

- [ ] **Step 3: Write failing test — at last visual row of document**

```rust
#[test]
fn test_move_down_wrapped_at_bottom() {
    let mut state = EditorState::new(Lines::from("ABC"));
    state.mode = EditorMode::Insert;
    state.cursor = Index2::new(0, 1);

    MoveDownWrapped { width: 5 }.execute(&mut state);

    // Already at bottom, should not move
    assert_eq!(state.cursor, Index2::new(0, 1));
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p edtui move_down_wrapped`
Expected: compilation error — `MoveDownWrapped` does not exist.

- [ ] **Step 5: Implement `MoveDownWrapped`**

In `crates/edtui/src/actions/motion.rs`, after the `MoveDown` impl (around line 85), add:

```rust
/// Move the cursor down by one visual (wrapped) line.
///
/// When soft wrapping is enabled, a single logical line may span multiple
/// visual rows. This action moves the cursor to the next visual row,
/// staying within the same logical line if it wraps, or crossing to the
/// next logical line when at the last visual row.
#[derive(Clone, Debug, Copy)]
pub struct MoveDownWrapped {
    pub width: usize,
}

impl Execute for MoveDownWrapped {
    fn execute(&mut self, state: &mut EditorState) {
        if self.width == 0 || state.lines.is_empty() {
            return;
        }

        let row = state.cursor.row;
        let col = state.cursor.col;
        let line: Vec<char> = state
            .lines
            .get(jagged::index::RowIndex::new(row))
            .map(|l| l.to_vec())
            .unwrap_or_default();

        let tab_width = state.view.tab_width;
        let wrapped = crate::view::line_wrapper::LineWrapper::wrap_line(&line, self.width, tab_width);
        let num_visual_rows = wrapped.len().max(1);

        // Find which visual row the cursor is on and the column offset
        let mut chars_before = 0;
        let mut visual_row = 0;
        for (i, vline) in wrapped.iter().enumerate() {
            if chars_before + vline.len() > col {
                visual_row = i;
                break;
            }
            if i == wrapped.len() - 1 {
                visual_row = i;
                break;
            }
            chars_before += vline.len();
        }
        let visual_col = col - chars_before;

        if visual_row + 1 < num_visual_rows {
            // Move to next visual row within the same logical line
            let next_row_start = chars_before + wrapped[visual_row].len();
            let next_row_len = wrapped[visual_row + 1].len();
            let target_col = visual_col.min(next_row_len.saturating_sub(
                if state.mode == EditorMode::Insert { 0 } else { 1 },
            ));
            state.cursor.col = next_row_start + target_col;
        } else if row + 1 < state.lines.len() {
            // Move to the first visual row of the next logical line
            let next_line: Vec<char> = state
                .lines
                .get(jagged::index::RowIndex::new(row + 1))
                .map(|l| l.to_vec())
                .unwrap_or_default();
            let next_wrapped =
                crate::view::line_wrapper::LineWrapper::wrap_line(&next_line, self.width, tab_width);
            let next_first_len = next_wrapped.first().map_or(0, |v| v.len());
            let target_col = visual_col.min(next_first_len.saturating_sub(
                if state.mode == EditorMode::Insert { 0 } else { 1 },
            ));
            state.cursor.row = row + 1;
            state.cursor.col = target_col;
        }
        // else: at last visual row of last line — don't move

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}
```

- [ ] **Step 6: Re-export in `crates/edtui/src/actions.rs`**

Find the existing `pub use self::motion::` line and add `MoveDownWrapped`:

```rust
pub use self::motion::{
    MoveBackward, MoveDown, MoveDownWrapped, MoveForward, MoveHalfPageDown, MoveHalfPageUp,
    MovePageDown, MovePageUp, MoveToEndOfLine, MoveToFirst, MoveToFirstRow, MoveToLastRow,
    MoveToMatchinBracket, MoveToStartOfLine, MoveUp, MoveWordBackward, MoveWordForward,
    MoveWordForwardToEndOfWord,
};
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p edtui move_down_wrapped`
Expected: all 3 tests PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/edtui/src/actions/motion.rs crates/edtui/src/actions.rs
git commit -m "feat(edtui): add MoveDownWrapped for visual line movement"
```

---

### Task 2: Add `MoveUpWrapped` action to edtui

**Files:**
- Modify: `crates/edtui/src/actions/motion.rs` — add `MoveUpWrapped` struct + `Execute` impl
- Modify: `crates/edtui/src/actions.rs` — re-export `MoveUpWrapped`

- [ ] **Step 1: Write failing test — basic case**

```rust
#[test]
fn test_move_up_wrapped_within_same_line() {
    // "ABCDEFGHIJ" with width=5 wraps to:
    // visual row 0: "ABCDE"
    // visual row 1: "FGHIJ"
    let mut state = EditorState::new(Lines::from("ABCDEFGHIJ"));
    state.mode = EditorMode::Insert;
    state.cursor = Index2::new(0, 7); // on 'H', visual row 1

    MoveUpWrapped { width: 5 }.execute(&mut state);

    // Should move to visual row 0, same visual column → col 2
    assert_eq!(state.cursor, Index2::new(0, 2));
}
```

- [ ] **Step 2: Write failing test — wrapping to previous logical line**

```rust
#[test]
fn test_move_up_wrapped_to_prev_logical_line() {
    // "ABCDEFGHIJ" (wraps to 2 visual rows) + "XY"
    // width=5:
    // visual row 0: "ABCDE" (line 0)
    // visual row 1: "FGHIJ" (line 0)
    // visual row 2: "XY"    (line 1)
    let mut state = EditorState::new(Lines::from("ABCDEFGHIJ\nXY"));
    state.mode = EditorMode::Insert;
    state.cursor = Index2::new(1, 1); // on 'Y', visual row 2

    MoveUpWrapped { width: 5 }.execute(&mut state);

    // Should move to last visual row of line 0 (visual row 1), col 1 → logical col 6
    assert_eq!(state.cursor, Index2::new(0, 6));
}
```

- [ ] **Step 3: Write failing test — at top of document**

```rust
#[test]
fn test_move_up_wrapped_at_top() {
    let mut state = EditorState::new(Lines::from("ABC"));
    state.mode = EditorMode::Insert;
    state.cursor = Index2::new(0, 1);

    MoveUpWrapped { width: 5 }.execute(&mut state);

    // Already at top, should not move
    assert_eq!(state.cursor, Index2::new(0, 1));
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p edtui move_up_wrapped`
Expected: compilation error — `MoveUpWrapped` does not exist.

- [ ] **Step 5: Implement `MoveUpWrapped`**

In `crates/edtui/src/actions/motion.rs`, after the `MoveDownWrapped` impl, add:

```rust
/// Move the cursor up by one visual (wrapped) line.
///
/// Mirror of `MoveDownWrapped`. Moves to the previous visual row within
/// the same logical line, or to the last visual row of the previous
/// logical line when at the first visual row.
#[derive(Clone, Debug, Copy)]
pub struct MoveUpWrapped {
    pub width: usize,
}

impl Execute for MoveUpWrapped {
    fn execute(&mut self, state: &mut EditorState) {
        if self.width == 0 || state.lines.is_empty() {
            return;
        }

        let row = state.cursor.row;
        let col = state.cursor.col;
        let line: Vec<char> = state
            .lines
            .get(jagged::index::RowIndex::new(row))
            .map(|l| l.to_vec())
            .unwrap_or_default();

        let tab_width = state.view.tab_width;
        let wrapped = crate::view::line_wrapper::LineWrapper::wrap_line(&line, self.width, tab_width);

        // Find which visual row the cursor is on
        let mut chars_before = 0;
        let mut visual_row = 0;
        for (i, vline) in wrapped.iter().enumerate() {
            if chars_before + vline.len() > col {
                visual_row = i;
                break;
            }
            if i == wrapped.len() - 1 {
                visual_row = i;
                break;
            }
            chars_before += vline.len();
        }
        let visual_col = col - chars_before;

        if visual_row > 0 {
            // Move to previous visual row within the same logical line
            let prev_row_start: usize = wrapped[..visual_row - 1].iter().map(|v| v.len()).sum();
            let prev_row_len = wrapped[visual_row - 1].len();
            let target_col = visual_col.min(prev_row_len.saturating_sub(
                if state.mode == EditorMode::Insert { 0 } else { 1 },
            ));
            state.cursor.col = prev_row_start + target_col;
        } else if row > 0 {
            // Move to the last visual row of the previous logical line
            let prev_line: Vec<char> = state
                .lines
                .get(jagged::index::RowIndex::new(row - 1))
                .map(|l| l.to_vec())
                .unwrap_or_default();
            let prev_wrapped =
                crate::view::line_wrapper::LineWrapper::wrap_line(&prev_line, self.width, tab_width);
            let last_row_start: usize = if prev_wrapped.len() > 1 {
                prev_wrapped[..prev_wrapped.len() - 1]
                    .iter()
                    .map(|v| v.len())
                    .sum()
            } else {
                0
            };
            let last_row_len = prev_wrapped.last().map_or(0, |v| v.len());
            let target_col = visual_col.min(last_row_len.saturating_sub(
                if state.mode == EditorMode::Insert { 0 } else { 1 },
            ));
            state.cursor.row = row - 1;
            state.cursor.col = last_row_start + target_col;
        }
        // else: at first visual row of first line — don't move

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}
```

- [ ] **Step 6: Re-export in `crates/edtui/src/actions.rs`**

Add `MoveUpWrapped` to the existing `pub use self::motion::` line.

- [ ] **Step 7: Run tests**

Run: `cargo test -p edtui move_up_wrapped && cargo test -p edtui move_down_wrapped`
Expected: all 6 tests PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/edtui/src/actions/motion.rs crates/edtui/src/actions.rs
git commit -m "feat(edtui): add MoveUpWrapped for visual line movement"
```

---

### Task 3: Wire wrapped movement into deepwrite's keymap

**Files:**
- Modify: `src/editor/keymap.rs` — replace `MoveUp(1)`/`MoveDown(1)` with custom wrap-aware handler
- Modify: `src/app.rs` — intercept Up/Down in `handle_edit_key` to pass viewport width

The keymap uses static `MoveUp(1)`/`MoveDown(1)` actions but `MoveUpWrapped`/`MoveDownWrapped` need a `width` field that depends on the current editor area (known at render time). So we intercept Up/Down in `handle_edit_key` before passing to edtui.

- [ ] **Step 1: Intercept Up/Down in `handle_edit_key`**

In `src/app.rs`, in `handle_edit_key`, before the `// Everything else goes to the editor` section, add:

```rust
// Up/Down: use wrapped movement for visual line navigation
if key.code == KeyCode::Up || key.code == KeyCode::Down {
    use edtui::actions::Execute;
    let width = self.editor_line_width as usize;
    if key.code == KeyCode::Up {
        edtui::actions::MoveUpWrapped { width }.execute(&mut self.editor.state);
    } else {
        edtui::actions::MoveDownWrapped { width }.execute(&mut self.editor.state);
    }
    self.editor.update_highlights(&self.theme, self.focus_mode);
    return;
}
```

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all pass.

- [ ] **Step 3: Manual test**

Run: `cargo run -- .` — open a long-paragraph Markdown file, verify Up/Down moves by visual line, not logical line.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "feat: use visual line movement for Up/Down in Edit mode

Up/Down arrow keys now move by wrapped visual lines instead of
logical lines, matching expected behavior for soft-wrapped text."
```
