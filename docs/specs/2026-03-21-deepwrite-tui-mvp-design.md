# Deepwrite TUI MVP Design Spec

## Overview

Deepwrite TUI is a terminal-based Markdown writing tool that combines Yazi-style file browsing with iA Writer-style focused writing. It is one of two product lines — the TUI version targets semi-technical users who work with Claude Code and primarily edit Markdown files (skills, specs, documentation), while a native macOS app targets general writers.

### Target Users

Semi-technical users who use AI tools like Claude Code in the terminal. They edit skills, specs, design documents, and notes — primarily Markdown files, not code. They are comfortable with the terminal but don't necessarily know Vim or advanced terminal workflows.

### Design Principles

- **Type and go** — No Vim modes to learn. Arrow keys and typing work immediately.
- **Easy to install** — `brew install deepwrite`, single binary, zero dependencies.
- **Focus like iA Writer** — Focus Mode, minimal UI, Markdown-native.
- **Lives next to Claude Code** — tmux/zellij split pane, same terminal workflow.

## Architecture

```
deepwrite (single Rust binary)
├── App Shell (Ratatui + crossterm)
│   ├── Dual-panel layout (file browser + editor)
│   ├── Mode switching (Browse ↔ Edit)
│   ├── Status bar (word/char count)
│   └── Theme (light/dark, auto-detect terminal)
│
├── File Browser (left panel)
│   ├── Directory tree navigation (arrow keys + Enter)
│   ├── Fuzzy search (file name filter)
│   ├── Create / rename / delete files
│   └── Auto-collapse on Focus Mode
│
├── Editor Core (right panel)
│   ├── edtui (fork) — text editing engine
│   ├── Focus Mode engine (sentence/paragraph/typewriter)
│   ├── Markdown syntax coloring (tree-sitter)
│   ├── Formatting shortcuts (Ctrl+B/I/K)
│   └── Sentence boundary detection (Chinese/English)
│
└── Services
    ├── tree-sitter + tree-sitter-markdown
    ├── Auto-save (notify + debounce)
    ├── Config system (~/.config/deepwrite/config.toml)
    ├── Word/character counting (unicode-segmentation)
    └── Terminal capability detection (truecolor, etc.)
```

**Key architectural decision:** Fork edtui as the text editing engine for MVP. edtui provides non-modal editing, soft wrapping, clipboard integration, and is designed as an embeddable Ratatui widget. Post-MVP, migrate to a ropey-based custom core if edtui's limitations become blocking.

**Why edtui over building from scratch:** Text editing is one of the hardest UI problems in software. edtui gives us working cursor movement, selection, clipboard, and soft wrapping out of the box. Building these from zero would consume the entire MVP timeline.

**Why Ratatui:** Largest Rust TUI ecosystem, most active development, best editor widget options. Immediate-mode rendering gives full control over layout. Used by Yazi, Dawn, and other modern terminal tools.

## File Browser (Left Panel)

```
┌─ ~/Projects/my-specs ──────┐┌──────────────────────────────────┐
│ 📁 skills/                 ││                                  │
│   ├── code-review.md       ││                                  │
│   ├── commit.md            ││   (editor area)                  │
│   └── brainstorm.md        ││                                  │
│ 📁 docs/                   ││                                  │
│   ├── api-design.md  ←     ││                                  │
│   └── roadmap.md           ││                                  │
│ README.md                  ││                                  │
│ CLAUDE.md                  ││                                  │
└────────────────────────────┘└──────────────────────────────────┘
```

### Navigation

| Action | Key |
|--------|-----|
| Move up/down | `↑` / `↓` (or `j` / `k`) |
| Enter directory | `→` or `Enter` |
| Go up one level | `←` or `Backspace` |
| Open file for editing | `Enter` (on a file) |
| New file | `n` |
| New directory | `N` |
| Rename | `r` |
| Delete | `d` (with confirmation prompt) |
| Fuzzy search | `/` then type to filter |
| Toggle hidden files | `.` |
| Quit Deepwrite | `q` |

### Behavior

