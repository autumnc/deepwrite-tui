# Deepwrite

A terminal-based Markdown writing tool that combines **Yazi-style file browsing** with **iA Writer-style focused writing**.

Built for anyone who works with Markdown — writers, developers, and "vibe coders" who use AI tools like Claude Code for documentation.

## Features

- **Two-mode interface** — Browse files on the left, edit on the right
- **Focus Mode** — Sentence, paragraph, and typewriter dimming to help you concentrate
- **Non-modal editing** — Just start typing. Arrow keys, not `hjkl`. Emacs-style shortcuts (Ctrl+A/E for home/end)
- **Markdown syntax highlighting** — Headings, bold, italic, code blocks, links
- **Formatting shortcuts** — Ctrl+B for bold, Ctrl+I for italic, Ctrl+1/2/3 for headings
- **CJK-aware word count** — Accurate counting for Chinese, Japanese, Korean text
- **Auto-save** — Debounced 2-second writes via temp file + atomic rename
- **External change detection** — Picks up file changes from other editors
- **Light/dark theme** — Auto-detects system preference
- **Configurable** — `~/.config/deepwrite/config.toml`

## Installation

### Homebrew (macOS / Linux)

```bash
brew tap tomdhyang/tap
brew install deepwrite
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/tomdhyang/deepwrite-tui/releases).

Available for macOS (Intel + Apple Silicon), Linux (x64), and Windows (x64).

### Build from source

```bash
cargo install --git https://github.com/tomdhyang/deepwrite-tui.git
```

Requires [Rust](https://rustup.rs/) (latest stable recommended).

## Usage

```bash
# Open current directory
deepwrite

# Open a specific directory
deepwrite ~/Documents/notes

# Open a specific file
deepwrite README.md
```

### Browse Mode

| Key | Action |
|-----|--------|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `Enter` / `l` | Open file / enter directory |
| `h` / `Backspace` | Go to parent directory |
| `a` | Create new file or directory |
| `r` | Rename |
| `d` | Delete |
| `y` | Copy file path |
| `?` | Help |
| `q` | Quit |

### Edit Mode

| Key | Action |
|-----|--------|
| `Esc` | Back to Browse |
| `Ctrl+B` | Bold |
| `Ctrl+I` | Italic |
| `Ctrl+1/2/3` | Heading 1/2/3 |
| `Ctrl+A` | Go to line start |
| `Ctrl+E` | Go to line end |
| `Ctrl+C` | Copy |
| `Ctrl+V` | Paste |
| `Ctrl+Z` | Undo |

## Configuration

Config file at `~/.config/deepwrite/config.toml`:

```toml
[editor]
tab_width = 4

[focus]
mode = "sentence"     # "none", "sentence", "paragraph", "typewriter"

[theme]
mode = "auto"         # "auto", "light", "dark"

[browser]
show_hidden = false
```

## License

[MIT](LICENSE)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
