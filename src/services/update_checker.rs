use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use ureq::Agent;

const GITHUB_API_URL: &str = "https://api.github.com/repos/tomdhyang/deepwrite-tui/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);
const CHECK_INTERVAL_SECS: u64 = 86_400;

/// Result of an update check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheckResult {
    pub latest_version: String,
    pub is_newer: bool,
}

#[derive(Debug, Deserialize)]
struct LatestReleaseResponse {
    tag_name: String,
}

/// Compare two semver strings. Returns true if `latest` is newer than `current`.
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    fn parse(v: &str) -> Option<(u32, u32, u32)> {
        let core = v
            .trim()
            .strip_prefix('v')
            .unwrap_or(v.trim())
            .split_once('-')
            .map_or(
                v.trim().strip_prefix('v').unwrap_or(v.trim()),
                |(prefix, _)| prefix,
            );
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some((major, minor, patch))
    }

    match (parse(current), parse(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Return whether a cached check attempt is old enough to re-run.
pub fn is_check_due(last_attempt_secs: u64, now_secs: u64) -> bool {
    now_secs.saturating_sub(last_attempt_secs) >= CHECK_INTERVAL_SECS
}

/// Spawn a background thread that checks GitHub for the latest release.
/// Returns `None` when a cached check attempt is still fresh.
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

fn check_latest_version() -> UpdateCheckResult {
    let latest_version = fetch_latest_version().unwrap_or_default();
    let is_newer = !latest_version.is_empty() && is_newer_version(CURRENT_VERSION, &latest_version);

    UpdateCheckResult {
        latest_version: latest_version.trim_start_matches('v').to_string(),
        is_newer,
    }
}

fn fetch_latest_version() -> Option<String> {
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(CHECK_TIMEOUT))
        .user_agent("deepwrite-update-checker")
        .build()
        .into();

    let mut response = agent.get(GITHUB_API_URL).call().ok()?;
    let body = response.body_mut().read_to_string().ok()?;
    let parsed: LatestReleaseResponse = serde_json::from_str(&body).ok()?;
    Some(parsed.tag_name)
}

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
