# Contributing to Deepwrite

Thank you for your interest in contributing!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/<your-username>/deepwrite-tui.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Run checks:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   ```
6. Commit using [Conventional Commits](https://www.conventionalcommits.org/):
   ```
   feat: add new feature
   fix: fix a bug
   docs: update documentation
   ```
7. Push and open a Pull Request

## Development

```bash
cargo build          # Build
cargo run -- [path]  # Run
cargo test           # Test
cargo fmt            # Format
cargo clippy         # Lint
```

This is a Cargo workspace:
- Root (`deepwrite`) — the main application
- `crates/edtui` — forked text editor widget (internal dependency)

## Guidelines

- Keep PRs focused on a single change
- Add tests for new features
- Follow existing code style
- Use conventional commit messages so the changelog is auto-generated

## Reporting Issues

Open an issue on GitHub. Include:
- What you expected to happen
- What actually happened
- Steps to reproduce
- Your OS and terminal emulator
