//! Custom non-modal keymap for Deepwrite.
//!
//! This builds a `KeyEventHandler` that works exclusively in Insert mode,
//! giving the user a standard text-editor experience (no Vim Normal/Visual
//! mode switching). Arrow keys, Home/End, Backspace/Delete, and common
//! Ctrl shortcuts all work immediately.

use std::collections::HashMap;

use edtui::actions::{
    CopySelection, DeleteChar, DeleteCharForward, LineBreak, MoveBackward, MoveDown, MoveForward,
    MoveToEndOfLine, MoveToFirstRow, MoveToLastRow, MoveToStartOfLine, MoveUp, MoveWordBackward,
    MoveWordForward, Paste, Redo, SwitchMode, Undo,
};
use edtui::events::{KeyEventHandler, KeyEventRegister, KeyInput};
use edtui::EditorMode;

use crossterm::event::KeyCode;

/// Build a `KeyEventHandler` with insert-mode-only bindings.
///
/// `capture_on_insert` is set to `true` so that every character
/// typed is automatically captured for undo history (emacs-style).
pub fn deepwrite_keymap() -> KeyEventHandler {
    let map: HashMap<KeyEventRegister, edtui::actions::Action> = HashMap::from([
        // ── Navigation ───────────────────────────────────────────
        (
            KeyEventRegister::i(vec![KeyInput::new(KeyCode::Up)]),
            MoveUp(1).into(),
        ),
        (
            KeyEventRegister::i(vec![KeyInput::new(KeyCode::Down)]),
            MoveDown(1).into(),
        ),
        (
            KeyEventRegister::i(vec![KeyInput::new(KeyCode::Left)]),
            MoveBackward(1).into(),
        ),
        (
            KeyEventRegister::i(vec![KeyInput::new(KeyCode::Right)]),
            MoveForward(1).into(),
        ),
        // Shift+Arrow selection in non-modal editing: enter Visual mode on the
        // first keypress, then let motion extend the selection.
        (KeyEventRegister::i(vec![KeyInput::shift(KeyCode::Left)]), {
            use edtui::actions::Chainable;
            SwitchMode(EditorMode::Visual).chain(MoveBackward(1)).into()
        }),
        (
            KeyEventRegister::i(vec![KeyInput::shift(KeyCode::Right)]),
            {
                use edtui::actions::Chainable;
                SwitchMode(EditorMode::Visual).chain(MoveForward(1)).into()
            },
        ),
        (KeyEventRegister::i(vec![KeyInput::shift(KeyCode::Up)]), {
            use edtui::actions::Chainable;
            SwitchMode(EditorMode::Visual).chain(MoveUp(1)).into()
        }),
        (KeyEventRegister::i(vec![KeyInput::shift(KeyCode::Down)]), {
            use edtui::actions::Chainable;
            SwitchMode(EditorMode::Visual).chain(MoveDown(1)).into()
        }),
        (
            KeyEventRegister::v(vec![KeyInput::shift(KeyCode::Left)]),
            MoveBackward(1).into(),
        ),
        (
            KeyEventRegister::v(vec![KeyInput::shift(KeyCode::Right)]),
            MoveForward(1).into(),
        ),
        (
            KeyEventRegister::v(vec![KeyInput::shift(KeyCode::Up)]),
            MoveUp(1).into(),
        ),
        (
            KeyEventRegister::v(vec![KeyInput::shift(KeyCode::Down)]),
            MoveDown(1).into(),
        ),
        // Word navigation
        (
            KeyEventRegister::i(vec![KeyInput::ctrl(KeyCode::Left)]),
            MoveWordBackward(1).into(),
        ),
        (
            KeyEventRegister::i(vec![KeyInput::ctrl(KeyCode::Right)]),
            MoveWordForward(1).into(),
        ),
        // Home / End
        (
            KeyEventRegister::i(vec![KeyInput::new(KeyCode::Home)]),
            MoveToStartOfLine().into(),
        ),
        (
            KeyEventRegister::i(vec![KeyInput::new(KeyCode::End)]),
            MoveToEndOfLine().into(),
        ),
        // ── Editing ──────────────────────────────────────────────
        (
            KeyEventRegister::i(vec![KeyInput::new(KeyCode::Backspace)]),
            DeleteChar(1).into(),
        ),
        (
            KeyEventRegister::i(vec![KeyInput::new(KeyCode::Delete)]),
            DeleteCharForward(1).into(),
        ),
        (
            KeyEventRegister::i(vec![KeyInput::new(KeyCode::Enter)]),
            LineBreak(1).into(),
        ),
        // ── Clipboard (system via arboard) ───────────────────────
        // Ctrl+C: copy selection (then back to insert mode)
        (
            KeyEventRegister::i(vec![KeyInput::ctrl('c')]),
            CopySelection.into(),
        ),
        // Ctrl+V: paste
        (KeyEventRegister::i(vec![KeyInput::ctrl('v')]), Paste.into()),
        // Ctrl+X: cut selection in insert mode if one exists.
        (KeyEventRegister::i(vec![KeyInput::ctrl('x')]), {
            use edtui::actions::{Chainable, DeleteSelection};
            CopySelection.chain(DeleteSelection).into()
        }),
        // ── Undo / Redo ──────────────────────────────────────────
        (KeyEventRegister::i(vec![KeyInput::ctrl('z')]), Undo.into()),
        (KeyEventRegister::i(vec![KeyInput::ctrl('y')]), Redo.into()),
        // ── Select All (Ctrl+A) ──────────────────────────────────
        // Switch to Visual mode and select from first row to last row.
        // edtui does not have a built-in SelectAll action, so we compose:
        //   SwitchMode(Visual) -> MoveToFirstRow -> then user moves to last
        // Instead we use a two-step: go to first row, switch to visual,
        // then go to last row (this selects everything).
        //
        // Because KeyEventHandler only maps to a single Action (which can
        // be a Composed chain), we chain:
        //   MoveToFirstRow -> SwitchMode(Visual) -> MoveToLastRow
        (KeyEventRegister::i(vec![KeyInput::ctrl('a')]), {
            use edtui::actions::Chainable;
            MoveToFirstRow()
                .chain(MoveToStartOfLine())
                .chain(SwitchMode(EditorMode::Visual))
                .chain(MoveToLastRow())
                .chain(MoveToEndOfLine())
                .into()
        }),
        // ── Visual mode support ──────────────────────────────────
        // When the user ends up in Visual mode (via Ctrl+A), allow them
        // to copy with Ctrl+C, then return to Insert mode.
        (KeyEventRegister::v(vec![KeyInput::ctrl('c')]), {
            use edtui::actions::Chainable;
            CopySelection.chain(SwitchMode(EditorMode::Insert)).into()
        }),
        // Ctrl+X in visual mode: cut (copy + delete selection)
        (KeyEventRegister::v(vec![KeyInput::ctrl('x')]), {
            use edtui::actions::{Chainable, DeleteSelection};
            CopySelection
                .chain(DeleteSelection)
                .chain(SwitchMode(EditorMode::Insert))
                .into()
        }),
        // Esc in visual mode: go back to insert mode (not normal)
        (
            KeyEventRegister::v(vec![KeyInput::new(KeyCode::Esc)]),
            SwitchMode(EditorMode::Insert).into(),
        ),
    ]);

    // `capture_on_insert = true` mirrors emacs mode: every character
    // insertion is captured for undo history.
    KeyEventHandler::new(map, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edtui::{EditorEventHandler, EditorState, Index2, Lines};

    #[test]
    fn ctrl_a_selects_from_buffer_start_even_from_mid_line() {
        let mut state = EditorState::new(Lines::from("Alpha\nBeta"));
        state.mode = EditorMode::Insert;
        state.cursor = Index2::new(1, 2);

        let mut handler = EditorEventHandler::new(deepwrite_keymap());
        handler.on_key_event(KeyInput::ctrl('a'), &mut state);

        let selection = state.selection.expect("expected selection");
        assert_eq!(selection.start(), Index2::new(0, 0));
        assert_eq!(selection.end(), Index2::new(1, 4));
    }

    #[test]
    fn shift_right_starts_visual_selection() {
        let mut state = EditorState::new(Lines::from("Hello"));
        state.mode = EditorMode::Insert;

        let mut handler = EditorEventHandler::new(deepwrite_keymap());
        handler.on_key_event(KeyInput::shift(KeyCode::Right), &mut state);

        let selection = state.selection.expect("expected selection");
        assert_eq!(state.mode, EditorMode::Visual);
        assert_eq!(selection.start(), Index2::new(0, 0));
        assert_eq!(selection.end(), Index2::new(0, 1));
    }
}
