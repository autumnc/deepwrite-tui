# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Bug Fixes

- Correct off-by-one in scrolloff and mouse click row calculation
- Handle unsupported key codes instead of panicking
- Correct wrapped line movement behavior

### CI

- Add CI workflow (fmt, clippy, test on 3 platforms)
- Initialize cargo-dist for cross-platform binary releases
- Add release-plz for automated version bumps and changelog updates

### Documentation

- Update spec to reflect actual implementation
- Add problem statement and pain points to spec
- Add open-source release design spec
- Add open-source release implementation plan
- Add README with installation, usage, and configuration
- Add contributing guidelines

### Features

- Deepwrite-tui — terminal Markdown writing tool with Focus Mode
- Unify file/directory creation under single `a` key
- Add h/l navigation keybindings (Yazi vim convention)
- Add help screen overlay (? key in Browse mode)
- Add scrolloff (5-line buffer) to browser list
- Add mouse support (click to select, scroll to navigate)
- Copy file path to clipboard with 'y' key (Browse mode)
- Replace fixed panel_width with ratio-based layout (Yazi-style)
- Add Zhuyin input method support for Ctrl shortcuts
- Add MoveDownWrapped for visual line movement
- Add MoveUpWrapped for visual line movement
- Use visual line movement for Up/Down in Edit mode
- Prompt input cursor navigation + auto-generate config template

### Miscellaneous

- Clean up tracked OS artifacts, stale edtui workflows, and improve .gitignore
- Add MIT license
- Add package metadata and mark edtui as non-publishable
