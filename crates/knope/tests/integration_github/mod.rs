//! Integration tests for GitHub API interactions.
//!
//! These tests verify that the reqwest-based HTTP client can correctly:
//! - Create and delete releases via the GitHub API
//! - Upload binary assets to GitHub releases
//! - Create and close pull requests
//!
//! All tests clean up after themselves by deleting any resources they create.

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct Release {
    id: u64,
    tag_name: String,
    name: Option<String>,
    prerelease: bool,
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
}

#[derive(Debug, Deserialize)]
struct PullRequest {
    number: u64,
    title: String,
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitRef {
    object: GitObject,
}

#[derive(Debug, Deserialize)]
struct GitObject {
    sha: String,
}

fn github_env() -> Option<(String, String, String, Client)> {
    let token = std::env::var("KNOPE_INTEGRATION_GITHUB_TOKEN").ok()?;
    let owner = std::env::var("KNOPE_INTEGRATION_GITHUB_OWNER").ok()?;
    let repo = std::env::var("KNOPE_INTEGRATION_GITHUB_REPO").ok()?;
    let client = Client::builder()
        .user_agent("Knope")
        .build()
        .expect("Failed to build HTTP client");
    Some((token, owner, repo, client))
}

fn skip_unless_configured() -> (String, String, String, Client) {
    github_env().expect(
        "Skipping: set KNOPE_INTEGRATION_GITHUB_TOKEN, KNOPE_INTEGRATION_GITHUB_OWNER, \
         and KNOPE_INTEGRATION_GITHUB_REPO to run GitHub integration tests",
    )
}

/// Helper to delete a GitHub release by its ID. Best-effort cleanup.
async fn delete_release(client: &Client, token: &str, owner: &str, repo: &str, release_id: u64) {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/{release_id}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;
}

/// Helper to delete a git tag. Best-effort cleanup.
async fn delete_tag(client: &Client, token: &str, owner: &str, repo: &str, tag: &str) {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/git/refs/tags/{tag}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;
}

/// Helper to close a pull request. Best-effort cleanup.
async fn close_pull_request(
    client: &Client,
    token: &str,
    owner: &str,
    repo: &str,
    pr_number: u64,
) {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls/{pr_number}");
    let _ = client
        .patch(&url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .json(&json!({"state": "closed"}))
        .send()
        .await;
}

/// Helper to delete a branch. Best-effort cleanup.
async fn delete_branch(client: &Client, token: &str, owner: &str, repo: &str, branch: &str) {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/git/refs/heads/{branch}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;
}

/// Clean up any leftover release matching the given tag. Best-effort.
async fn cleanup_release_by_tag(
    client: &Client,
    token: &str,
    owner: &str,
    repo: &str,
    tag: &str,
) {
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

/// Verify that we can create a release on GitHub, read it back, then delete it.
///
/// This exercises the same JSON serialization and auth header format used in
/// `crates/knope/src/integrations/github/create_release.rs`.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn github_create_and_delete_release() {
    let (token, owner, repo, client) = skip_unless_configured();
    let tag = "integration-test-release-v0.0.0";
    let release_name = "Integration Test Release";

    // Clean up any leftover release/tag from a previous failed run.
    cleanup_release_by_tag(&client, &token, &owner, &repo, tag).await;

    // --- Create release ---
    let create_url = format!("https://api.github.com/repos/{owner}/{repo}/releases");
    let create_body = json!({
        "tag_name": tag,
        "name": release_name,
        "body": "Automated integration test release — safe to delete.",
        "prerelease": true,
        "draft": false,
        "generate_release_notes": false,
    });

    let resp = client
        .post(&create_url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .json(&create_body)
        .send()
        .await
        .expect("Failed to send create-release request");

    assert!(
        resp.status().is_success(),
        "Create release failed with status {}: {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );

    // Re-fetch and verify via the tag endpoint
    let tag_url = format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}");
    let resp = client
        .get(&tag_url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .unwrap();

    let created: Release = resp.json().await.expect("Should deserialize release");
    assert_eq!(created.tag_name, tag);
    assert_eq!(created.name.as_deref(), Some(release_name));
    assert!(created.prerelease);

    // --- Cleanup ---
    delete_release(&client, &token, &owner, &repo, created.id).await;
    delete_tag(&client, &token, &owner, &repo, tag).await;
}

/// Verify that binary asset upload works correctly (using raw body, not JSON).
///
/// This exercises the same upload path used in `create_release.rs` where we
/// use `.body(file)` with `application/octet-stream`.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn github_upload_release_asset() {
    let (token, owner, repo, client) = skip_unless_configured();
    let tag = "integration-test-asset-v0.0.0";

    // Clean up leftovers
    cleanup_release_by_tag(&client, &token, &owner, &repo, tag).await;

    // Create a draft release (same pattern as production code with assets)
    let create_url = format!("https://api.github.com/repos/{owner}/{repo}/releases");
    let resp = client
        .post(&create_url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .json(&json!({
            "tag_name": tag,
            "name": "Asset Upload Test",
            "body": "Integration test for asset upload.",
            "draft": true,
            "prerelease": true,
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Create draft release failed: {}",
        resp.text().await.unwrap_or_default()
    );

    // Re-fetch to get upload_url.
    // Draft releases aren't accessible via the tags endpoint, so use the releases list.
    let resp = client
        .get(&create_url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .unwrap();

    let releases: Vec<Release> = resp.json().await.unwrap();
    let created = releases
        .into_iter()
        .find(|r| r.tag_name == tag)
        .expect("Draft release should be in the list");

    let release_id = created.id;

    // The upload_url is a URI template like:
    // https://uploads.github.com/repos/{owner}/{repo}/releases/{id}/assets{?name,label}
    // We need to replace the template part with an actual query parameter.
    let upload_url = created
        .upload_url
        .split('{')
        .next()
        .unwrap_or(&created.upload_url);
    let upload_url = format!("{upload_url}?name=test-asset.txt");

    let test_content = b"Hello from knope integration tests!";
    let resp = client
        .post(&upload_url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", test_content.len().to_string())
        .body(test_content.to_vec())
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Asset upload failed with status {}: {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );

    // Verify asset was uploaded by checking the release assets
    let assets_url =
        format!("https://api.github.com/repos/{owner}/{repo}/releases/{release_id}/assets");
    let resp = client
        .get(&assets_url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .unwrap();

    let assets: Vec<Asset> = resp.json().await.unwrap();
    assert!(
        assets.iter().any(|a| a.name == "test-asset.txt"),
        "Uploaded asset should appear in the release assets list"
    );

    // Publish the draft release (same as production code)
    let release_url =
        format!("https://api.github.com/repos/{owner}/{repo}/releases/{release_id}");
    let resp = client
        .patch(&release_url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .json(&json!({"draft": false}))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Publishing release failed: {}",
        resp.text().await.unwrap_or_default()
    );

    // --- Cleanup ---
    delete_release(&client, &token, &owner, &repo, release_id).await;
    delete_tag(&client, &token, &owner, &repo, tag).await;
}

/// Verify pull request creation and update on GitHub.
///
/// This test requires the test repo to have a default branch (e.g., `main`)
/// with at least one commit. The test creates a lightweight branch, opens a PR,
/// verifies the PR was created, then closes and cleans up.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn github_create_and_update_pull_request() {
    let (token, owner, repo, client) = skip_unless_configured();
    let test_branch = "integration-test-pr-branch";
    let auth = format!("token {token}");

    // Clean up any leftover branch/PR from a previous run
    delete_branch(&client, &token, &owner, &repo, test_branch).await;

    // Get the default branch's HEAD SHA
    let refs_url = format!("https://api.github.com/repos/{owner}/{repo}/git/ref/heads/main");
    let ref_data: GitRef = client
        .get(&refs_url)
        .header("Authorization", &auth)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .expect("Network error fetching main branch ref")
        .error_for_status()
        .expect("Failed to get main branch ref (does the test repo have a 'main' branch?)")
        .json()
        .await
        .expect("Should deserialize git ref");

    // Create the test branch pointing at the same commit
    let create_ref_url = format!("https://api.github.com/repos/{owner}/{repo}/git/refs");
    let resp = client
        .post(&create_ref_url)
        .header("Authorization", &auth)
        .header("Accept", "application/vnd.github+json")
        .json(&json!({
            "ref": format!("refs/heads/{test_branch}"),
            "sha": ref_data.object.sha,
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Failed to create test branch: {}",
        resp.text().await.unwrap_or_default()
    );

    // Create a PR from the test branch to main
    let pulls_url = format!("https://api.github.com/repos/{owner}/{repo}/pulls");
    let resp = client
        .post(&pulls_url)
        .header("Authorization", &auth)
        .header("Accept", "application/vnd.github+json")
        .json(&json!({
            "title": "Integration Test PR",
            "body": "Automated integration test PR — safe to close.",
            "head": test_branch,
            "base": "main",
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Failed to create PR: {}",
        resp.text().await.unwrap_or_default()
    );

    let pr: PullRequest = resp.json().await.expect("Should deserialize PR");
    let pr_number = pr.number;
    let pr_url = format!("https://api.github.com/repos/{owner}/{repo}/pulls/{pr_number}");

    // Update the PR title and body (same PATCH pattern as production code)
    let resp = client
        .patch(&pr_url)
        .header("Authorization", &auth)
        .header("Accept", "application/vnd.github+json")
        .json(&json!({
            "title": "Integration Test PR (Updated)",
            "body": "Updated body from integration test.",
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Failed to update PR: {}",
        resp.text().await.unwrap_or_default()
    );

    // Verify the update took effect
    let resp = client
        .get(&pr_url)
        .header("Authorization", &auth)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .unwrap();

    let updated_pr: PullRequest = resp.json().await.expect("Should deserialize updated PR");
    assert_eq!(updated_pr.title, "Integration Test PR (Updated)");
    assert_eq!(
        updated_pr.body.as_deref(),
        Some("Updated body from integration test.")
    );

    // --- Cleanup ---
    close_pull_request(&client, &token, &owner, &repo, pr_number).await;
    delete_branch(&client, &token, &owner, &repo, test_branch).await;
}

/// Verify that listing pull requests with query filters works.
///
/// This exercises the same GET with query parameters pattern used to check for
/// existing PRs before creating new ones.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn github_list_pull_requests() {
    let (token, owner, repo, client) = skip_unless_configured();

    let pulls_url = format!("https://api.github.com/repos/{owner}/{repo}/pulls");
    let resp = client
        .get(&pulls_url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .query(&[("state", "closed"), ("per_page", "1")])
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Listing PRs failed: {}",
        resp.text().await.unwrap_or_default()
    );

    // We just verify the response deserializes as a JSON array of PRs
    let pulls: Vec<PullRequest> = resp.json().await.unwrap();
    // The test repo might have 0 closed PRs, that's fine — we just check it's an array.
    assert!(pulls.len() <= 1, "per_page=1 should return at most 1 result");
}

/// Verify HTTP error handling returns appropriate status codes.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn github_error_handling() {
    let (token, owner, repo, client) = skip_unless_configured();

    // Request a release that doesn't exist — should get 404.
    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/tags/nonexistent-tag-12345"
    );
    let resp = client
        .get(&url)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status().as_u16(),
        404,
        "Requesting a nonexistent release should return 404"
    );

    // Request with a bad token — should get 401.
    let releases_url = format!("https://api.github.com/repos/{owner}/{repo}/releases");
    let resp = client
        .get(releases_url)
        .header("Authorization", "token bad-token-value")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status().as_u16(),
        401,
        "Using an invalid token should return 401"
    );
}
