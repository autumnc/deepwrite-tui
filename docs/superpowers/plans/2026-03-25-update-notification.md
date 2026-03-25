# Update Notification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a one-line notification when a newer version of Deepwrite is available on GitHub, so users know to upgrade.

**Architecture:** On startup, Deepwrite may spawn a background thread that calls the GitHub Releases API for the latest stable release and compares it with the compiled-in version. The app stores an optional receiver and, if a newer version is found, an `update_available` string that the status bar renders. To avoid turning the existing test suite into implicit network tests, `App` gets a crate-private constructor that accepts an injected update-check receiver; production uses the real checker, while tests pass `None`. To enforce rate limiting, the checker records a `last_attempt_at` timestamp before spawning the thread, so the app performs at most one update-check attempt per 24 hours.

**Tech Stack:** `ureq` (lightweight blocking HTTP client), `serde_json` (parse GitHub API response), `std::thread` (background check), `std::sync::mpsc` (channel to send result back to main thread)

---

## File Map

| Action | File | Purpose |
|--------|------|---------|
| Modify | `Cargo.toml` | Add networking/parsing dependencies |
| Modify | `src/config.rs` | Add `[updates]` config section and template docs |
| Modify | `tests/config_test.rs` | Verify update config defaults and parsing |
| Create | `src/services/update_checker.rs` | Background version check logic |
| Modify | `src/services/mod.rs` | Export new module |
| Modify | `src/app.rs` | Add update-check receiver, test-safe constructor, polling |
| Modify | `src/ui/status_bar.rs` | Render update notification using styled spans |
| Create | `tests/update_checker.rs` | Tests for version comparison and cache timing helpers |

---

### Task 1: Add dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add `ureq` and `serde_json` to `Cargo.toml`**

Add to `[dependencies]`:

```toml
ureq = "3"
serde_json = "1"
```

`ureq` is a minimal blocking HTTP client, and `serde_json` parses the GitHub API response body.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add dependencies for update checking"
```

---

### Task 2: Add update config section and template docs

**Files:**
- Modify: `src/config.rs`
- Modify: `tests/config_test.rs`

- [ ] **Step 1: Extend config tests in `tests/config_test.rs`**

Add assertions for the new config defaults:

```rust
#[test]
fn test_update_config_defaults() {
    let cfg = Config::default();
    assert!(cfg.updates.check_on_startup);
}
```

Also extend the existing partial-TOML parsing test with:

```rust
assert!(cfg.updates.check_on_startup);
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test config_test`

Expected: FAIL because `Config` has no `updates` field yet.

- [ ] **Step 3: Add `UpdateConfig` to `src/config.rs`**

Add:

```rust
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct UpdateConfig {
    pub check_on_startup: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            check_on_startup: true,
        }
    }
}
```

Add the field to `Config`:

```rust
pub updates: UpdateConfig,
```

- [ ] **Step 4: Update the generated config template**

In `Config::write_template`, add a commented-out section:

```toml
[updates]
# check_on_startup = true
```

This keeps the new feature discoverable for users.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test config_test`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/config.rs tests/config_test.rs
git commit -m "feat: add update check config section"
```

---

### Task 3: Implement update checker core

**Files:**
- Create: `src/services/update_checker.rs`
- Modify: `src/services/mod.rs`
- Create: `tests/update_checker.rs`

- [ ] **Step 1: Write tests for pure version/timing helpers**

Create `tests/update_checker.rs`:

```rust
use deepwrite::services::update_checker::{is_check_due, is_newer_version};

#[test]
fn test_newer_version() {
    assert!(is_newer_version("0.1.0", "0.2.0"));
    assert!(is_newer_version("0.1.0", "0.1.1"));
    assert!(is_newer_version("0.1.0", "1.0.0"));
}

#[test]
fn test_same_version() {
    assert!(!is_newer_version("0.1.0", "0.1.0"));
}

#[test]
fn test_older_version() {
    assert!(!is_newer_version("0.2.0", "0.1.0"));
}

