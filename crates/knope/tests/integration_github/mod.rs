//! Integration tests for GitHub API interactions.
//!
//! These tests verify that Knope can correctly:
//! - Create releases on GitHub via the Release workflow
//! - Upload binary assets to GitHub releases
//! - Handle authentication errors gracefully
//!
//! All tests clean up after themselves by deleting any resources they create.

use std::{path::Path, process::Command, time::Duration};

use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Release {
    id: u64,
    tag_name: String,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
}

fn github_env() -> (String, String, String) {
    let token = std::env::var("KNOPE_INTEGRATION_GITHUB_TOKEN")
        .expect("KNOPE_INTEGRATION_GITHUB_TOKEN must be set");
    let owner = std::env::var("KNOPE_INTEGRATION_GITHUB_OWNER")
        .expect("KNOPE_INTEGRATION_GITHUB_OWNER must be set");
    let repo = std::env::var("KNOPE_INTEGRATION_GITHUB_REPO")
        .expect("KNOPE_INTEGRATION_GITHUB_REPO must be set");
    (token, owner, repo)
}

fn http_client() -> Client {
    Client::builder()
        .user_agent("Knope")
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client")
}

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(dir)
        .output()
        .expect("Failed to run git command")
}

/// Redact any embedded credentials from text that may contain URLs,
/// to avoid leaking tokens in test failure messages.
fn redact_url_credentials(s: &str) -> String {
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

fn assert_git(dir: &Path, args: &[&str]) {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {} failed:\nstdout: {}\nstderr: {}",
        args.join(" "),
        redact_url_credentials(&String::from_utf8_lossy(&output.stdout)),
        redact_url_credentials(&String::from_utf8_lossy(&output.stderr))
    );
}

/// Push the current branch to the remote origin with network timeout settings so that
/// a slow or unresponsive host does not block the test indefinitely.
fn push_branch(dir: &Path, branch: &str) {
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

/// Add a git remote without including the URL in any panic message, so that
/// tokens embedded in the URL are not leaked into test logs.
fn set_git_remote(dir: &Path, remote_url: &str) {
    let output = Command::new("git")
        .args(["remote", "add", "origin", remote_url])
        .current_dir(dir)
        .output()
        .expect("Failed to run git remote add");
    assert!(output.status.success(), "git remote add failed");
}

/// Set up a temporary directory with a Git repo, knope.toml, Cargo.toml, and CHANGELOG.md.
///
/// The repo is pre-configured at version `0.1.0` so that the `Release` step can immediately
/// create a GitHub release without needing `PrepareRelease` or a `git push` inside knope.
/// The caller is responsible for pushing the branch to the remote before running knope.
fn setup_test_repo(
    version: &str,
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    extra_knope_config: &str,
) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path();

    assert_git(path, &["init", "-b", branch]);
    assert_git(
        path,
        &["config", "user.email", "integration-test@knope.dev"],
    );
    assert_git(path, &["config", "user.name", "Knope Integration Test"]);

    let remote_url = format!("https://x-access-token:{token}@github.com/{owner}/{repo}.git");
    set_git_remote(path, &remote_url);

    // Workflow contains only the Release step: the repo is already at the right version,
    // so no PrepareRelease or git push is needed inside the knope workflow.
    let knope_toml = format!(
        r#"[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"
{extra_knope_config}
[[workflows]]
name = "release"

[[workflows.steps]]
type = "Release"

[github]
owner = "{owner}"
repo = "{repo}"
"#
    );
    std::fs::write(path.join("knope.toml"), knope_toml).expect("Failed to write knope.toml");
    // Version is already at 0.1.0; the Release step will detect there is no v0.1.0 tag yet
    // and create a GitHub release pointing at the current HEAD commit.
    std::fs::write(
        path.join("Cargo.toml"),
        format!("[package]\nname = \"integration-test\"\nversion = \"{version}\"\n"),
    )
    .expect("Failed to write Cargo.toml");
    std::fs::write(path.join("CHANGELOG.md"), "").expect("Failed to write CHANGELOG.md");

    assert_git(path, &["add", "."]);
    assert_git(path, &["commit", "-m", "chore: release"]);

    dir
}

