# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-03-25

### Bug Fixes

- Address code review findings in CI and release workflows
- Add version to edtui path dependency for cargo package compatibility
- Mark deepwrite as publish=false to skip cargo package in release-plz
- Align release-plz config with Cargo.toml publish=false for deepwrite
- Disable semver check in release-plz to avoid cargo package on forked edtui
- Move git_only to workspace level to trigger release-plz --workspace cargo package fix
- Address code review findings for release workflow
- Reset CHANGELOG to Unreleased and fix plan version reference

### Documentation

- Add update notification implementation plan
- Update notification implementation plan (revised)

### Features

- Add update notification on startup

### Miscellaneous

- Replace release-plz with cargo-release
- Replace release-plz with cargo-release and update docs
## [0.1.0] - 2026-03-25

### Bug Fixes

- Correct off-by-one in scrolloff and mouse click row calculation
- Handle unsupported key codes instead of panicking
- Correct wrapped line movement behavior
- Resolve clippy warning and formatting issues for CI

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
- Add git-cliff config and generate initial changelog