#[test]
fn test_version_with_v_prefix() {
    assert!(is_newer_version("0.1.0", "v0.2.0"));
}

#[test]
fn test_check_is_due_after_interval() {
    assert!(is_check_due(0, 86_400));
}

#[test]
fn test_check_is_not_due_before_interval() {
    assert!(!is_check_due(1_000, 1_000 + 60));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test update_checker`

Expected: FAIL because the module does not exist yet.

- [ ] **Step 3: Create `src/services/update_checker.rs`**

Implement:

- `UpdateCheckResult { latest_version: String, is_newer: bool }`
- `is_newer_version(current, latest) -> bool`
- `is_check_due(last_attempt_secs, now_secs) -> bool`
- `check_for_updates() -> Option<mpsc::Receiver<UpdateCheckResult>>`
- internal helpers for fetching/parsing the GitHub release response

Recommended skeleton:

```rust
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/tomdhyang/deepwrite-tui/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);
const CHECK_INTERVAL_SECS: u64 = 86_400;

pub struct UpdateCheckResult {
    pub latest_version: String,
    pub is_newer: bool,
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    // Parse simple semver triplets, ignoring an optional leading 'v'.
    // Return false on malformed input.
}

pub fn is_check_due(last_attempt_secs: u64, now_secs: u64) -> bool {
    now_secs.saturating_sub(last_attempt_secs) >= CHECK_INTERVAL_SECS
}

pub fn check_for_updates() -> Option<mpsc::Receiver<UpdateCheckResult>> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = check_latest_version();
        let _ = tx.send(result);
    });

    Some(rx)
}
```

For the HTTP/JSON path:

- Set a `User-Agent` header.
- Fail silently on request or parse errors.
- Read `tag_name`.
- Trim a leading `v` before storing `latest_version`.

- [ ] **Step 4: Export the module**

Add to `src/services/mod.rs`:

```rust
pub mod update_checker;
```

- [ ] **Step 5: Run tests**

Run: `cargo test update_checker`

Expected: all helper tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/services/update_checker.rs src/services/mod.rs tests/update_checker.rs
git commit -m "feat: add update checker core"
```

---

### Task 4: Integrate into `App` without polluting tests

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add fields to `App`**

Add:

```rust
update_check_rx: Option<mpsc::Receiver<update_checker::UpdateCheckResult>>,
pub update_available: Option<String>,
```

- [ ] **Step 2: Add a test-safe constructor**

Keep the existing public API:

```rust
pub fn new(config: Config, start_dir: PathBuf) -> Self
```

But make it delegate to a crate-private helper:

```rust
fn new_with_update_receiver(
    config: Config,
    start_dir: PathBuf,
    update_check_rx: Option<mpsc::Receiver<update_checker::UpdateCheckResult>>,
) -> Self
```

Production code should compute:

```rust
let update_check_rx = if config.updates.check_on_startup {
    update_checker::check_for_updates()
} else {
    None
};
```

Then call `new_with_update_receiver(...)`.

This keeps startup behavior intact while allowing tests to inject `None`.

- [ ] **Step 3: Update app tests to disable network checks**

Inside the `src/app.rs` test module:

- update `test_app(...)` to call `new_with_update_receiver(..., None)`
- update any direct `App::new(...)` calls in tests to also use `new_with_update_receiver(..., None)` where appropriate

Goal: no existing test should spawn a real update-check thread.

- [ ] **Step 4: Poll the receiver with a borrow-safe pattern**

In `run()`, add non-blocking polling using `take()` so the code compiles cleanly:

```rust
if let Some(rx) = self.update_check_rx.take() {
    match rx.try_recv() {
        Ok(result) => {
            if result.is_newer {
                self.update_available = Some(result.latest_version);
            }
        }
        Err(mpsc::TryRecvError::Empty) => {
            self.update_check_rx = Some(rx);
        }
        Err(mpsc::TryRecvError::Disconnected) => {}
    }
}
```