async fn delete_release(client: &Client, token: &str, owner: &str, repo: &str, release_id: u64) {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/{release_id}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;
}

async fn delete_tag(client: &Client, token: &str, owner: &str, repo: &str, tag: &str) {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/git/refs/tags/{tag}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;
}

async fn delete_branch(client: &Client, token: &str, owner: &str, repo: &str, branch: &str) {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/git/refs/heads/{branch}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;
}

async fn cleanup_release_by_tag(client: &Client, token: &str, owner: &str, repo: &str, tag: &str) {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}");
    if let Ok(resp) = client
        .get(&url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(release) = resp.json::<Release>().await {
                delete_release(client, token, owner, repo, release.id).await;
            }
        }
    }
    delete_tag(client, token, owner, repo, tag).await;
}

/// Test that `knope release` creates a GitHub release via the real API.
///
/// Sets up a git repository pre-configured at version `0.1.0`, pushes the branch to
/// the remote, runs `knope release` (Release step only), then verifies the release
/// was actually created on GitHub.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn github_release_workflow() {
    let (token, owner, repo) = github_env();
    let client = http_client();
    let branch = "integration-test-release";
    let version = "0.1.0";
    let expected_tag = format!("v{version}");

    // Clean up any leftover resources from a previous failed run
    cleanup_release_by_tag(&client, &token, &owner, &repo, &expected_tag).await;
    delete_branch(&client, &token, &owner, &repo, branch).await;

    let dir = setup_test_repo(version, &token, &owner, &repo, branch, "");
    let path = dir.path();

    // Push the branch so knope can resolve the HEAD commit SHA when creating the release.
    push_branch(path, branch);

    // Run knope release (Release step only — no PrepareRelease, no git push).
    let output = Command::new(env!("CARGO_BIN_EXE_knope"))
        .current_dir(path)
        .env("GITHUB_TOKEN", &token)
        .args(["release"])
        .output()
        .expect("Failed to run knope");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        cleanup_release_by_tag(&client, &token, &owner, &repo, &expected_tag).await;
        delete_branch(&client, &token, &owner, &repo, branch).await;
        panic!("knope release failed:\nstdout: {stdout}\nstderr: {stderr}");
    }

    // GitHub may take a moment to finalise a new release; poll with retries.
    let mut release_opt = None;
    for _ in 0..5 {
        let resp = client
            .get(format!(
                "https://api.github.com/repos/{owner}/{repo}/releases/tags/{expected_tag}"
            ))
            .header("Authorization", format!("token {token}"))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .expect("Failed to fetch release");
        if resp.status().is_success() {
            release_opt = Some(
                resp.json::<Release>()
                    .await
                    .expect("Failed to deserialize release"),
            );
            break;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let Some(release) = release_opt else {
        cleanup_release_by_tag(&client, &token, &owner, &repo, &expected_tag).await;
        delete_branch(&client, &token, &owner, &repo, branch).await;
        panic!("Release {expected_tag} should exist on GitHub after retries");
    };
    assert_eq!(release.tag_name, expected_tag);

    // Cleanup
    delete_release(&client, &token, &owner, &repo, release.id).await;
    delete_tag(&client, &token, &owner, &repo, &expected_tag).await;
    delete_branch(&client, &token, &owner, &repo, branch).await;
}

/// Test that `knope release` can upload binary assets to a GitHub release.
///
/// Configures assets in knope.toml, runs the release workflow, then verifies
/// the asset was uploaded to the created release.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn github_release_with_assets() {
    let (token, owner, repo) = github_env();
    let client = http_client();
    let branch = "integration-test-assets";
    let version = "0.2.0";
    let expected_tag = format!("v{version}");

    // Clean up leftovers
    cleanup_release_by_tag(&client, &token, &owner, &repo, &expected_tag).await;
    delete_branch(&client, &token, &owner, &repo, branch).await;

    let asset_config = "\n[[package.assets]]\npath = \"dist/test-asset.txt\"\n";
    let dir = setup_test_repo(version, &token, &owner, &repo, branch, asset_config);
    let path = dir.path();

    // Create the asset file (it only needs to exist locally when knope runs, not in git).
    std::fs::create_dir_all(path.join("dist")).expect("Failed to create dist dir");
    std::fs::write(
        path.join("dist/test-asset.txt"),
        "Hello from knope integration tests!",
    )
    .expect("Failed to write asset");

    // Push the branch so knope can resolve the HEAD commit SHA when creating the release.
    push_branch(path, branch);

    // Run knope release (Release step only — no PrepareRelease, no git push).
    let output = Command::new(env!("CARGO_BIN_EXE_knope"))
        .current_dir(path)
        .env("GITHUB_TOKEN", &token)
        .args(["release"])
        .output()
        .expect("Failed to run knope");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        cleanup_release_by_tag(&client, &token, &owner, &repo, &expected_tag).await;
        delete_branch(&client, &token, &owner, &repo, branch).await;
        panic!("knope release with assets failed:\nstdout: {stdout}\nstderr: {stderr}");
    }

    // Poll for the release to appear.
    let mut release_opt = None;
    for _ in 0..5 {
        let resp = client
            .get(format!(
                "https://api.github.com/repos/{owner}/{repo}/releases/tags/{expected_tag}"
            ))
            .header("Authorization", format!("token {token}"))
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .expect("Failed to fetch release");
        if resp.status().is_success() {
            release_opt = Some(
                resp.json::<Release>()
                    .await
                    .expect("Failed to deserialize release"),
            );
            break;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let Some(release) = release_opt else {
        cleanup_release_by_tag(&client, &token, &owner, &repo, &expected_tag).await;
        delete_branch(&client, &token, &owner, &repo, branch).await;
        panic!("Release {expected_tag} should exist on GitHub after retries");
    };

    // Check assets
    let assets_url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/{}/assets",
        release.id
    );
    let resp = client
        .get(&assets_url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .expect("Failed to fetch release assets");

    let assets: Vec<Asset> = resp.json().await.expect("Failed to deserialize assets");

    // Cleanup
    delete_release(&client, &token, &owner, &repo, release.id).await;
    delete_tag(&client, &token, &owner, &repo, &expected_tag).await;
    delete_branch(&client, &token, &owner, &repo, branch).await;

    assert!(
        assets.iter().any(|a| a.name == "test-asset.txt"),
        "Uploaded asset should appear in the release assets list"
    );
}

/// Test that Knope handles authentication errors gracefully.
///
/// Runs `knope release` with an invalid `GITHUB_TOKEN` and verifies
/// the command fails with a non-zero exit code.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn github_error_bad_token() {
    let (_token, owner, repo) = github_env();

    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path();

    assert_git(path, &["init"]);
    assert_git(
        path,
        &["config", "user.email", "integration-test@knope.dev"],
    );
    assert_git(path, &["config", "user.name", "Knope Integration Test"]);

    // Workflow with only the Release step — it will fail at the API call due to the bad token.
    let knope_toml = format!(
        r#"[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

[[workflows.steps]]
type = "Release"

[github]
owner = "{owner}"
repo = "{repo}"
"#
    );
    std::fs::write(path.join("knope.toml"), knope_toml).expect("Failed to write knope.toml");
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"integration-test\"\nversion = \"0.1.0\"\n",
    )
    .expect("Failed to write Cargo.toml");
    std::fs::write(path.join("CHANGELOG.md"), "").expect("Failed to write CHANGELOG.md");

    assert_git(path, &["add", "."]);
    assert_git(path, &["commit", "-m", "chore: release"]);

    // Run knope release with a bad token
    let output = Command::new(env!("CARGO_BIN_EXE_knope"))
        .current_dir(path)
        .env("GITHUB_TOKEN", "bad-token-value")
        .args(["release"])
        .output()
        .expect("Failed to run knope");

    assert!(
        !output.status.success(),
        "knope release should fail with a bad token"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.is_empty(),
        "Expected error output when using a bad token"
    );
}
