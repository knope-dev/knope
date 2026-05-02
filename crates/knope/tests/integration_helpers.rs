#![allow(dead_code)]
//! Shared helpers for GitHub and Gitea integration test modules.

use std::{path::Path, process::Command};

/// Redact any embedded credentials from text that may contain URLs,
/// to avoid leaking tokens in test failure messages.
pub(crate) fn redact_url_credentials(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut remaining = s;
    while let Some(scheme_end) = remaining.find("://") {
        result.push_str(&remaining[..scheme_end + 3]);
        remaining = &remaining[scheme_end + 3..];
        if let Some(at_pos) = remaining.find('@') {
            result.push_str("<redacted>");
            remaining = &remaining[at_pos..];
        }
    }
    result.push_str(remaining);
    result
}

/// Push the current branch to the remote origin with network timeout settings so that
/// a slow or unresponsive host does not block the test indefinitely.
pub(crate) fn push_branch(dir: &Path, branch: &str) {
    let output = Command::new("git")
        .args([
            "-c",
            "http.connectTimeout=30",
            "-c",
            "http.lowSpeedLimit=1000",
            "-c",
            "http.lowSpeedTime=30",
            "push",
            "--set-upstream",
            "origin",
            branch,
            "--force",
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(dir)
        .output()
        .expect("Failed to run git push");
    assert!(
        output.status.success(),
        "git push failed:\nstdout: {}\nstderr: {}",
        redact_url_credentials(&String::from_utf8_lossy(&output.stdout)),
        redact_url_credentials(&String::from_utf8_lossy(&output.stderr))
    );
}
