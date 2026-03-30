//! Integration tests for GitHub API interactions.
//!
//! These tests verify that Knope can correctly:
//! - Create releases on GitHub via the Release workflow
//! - Upload binary assets to GitHub releases
//! - Handle authentication errors gracefully
//!
//! All tests clean up after themselves by deleting any resources they create.

use std::{path::Path, process::Command};

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

/// Set up a temporary directory with a Git repo, knope.toml, Cargo.toml, and CHANGELOG.md.
///
/// The repo is configured with a remote pointing to the real test repository and has
/// an initial commit tagged `v0.0.0` followed by a conventional commit.
fn setup_test_repo(
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
    assert_git(path, &["remote", "add", "origin", &remote_url]);

    let knope_toml = format!(
        r#"[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"
{extra_knope_config}
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

[github]
owner = "{owner}"
repo = "{repo}"
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
/// Sets up a git repository with conventional commits, runs `knope release`,
/// then verifies the release was actually created on GitHub.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn github_release_workflow() {
    let (token, owner, repo) = github_env();
    let client = http_client();
    let branch = "integration-test-release";
    let initial_tag = "v0.0.0";
    let expected_tag = "v0.1.0";

    // Clean up any leftover resources from a previous failed run
    cleanup_release_by_tag(&client, &token, &owner, &repo, expected_tag).await;
    delete_tag(&client, &token, &owner, &repo, initial_tag).await;
    delete_branch(&client, &token, &owner, &repo, branch).await;

    let dir = setup_test_repo(&token, &owner, &repo, branch, "");
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
        .env("GITHUB_TOKEN", &token)
        .args(["release"])
        .output()
        .expect("Failed to run knope");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "knope release failed:\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Verify the release was created on GitHub
    let resp = client
        .get(format!(
            "https://api.github.com/repos/{owner}/{repo}/releases/tags/{expected_tag}"
        ))
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .expect("Failed to fetch release");

    assert!(
        resp.status().is_success(),
        "Release {expected_tag} should exist on GitHub, got status {}",
        resp.status()
    );

    let release: Release = resp.json().await.expect("Failed to deserialize release");
    assert_eq!(release.tag_name, expected_tag);

    // Cleanup
    delete_release(&client, &token, &owner, &repo, release.id).await;
    delete_tag(&client, &token, &owner, &repo, expected_tag).await;
    delete_tag(&client, &token, &owner, &repo, initial_tag).await;
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
    let initial_tag = "v0.0.0";
    let expected_tag = "v0.1.0";

    // Clean up leftovers
    cleanup_release_by_tag(&client, &token, &owner, &repo, expected_tag).await;
    delete_tag(&client, &token, &owner, &repo, initial_tag).await;
    delete_branch(&client, &token, &owner, &repo, branch).await;

    let asset_config = "\n[[package.assets]]\npath = \"dist/test-asset.txt\"\n";
    let dir = setup_test_repo(&token, &owner, &repo, branch, asset_config);
    let path = dir.path();

    // Create the asset file
    std::fs::create_dir_all(path.join("dist")).expect("Failed to create dist dir");
    std::fs::write(
        path.join("dist/test-asset.txt"),
        "Hello from knope integration tests!",
    )
    .expect("Failed to write asset");

    // Re-commit to include the asset in git
    assert_git(path, &["add", "."]);
    assert_git(path, &["commit", "--amend", "--no-edit"]);

    // Push to remote
    assert_git(
        path,
        &["push", "--set-upstream", "origin", branch, "--force"],
    );
    assert_git(path, &["push", "origin", initial_tag, "--force"]);

    // Run knope release
    let output = Command::new(env!("CARGO_BIN_EXE_knope"))
        .current_dir(path)
        .env("GITHUB_TOKEN", &token)
        .args(["release"])
        .output()
        .expect("Failed to run knope");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "knope release with assets failed:\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Verify the release and asset exist
    let resp = client
        .get(format!(
            "https://api.github.com/repos/{owner}/{repo}/releases/tags/{expected_tag}"
        ))
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .expect("Failed to fetch release");

    assert!(resp.status().is_success(), "Release should exist on GitHub");

    let release: Release = resp.json().await.expect("Failed to deserialize release");

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
    assert!(
        assets.iter().any(|a| a.name == "test-asset.txt"),
        "Uploaded asset should appear in the release assets list"
    );

    // Cleanup
    delete_release(&client, &token, &owner, &repo, release.id).await;
    delete_tag(&client, &token, &owner, &repo, expected_tag).await;
    delete_tag(&client, &token, &owner, &repo, initial_tag).await;
    delete_branch(&client, &token, &owner, &repo, branch).await;
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

[github]
owner = "{owner}"
repo = "{repo}"
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
