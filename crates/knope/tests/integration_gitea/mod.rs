//! Integration tests for Gitea API interactions.
//!
//! These tests verify that Knope can correctly:
//! - Create releases on a Gitea instance via the Release workflow
//! - Handle authentication errors gracefully
//!
//! All tests clean up after themselves by deleting any resources they create.

use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Release {
    id: u64,
    tag_name: String,
}

fn gitea_env() -> (String, String, String, String) {
    let token = std::env::var("KNOPE_INTEGRATION_GITEA_TOKEN")
        .expect("KNOPE_INTEGRATION_GITEA_TOKEN must be set");
    let host = std::env::var("KNOPE_INTEGRATION_GITEA_HOST")
        .expect("KNOPE_INTEGRATION_GITEA_HOST must be set");
    let owner = std::env::var("KNOPE_INTEGRATION_GITEA_OWNER")
        .expect("KNOPE_INTEGRATION_GITEA_OWNER must be set");
    let repo = std::env::var("KNOPE_INTEGRATION_GITEA_REPO")
        .expect("KNOPE_INTEGRATION_GITEA_REPO must be set");
    (token, host, owner, repo)
}

fn http_client() -> Client {
    Client::builder()
        .user_agent("Knope")
        .build()
        .expect("Failed to build HTTP client")
}

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("Failed to run git command")
}

fn assert_git(dir: &Path, args: &[&str]) {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {} failed:\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Build a git remote URL for a Gitea instance with embedded token auth.
fn gitea_remote_url(token: &str, host: &str, owner: &str, repo: &str) -> String {
    let host_without_proto = host
        .strip_prefix("https://")
        .or_else(|| host.strip_prefix("http://"))
        .unwrap_or(host);
    format!("https://knope:{token}@{host_without_proto}/{owner}/{repo}.git")
}

fn api_base(host: &str, owner: &str, repo: &str) -> String {
    format!("{host}/api/v1/repos/{owner}/{repo}")
}

/// Set up a temporary directory with a Git repo, knope.toml, Cargo.toml, and CHANGELOG.md
/// configured for a Gitea release workflow.
fn setup_test_repo(
    token: &str,
    host: &str,
    owner: &str,
    repo: &str,
    branch: &str,
) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path();

    assert_git(path, &["init", "-b", branch]);
    assert_git(path, &["config", "user.email", "integration-test@knope.dev"]);
    assert_git(path, &["config", "user.name", "Knope Integration Test"]);

    let remote_url = gitea_remote_url(token, host, owner, repo);
    assert_git(path, &["remote", "add", "origin", &remote_url]);

    let knope_toml = format!(
        r#"[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

[[workflows.steps]]
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "git commit -m 'chore: release'"

[[workflows.steps]]
type = "Command"
command = "git push"

[[workflows.steps]]
type = "Release"

[gitea]
owner = "{owner}"
repo = "{repo}"
host = "{host}"
"#
    );
    std::fs::write(path.join("knope.toml"), knope_toml).expect("Failed to write knope.toml");
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"integration-test\"\nversion = \"0.0.0\"\n",
    )
    .expect("Failed to write Cargo.toml");
    std::fs::write(path.join("CHANGELOG.md"), "").expect("Failed to write CHANGELOG.md");

    assert_git(path, &["add", "."]);
    assert_git(path, &["commit", "-m", "feat: Initial setup"]);
    assert_git(path, &["tag", "v0.0.0"]);

    std::fs::write(path.join("test.txt"), "integration test content")
        .expect("Failed to write test file");
    assert_git(path, &["add", "."]);
    assert_git(
        path,
        &["commit", "-m", "feat: New feature for integration test"],
    );

    dir
}

async fn delete_release(client: &Client, token: &str, base: &str, release_id: u64) {
    let url = format!("{base}/releases/{release_id}");
    let _ = client
        .delete(&url)
        .query(&[("access_token", token)])
        .send()
        .await;
}

async fn delete_tag(client: &Client, token: &str, base: &str, tag: &str) {
    let url = format!("{base}/tags/{tag}");
    let _ = client
        .delete(&url)
        .query(&[("access_token", token)])
        .send()
        .await;
}

async fn delete_branch(client: &Client, token: &str, base: &str, branch: &str) {
    let url = format!("{base}/branches/{branch}");
    let _ = client
        .delete(&url)
        .query(&[("access_token", token)])
        .send()
        .await;
}