Do not use a pattern that borrows `&self.update_check_rx` and then assigns `self.update_check_rx = None` in the same scope.

- [ ] **Step 5: Verify it compiles and tests pass**

Run: `cargo check && cargo test`

Expected: no errors, all tests pass, and the existing test suite remains network-free.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "feat: integrate update checker into app startup"
```

---

### Task 5: Render the notification in the status bar

**Files:**
- Modify: `src/ui/status_bar.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Extend `render_status_bar`**

Update the function signature to accept:

```rust
update_available: Option<&str>,
```

- [ ] **Step 2: Switch the right-hand content to styled spans**

The current status bar renders the right side as one plain string. To make the update notification visually distinct, refactor the right-hand side to build a `Line`/`Span` sequence instead of a single `String`.

Recommended shape:

```rust
let right_line = build_right_status_line(word_count, char_count, update_available, theme);
```

Where:

- `"Update: X available"` uses `theme.accent`
- separators and counts continue using the normal status-bar style
- if `update_available` is `None`, the output remains the existing word/char count

- [ ] **Step 3: Preserve the current layout behavior**

Keep these invariants:

- filename/mode remains on the left
- status message or focus label remains in the center
- update notification lives on the right and does not replace the center status message
- truncation still respects grapheme boundaries and narrow layouts

- [ ] **Step 4: Update the call site in `src/app.rs`**

Pass:

```rust
self.update_available.as_deref()
```

- [ ] **Step 5: Add or update unit tests in `src/ui/status_bar.rs`**

Cover at least:

- no update available -> right section is just counts
- update available -> update segment is present
- existing truncation helpers still behave correctly

- [ ] **Step 6: Verify it compiles**

Run: `cargo check && cargo test status_bar`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/ui/status_bar.rs src/app.rs
git commit -m "feat: show update notification in status bar"
```

---

### Task 6: Add cache-based rate limiting using `last_attempt_at`

**Files:**
- Modify: `src/services/update_checker.rs`

- [ ] **Step 1: Add cache-file helpers**

Store the timestamp at:

```text
~/.local/share/deepwrite/last_update_check
```

Implement helpers such as:

```rust
use std::fs;
use std::path::PathBuf;

fn cache_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("deepwrite").join("last_update_check"))
}

fn current_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn should_check() -> bool {
    let Some(path) = cache_path() else {
        return true;
    };

    let Ok(contents) = fs::read_to_string(path) else {
        return true;
    };

    let Ok(last_attempt) = contents.trim().parse::<u64>() else {
        return true;
    };

    is_check_due(last_attempt, current_unix_secs())
}

fn record_check_attempt() {
    let Some(path) = cache_path() else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let _ = fs::write(path, current_unix_secs().to_string());
}
```

Use `saturating_sub` inside `is_check_due` so future timestamps do not underflow.

- [ ] **Step 2: Record the attempt before spawning the thread**

Update `check_for_updates()` to:

```rust
pub fn check_for_updates() -> Option<mpsc::Receiver<UpdateCheckResult>> {
    if !should_check() {
        return None;
    }

    record_check_attempt();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = check_latest_version();
        let _ = tx.send(result);
    });

    Some(rx)
}
```

This is intentionally `last_attempt_at`, not `last_success_at`, so the app truly performs at most one check attempt per 24 hours.

- [ ] **Step 3: Run tests**

Run: `cargo check && cargo test`

Expected: no errors, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/services/update_checker.rs
git commit -m "feat: rate limit update checks to once per day"
```

---

## Summary

After all 6 tasks, the user experience is:

```text
$ deepwrite
[status bar shows: "Update: 0.2.0 available | 342W  1,205C"]
```

- Check happens in the background and never blocks startup
- The app performs at most one update-check attempt per 24 hours
- Existing tests stay deterministic because they do not spawn real network checks
- Users can disable the behavior with:

```toml
[updates]
check_on_startup = false
```

- Failures remain silent: if offline or GitHub is unavailable, Deepwrite simply shows no update notice