- On launch, shows the directory specified by `deepwrite [path]`, defaults to `cwd`
- Only shows `.md` and `.txt` files (and directories), hides all other file types
- Hides dotfiles (`.git/`, etc.) by default, toggle with `.`
- When Focus Mode is entered, left panel auto-collapses; restores on Focus Mode exit
- Panel width: fixed 30 characters (MVP simplification, not resizable)
- Sort order: directories first, then alphabetical by filename

## Editor Core

```
┌────────────────────────────────────────────────┐
│                                                │
│                                                │
│        # API Design Spec                       │
│                                                │
│        ## Overview                              │
│                                                │
│        This document describes the API          │
│        design for the new authentication        │
│        service. The goal is to provide a        │
│        simple, secure interface for all         │
│        client applications.█                    │
│                                                │
│        ## Endpoints                             │
│                                                │
│        ### POST /auth/login                     │
│                                                │
│                                                │
├────────────────────────────────────────────────┤
│ 📝 api-design.md         42 words  231 chars   │
└────────────────────────────────────────────────┘
```

### Text Editing

- **Engine:** forked edtui, non-modal (type immediately on entering editor)
- **Content width:** 72 characters, centered with auto-calculated left/right padding
- **Soft wrapping:** Enabled, wraps at word boundaries
- **Undo / Redo:** `Ctrl+Z` / `Ctrl+Y`
- **Selection:** `Shift+arrow keys` to select, `Ctrl+A` to select all
- **Clipboard:** `Ctrl+C` copy, `Ctrl+V` paste, `Ctrl+X` cut (system clipboard)
- **Scroll past end:** Enabled (half screen of whitespace)

### Markdown Syntax Coloring (Source Mode)

Markdown symbols always visible, styled by element type. Parsed incrementally via tree-sitter with `tree-sitter-markdown` grammar.

