# Update Notification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a one-line notification when a newer version of Deepwrite is available on GitHub, so users know to upgrade.

**Architecture:** On startup, spawn a background thread that checks GitHub Releases API for the latest version. Compare with the compiled-in version. If newer, set a flag on App that the status bar renders. Cache the last check timestamp to avoid hitting the API every launch — check at most once per day.

**Tech Stack:** `ureq` (lightweight blocking HTTP client), `serde_json` (parse GitHub API response), `std::thread` (background check), `std::sync::mpsc` (channel to send result back to main thread)

---

## File Map

| Action | File | Purpose |
|--------|------|---------|
| Create | `src/services/update_checker.rs` | Background version check logic |
| Modify | `src/services/mod.rs` | Export new module |
| Modify | `src/app.rs` | Add update check receiver, display logic |
| Modify | `src/ui/status_bar.rs` | Render update notification |
| Modify | `src/config.rs` | Add `[updates]` config section |
| Modify | `Cargo.toml` | Add `ureq` and `serde_json` dependencies |
| Create | `tests/update_checker.rs` | Tests for version comparison logic |

---

### Task 1: Add dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add ureq and serde_json to Cargo.toml**

Add to `[dependencies]`:

```toml
ureq = "3"
serde_json = "1"
```

`ureq` is a minimal blocking HTTP client (no async runtime needed). `serde_json` parses the GitHub API response. Both are lightweight.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add ureq and serde_json for update checking"
```

---

### Task 2: Add update config section

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write test for update config defaults**

Add to `tests/config.rs` (or the existing config test file):

```rust
#[test]
fn test_update_config_defaults() {
    let config: Config = toml::from_str("").unwrap();
    assert!(config.updates.check_on_startup);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_update_config_defaults`

Expected: FAIL — `updates` field doesn't exist on Config yet.

- [ ] **Step 3: Add UpdateConfig struct to config.rs**

In `src/config.rs`, add:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConfig {
    #[serde(default = "default_true")]
    pub check_on_startup: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            check_on_startup: true,
        }
    }
}

fn default_true() -> bool {
    true
}
```

Add to the `Config` struct:

```rust
#[serde(default)]
pub updates: UpdateConfig,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_update_config_defaults`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs tests/
git commit -m "feat: add update check config section"
```

---

### Task 3: Implement version comparison and GitHub check

**Files:**
- Create: `src/services/update_checker.rs`
- Modify: `src/services/mod.rs`

- [ ] **Step 1: Write tests for version comparison**

Create `tests/update_checker.rs`:

```rust
use deepwrite::services::update_checker::is_newer_version;

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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_newer_version`

Expected: FAIL — module doesn't exist yet.

- [ ] **Step 3: Create update_checker.rs**

Create `src/services/update_checker.rs`:

```rust
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/tomdhyang/deepwrite-tui/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Compare two semver strings. Returns true if `latest` is newer than `current`.
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Option<(u32, u32, u32)> {
        let v = v.strip_prefix('v').unwrap_or(v);
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ))
    };

    match (parse(current), parse(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Result of an update check.
pub struct UpdateCheckResult {
    pub latest_version: String,
    pub is_newer: bool,
}

/// Spawn a background thread that checks GitHub for the latest release.
/// Returns a receiver that will eventually contain the result (or nothing if the check fails).
pub fn check_for_updates() -> mpsc::Receiver<UpdateCheckResult> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = check_latest_version();
        // Ignore send error — receiver may have been dropped if app quit early
        let _ = tx.send(result);
    });

    rx
}

fn check_latest_version() -> UpdateCheckResult {
    let response = ureq::get(GITHUB_API_URL)
        .header("User-Agent", "deepwrite-update-checker")
        .timeout(CHECK_TIMEOUT)
        .call();

    let latest_version = response
        .ok()
        .and_then(|r| r.into_body().read_to_string().ok())
        .and_then(|body| {
            let json: serde_json::Value = serde_json::from_str(&body).ok()?;
            json["tag_name"].as_str().map(String::from)
        })
        .unwrap_or_default();

    let is_newer = !latest_version.is_empty() && is_newer_version(CURRENT_VERSION, &latest_version);

    UpdateCheckResult {
        latest_version: latest_version.trim_start_matches('v').to_string(),
        is_newer,
    }
}
```

- [ ] **Step 4: Export module in services/mod.rs**

Add to `src/services/mod.rs`:

```rust
pub mod update_checker;
```

- [ ] **Step 5: Run tests**

Run: `cargo test update_checker`

Expected: all 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/services/update_checker.rs src/services/mod.rs tests/update_checker.rs
git commit -m "feat: add update checker with GitHub Releases API"
```