async fn cleanup_release_by_tag(client: &Client, token: &str, base: &str, tag: &str) {
    let url = format!("{base}/releases/tags/{tag}");
    if let Ok(resp) = client
        .get(&url)
        .query(&[("access_token", token)])
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(release) = resp.json::<Release>().await {
                delete_release(client, token, base, release.id).await;
            }
        }
    }
    delete_tag(client, token, base, tag).await;
}

/// Test that `knope release` creates a Gitea release via the real API.
///
/// Sets up a git repository with conventional commits, runs `knope release`,
/// then verifies the release was actually created on the Gitea instance.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn gitea_release_workflow() {
    let (token, host, owner, repo) = gitea_env();
    let client = http_client();
    let base = api_base(&host, &owner, &repo);
    let branch = "integration-test-release";
    let initial_tag = "v0.0.0";
    let expected_tag = "v0.1.0";

    // Clean up any leftover resources from a previous failed run
    cleanup_release_by_tag(&client, &token, &base, expected_tag).await;
    delete_tag(&client, &token, &base, initial_tag).await;
    delete_branch(&client, &token, &base, branch).await;

    let dir = setup_test_repo(&token, &host, &owner, &repo, branch);
    let path = dir.path();

    // Push to remote so knope's git push and release API calls succeed
    assert_git(
        path,
        &["push", "--set-upstream", "origin", branch, "--force"],
    );
    assert_git(path, &["push", "origin", initial_tag, "--force"]);

    // Run knope release
    let output = Command::new(env!("CARGO_BIN_EXE_knope"))
        .current_dir(path)
        .env("GITEA_TOKEN", &token)
        .args(["release"])
        .output()
        .expect("Failed to run knope");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "knope release failed:\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Verify the release was created on Gitea
    let url = format!("{base}/releases/tags/{expected_tag}");
    let resp = client
        .get(&url)
        .query(&[("access_token", token.as_str())])
        .send()
        .await
        .expect("Failed to fetch release");

    assert!(
        resp.status().is_success(),
        "Release {expected_tag} should exist on Gitea, got status {}",
        resp.status()
    );

    let release: Release = resp.json().await.expect("Failed to deserialize release");
    assert_eq!(release.tag_name, expected_tag);

    // Cleanup
    delete_release(&client, &token, &base, release.id).await;
    delete_tag(&client, &token, &base, expected_tag).await;
    delete_tag(&client, &token, &base, initial_tag).await;
    delete_branch(&client, &token, &base, branch).await;
}

/// Test that Knope handles authentication errors gracefully on Gitea.
///
/// Runs `knope release` with an invalid `GITEA_TOKEN` and verifies
/// the command fails with a non-zero exit code.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn gitea_error_bad_token() {
    let (_token, host, owner, repo) = gitea_env();

    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path();

    assert_git(path, &["init"]);
    assert_git(path, &["config", "user.email", "integration-test@knope.dev"]);
    assert_git(path, &["config", "user.name", "Knope Integration Test"]);

    // Workflow without push step — Release will fail at the API call
    let knope_toml = format!(
        r#"[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

[[workflows.steps]]
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "git commit -m 'chore: release'"

[[workflows.steps]]
type = "Release"

[gitea]
owner = "{owner}"
repo = "{repo}"
host = "{host}"
"#
    );
    std::fs::write(path.join("knope.toml"), knope_toml).expect("Failed to write knope.toml");
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"integration-test\"\nversion = \"0.0.0\"\n",
    )
    .expect("Failed to write Cargo.toml");
    std::fs::write(path.join("CHANGELOG.md"), "").expect("Failed to write CHANGELOG.md");

    assert_git(path, &["add", "."]);
    assert_git(path, &["commit", "-m", "feat: Initial setup"]);
    assert_git(path, &["tag", "v0.0.0"]);

    std::fs::write(path.join("test.txt"), "test").expect("Failed to write test file");
    assert_git(path, &["add", "."]);
    assert_git(path, &["commit", "-m", "feat: New feature"]);

    // Run knope release with a bad token
    let output = Command::new(env!("CARGO_BIN_EXE_knope"))
        .current_dir(path)
        .env("GITEA_TOKEN", "bad-token-value")
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
