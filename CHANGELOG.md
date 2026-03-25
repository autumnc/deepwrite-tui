# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Bug Fixes

- Address code review findings in CI and release workflows
- Add version to edtui path dependency for cargo package compatibility
- Mark deepwrite as publish=false to skip cargo package in release-plz
- Align release-plz config with Cargo.toml publish=false for deepwrite
- Disable semver check in release-plz to avoid cargo package on forked edtui
- Move git_only to workspace level to trigger release-plz --workspace cargo package fix
- Address code review findings for release workflow

### Documentation

- Add update notification implementation plan
- Update notification implementation plan (revised)

### Features

- Add update notification on startup

### Miscellaneous

- Replace release-plz with cargo-release
- Replace release-plz with cargo-release and update docs