---

### Task 4: Integrate into App startup and event loop

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add fields to App struct**

Add to the `App` struct:

```rust
update_check_rx: Option<mpsc::Receiver<update_checker::UpdateCheckResult>>,
pub update_available: Option<String>, // Latest version string, if newer
```

- [ ] **Step 2: Initialize in App::new()**

In `App::new()`, after existing initialization:

```rust
let update_check_rx = if config.updates.check_on_startup {
    Some(update_checker::check_for_updates())
} else {
    None
};
```

Set `update_available: None` in the struct initialization.

- [ ] **Step 3: Poll in the event loop**

In the `run()` method, inside the main loop (alongside auto_save and file_watcher polling), add:

```rust
// Check for update result (non-blocking)
if let Some(rx) = &self.update_check_rx {
    if let Ok(result) = rx.try_recv() {
        if result.is_newer {
            self.update_available = Some(result.latest_version);
        }
        self.update_check_rx = None; // Done checking
    }
}
```

- [ ] **Step 4: Verify it compiles and tests pass**

Run: `cargo check && cargo test`

Expected: no errors, all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat: integrate update checker into app startup and event loop"
```

---

### Task 5: Render notification in status bar

**Files:**
- Modify: `src/ui/status_bar.rs`
- Modify: `src/app.rs` (the render call site)

- [ ] **Step 1: Add update_available parameter to render_status_bar**

Update the `render_status_bar` function signature to accept an optional update version:

```rust
pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    filename: &str,
    word_count: usize,
    char_count: usize,
    focus_label: &str,
    theme: &Theme,
    update_available: Option<&str>,  // NEW
)
```

- [ ] **Step 2: Render update notification**

In the function body, when `update_available` is `Some(version)`, render a notification on the left side or center of the status bar. For example, prepend to the right section:

```rust
let right_text = if let Some(version) = update_available {
    format!("Update: {} available  |  {} words  {} chars", version, word_count, char_count)
} else {
    format!("{} words  {} chars", word_count, char_count)
};
```

Use the theme's accent or warning color to make "Update: X available" visually distinct.

- [ ] **Step 3: Update the call site in app.rs**

Where `render_status_bar` is called, pass `self.update_available.as_deref()`.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/ui/status_bar.rs src/app.rs
git commit -m "feat: show update notification in status bar"
```

---

### Task 6: Cache check timestamp (rate limiting)

**Files:**
- Modify: `src/services/update_checker.rs`

- [ ] **Step 1: Add cache file logic**

The cache file goes at `~/.local/share/deepwrite/last_update_check` (via `dirs::data_dir()`). It stores the Unix timestamp of the last check.

Add to `update_checker.rs`:

```rust
use std::fs;
use std::path::PathBuf;

const CHECK_INTERVAL_SECS: u64 = 86400; // 24 hours

fn cache_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("deepwrite").join("last_update_check"))
}

fn should_check() -> bool {
    let Some(path) = cache_path() else {
        return true;
    };

    let Ok(contents) = fs::read_to_string(&path) else {
        return true;
    };

    let Ok(last_check) = contents.trim().parse::<u64>() else {
        return true;
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    now - last_check > CHECK_INTERVAL_SECS
}

fn update_cache() {
    let Some(path) = cache_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default();
    let _ = fs::write(path, now);
}
```

- [ ] **Step 2: Integrate into check_for_updates()**

Modify `check_for_updates()`:

```rust
pub fn check_for_updates() -> Option<mpsc::Receiver<UpdateCheckResult>> {
    if !should_check() {
        return None;
    }

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = check_latest_version();
        update_cache();
        let _ = tx.send(result);
    });

    Some(rx)
}
```

Update `App::new()` accordingly (the return type changed from `Receiver` to `Option<Receiver>`).

- [ ] **Step 3: Verify it compiles and tests pass**

Run: `cargo check && cargo test`

Expected: no errors, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/services/update_checker.rs src/app.rs
git commit -m "feat: cache update check timestamp (once per day max)"
```

---

## Summary

After all 6 tasks, the user experience is:

```
$ deepwrite
[status bar shows: "Update: 0.2.0 available | 342 words  1,205 chars"]
```

- Check happens in background, never blocks startup
- At most once per 24 hours (cached)
- Configurable: set `check_on_startup = false` in config to disable
- Fails silently (no error if offline or API fails)