| Element | Light Mode Color | Dark Mode Color | ANSI Style |
|---------|-----------------|-----------------|------------|
| Headings (`#`) | `#4080A0` | `#7AA4C2` | foreground + **bold** |
| Bold (`**`) | inherit | inherit | **bold** |
| Italic (`*`) | inherit | inherit | *italic* |
| Links (`[]()`) | `#2A7AB5` | `#5BA3D9` | foreground + underline |
| Code (`` ` ``) | `#6B8E6B` | `#8FB88F` | foreground |
| Fenced code blocks (` ``` `) | `#6B8E6B` | `#8FB88F` | foreground |
| Block quotes (`>`) | `#888888` | `#777777` | foreground |
| List markers (`-`, `*`, `1.`) | `#888888` | `#777777` | foreground |
| Task lists (`- [ ]`, `- [x]`) | `#888888` | `#777777` | foreground |
| Strikethrough (`~~`) | `#888888` | `#777777` | foreground + strikethrough |
| Horizontal rules (`---`) | `#888888` | `#777777` | foreground |
| Images (`![]()`) | `#2A7AB5` | `#5BA3D9` | foreground |

**Note:** Headings do not vary in font size (terminal limitation). Differentiated by color + bold only. Kitty users may benefit from Text Sizing Protocol support in Phase 2.

### Formatting Shortcuts

| Shortcut | No selection | Text selected | Already-formatted selected |
|----------|-------------|---------------|---------------------------|
| `Ctrl+B` | Insert `****`, cursor between | Wrap in `**...**` | Unwrap (toggle off) |
| `Ctrl+I` | Insert `**`, cursor between | Wrap in `*...*` | Unwrap (toggle off) |
| `Ctrl+K` | Insert `[]()`, cursor in `[]` | `[selection](url)`, cursor in `url` | Unwrap, keep text |
| `Ctrl+1-6` | Prepend `# ` to current line | Prepend `# ` to first selected line | Toggle: remove if same level, replace if different |
| `Ctrl+U` | Insert `~~~~`, cursor between | Wrap in `~~...~~` | Unwrap (toggle off) |

### Mode Transitions

| Action | Key |
|--------|-----|
| Open file from browser | `Enter` |
| Return to file browser from editor | `Esc` (exits Focus Mode first, then second `Esc` returns to browser) |
| Toggle left panel visibility in editor | `Ctrl+E` |

## Focus Mode

Three modes, cycled via `Ctrl+D`: `Off → Sentence → Paragraph → Typewriter → Off`

### Sentence Mode

```
┌────────────────────────────────────────────────┐
│                                                │
│        This document describes the API         │  ← dimmed
│        design for the new authentication       │  ← dimmed
│        service. The goal is to provide a       │  ← ★ active sentence
│        simple, secure interface for all        │  ← ★ active sentence
│        client applications.█                   │  ← ★ active sentence
│                                                │
│        The service supports OAuth 2.0          │  ← dimmed
│        and API key authentication.             │  ← dimmed
│                                                │
└────────────────────────────────────────────────┘
```

- Detects the sentence containing the cursor
- Active sentence: full opacity (theme foreground color)
- All other text: dimmed foreground color

### Paragraph Mode

Same logic as Sentence Mode, but highlights the paragraph containing the cursor (delimited by blank lines / tree-sitter `paragraph` nodes).

### Typewriter Mode

- Cursor line fixed at vertical center of the editor viewport
- Text scrolls upward as user types
- No text dimming (all text at full opacity)
- Scroll past end: at least half the viewport height of padding at the bottom

### Focus Mode Implementation

**Dimming mechanism:**
- Active text: theme foreground color (light `#424242` / dark `#C5C9C6`)
- Inactive text: low-contrast foreground color (light `#C0C0C0` / dark `#555555`)
- No opacity (terminals don't support transparency) — uses pre-calculated low-contrast colors
- Dimming level: default 30%, configurable 10%-60% in config.toml
- Transition: instant (no animation, terminal limitation)

**Sentence boundary detection:**
- English: `.` `!` `?` followed by whitespace or newline
- Chinese: `。` `！` `？` `；` as sentence terminators
- Mixed text: both rulesets active simultaneously
- Uses unicode-segmentation crate with custom rules for Markdown edge cases (avoid splitting on `.` inside headings, code blocks, or URLs)

**Paragraph detection:**
- Uses tree-sitter AST `paragraph` nodes (consistent with native spec)
- Correctly handles fenced code blocks, list items, block quotes

**Focus Mode and UI interaction:**
- Entering Focus Mode → left panel auto-collapses, editor goes full-width
- Exiting Focus Mode (`Esc`) → left panel restores
- Status bar always visible

## Color System

### Light Mode

| Element | Value |
|---------|-------|
| Background | `#F5F6F6` |
| Text | `#424242` |
| Accent | `#00BAFF` |
| Status bar background | `#EAEAEA` |
| Status bar text | `#999999` |
| Dimmed text (Focus Mode) | `#C0C0C0` |
| File browser - directory | `#4080A0` |
| File browser - selected row | accent background + white text |

### Dark Mode

| Element | Value |
|---------|-------|
| Background | `#1D1F20` |
| Text | `#C5C9C6` |
| Accent | `#15BDEC` |
| Status bar background | `#161819` |
| Status bar text | `#666666` |
| Dimmed text (Focus Mode) | `#555555` |
| File browser - directory | `#7AA4C2` |
| File browser - selected row | accent background + white text |

- Default: auto-detect terminal background (crossterm query or `$COLORTERM`)
- Override in config: `theme.mode = "light"` / `"dark"`
- Requires truecolor terminal support; graceful fallback to 256-color approximation when unavailable

## Status Bar

```
├────────────────────────────────────────────────┤
│ 📝 api-design.md         42 words  231 chars   │
└────────────────────────────────────────────────┘
```

- Position: bottom of editor, always visible
- Left: current filename (`Untitled` for new documents)
- Right: word count + character count
- Update frequency: debounced 300ms
- Word counting: unicode-segmentation crate, handles Chinese/English mixed text (Chinese counts per character, English splits by whitespace)

## File Handling

### Supported Formats

- `.md` (Markdown) — primary
- `.txt` (Plain text) — secondary

### Auto-save

- Triggers 500ms after typing stops
- Write to temp file first, then atomic rename
- `Ctrl+S` bypasses debounce, saves immediately
- New untitled documents auto-save to `~/.local/share/deepwrite/unsaved/{uuid}.md`
- On closing an untitled document: prompt for filename and save location

### Encoding

- Read: auto-detect via encoding_rs crate (UTF-8, GBK, Big5, Shift_JIS, EUC-KR, ISO-8859-1)
- Write: always UTF-8

## Keyboard Shortcuts

### Browse Mode (file browser focused)

| Key | Action |
|-----|--------|
| `↑` / `↓` / `j` / `k` | Move up/down |
| `→` / `Enter` | Enter directory or open file |
| `←` / `Backspace` | Go up one level |
| `/` | Fuzzy search |
| `n` | New file |
| `N` | New directory |
| `r` | Rename |
| `d` | Delete (with confirmation) |
| `.` | Toggle hidden files |
| `q` | Quit Deepwrite |

### Edit Mode (editor focused)

| Key | Action |
|-----|--------|
| Arrow keys | Cursor movement |
| `Shift+arrow keys` | Select text |
| `Ctrl+A` | Select all |
| `Ctrl+C` / `Ctrl+V` / `Ctrl+X` | Copy / Paste / Cut (crossterm raw mode intercepts SIGINT) |
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo |
| `Ctrl+S` | Force immediate save |
| `Ctrl+B` | Bold toggle |
| `Ctrl+I` | Italic toggle |
| `Ctrl+K` | Link |
| `Ctrl+1-6` | Heading 1-6 |
| `Ctrl+U` | Strikethrough toggle |
| `Ctrl+D` | Cycle Focus Mode (Off → Sentence → Paragraph → Typewriter → Off) |
| `Ctrl+E` | Toggle left panel visibility |
| `Esc` | Exit Focus Mode; second press returns to Browse mode |

## Configuration

```toml
# ~/.config/deepwrite/config.toml

[editor]
line_width = 72                  # 64 | 72 | 80
auto_save = true
auto_save_delay_ms = 500

[focus]
mode = "off"                     # off | sentence | paragraph | typewriter
opacity = 30                     # 10-60, dimming level for inactive text

[theme]
mode = "system"                  # system | light | dark

[browser]
show_hidden = false
panel_width = 30
```

- First launch with no config file: uses built-in defaults, does not auto-create config file
- Config changes require restart to take effect (MVP simplification, no hot reload)

## CLI

```bash
# Open a directory
deepwrite ~/Projects/my-specs

# Open a specific file (enters Edit mode directly)
deepwrite ~/Projects/my-specs/api-design.md

# No arguments: open current directory
deepwrite

# Version
deepwrite --version
```

## Tech Stack Summary

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Language | Rust | Single binary, cross-platform, performance |
| TUI framework | Ratatui + crossterm | Largest ecosystem, immediate-mode rendering |
| Editor engine | edtui (fork) | Non-modal editing, soft wrap, clipboard, embeddable widget |
| Text data structure | edtui built-in (MVP) → ropey (post-MVP) | MVP speed vs long-term scalability |
| Markdown parsing | tree-sitter + tree-sitter-markdown | Incremental parsing, fast re-highlighting |
| Sentence segmentation | unicode-segmentation + custom rules | Chinese/English boundary detection |
| Auto-save | tokio debounce (internal timer) | Async debounced writes on typing stop |
| External file change detection | notify | Detect when external tools (e.g. Claude Code) modify the open file |
| Config | toml + serde | Standard Rust config pattern |
| Word counting | unicode-segmentation | Accurate CJK + English counting |
| Encoding detection | encoding_rs | Handles GBK, Big5, Shift_JIS, etc. |
| Distribution | cargo install + Homebrew | Single binary, zero dependencies |

## Phase 2+ (Not in MVP)

- Search & replace
- NLP syntax highlighting (adjectives, nouns, verbs colored by part of speech)
- Style Check (filler words, redundancies, clichés detection)
- File preview panel (rendered Markdown)
- Authorship tracking (Markdown Annotations format)
- Multi-buffer / tab switching
- Custom keybindings
- Plugin system
- Config hot reload
- Optional Vim mode
- Kitty Text Sizing Protocol (heading font size differentiation)
