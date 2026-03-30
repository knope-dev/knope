//! Integration tests for Gitea API interactions.
//!
//! These tests verify that the reqwest-based HTTP client can correctly:
//! - Create and delete releases via a Gitea instance API
//! - Create and close pull requests
//! - List issues with query-parameter–based authentication
//!
//! All tests clean up after themselves by deleting any resources they create.

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct Release {
    id: u64,
    tag_name: String,
    name: String,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct PullRequest {
    number: u64,
    title: String,
    body: String,
}

#[derive(Debug, Deserialize)]
struct Branch {
    commit: BranchCommit,
}

#[derive(Debug, Deserialize)]
struct BranchCommit {
    id: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Issue {
    number: u64,
    title: String,
}

fn gitea_env() -> Option<(String, String, String, String, Client)> {
    let token = std::env::var("KNOPE_INTEGRATION_GITEA_TOKEN").ok()?;
    let host = std::env::var("KNOPE_INTEGRATION_GITEA_HOST").ok()?;
    let owner = std::env::var("KNOPE_INTEGRATION_GITEA_OWNER").ok()?;
    let repo = std::env::var("KNOPE_INTEGRATION_GITEA_REPO").ok()?;
    let client = Client::builder()
        .user_agent("Knope")
        .build()
        .expect("Failed to build HTTP client");
    Some((token, host, owner, repo, client))
}

fn skip_unless_configured() -> (String, String, String, String, Client) {
    gitea_env().expect(
        "Skipping: set KNOPE_INTEGRATION_GITEA_TOKEN, KNOPE_INTEGRATION_GITEA_HOST, \
         KNOPE_INTEGRATION_GITEA_OWNER, and KNOPE_INTEGRATION_GITEA_REPO to run Gitea \
         integration tests",
    )
}

fn api_base(host: &str, owner: &str, repo: &str) -> String {
    format!("{host}/api/v1/repos/{owner}/{repo}")
}

/// Helper to delete a Gitea release by its ID. Best-effort cleanup.
async fn delete_release(client: &Client, token: &str, base: &str, release_id: u64) {
    let url = format!("{base}/releases/{release_id}");
    let _ = client
        .delete(&url)
        .query(&[("access_token", token)])
        .send()
        .await;
}

/// Helper to delete a git tag on Gitea. Best-effort cleanup.
async fn delete_tag(client: &Client, token: &str, base: &str, tag: &str) {
    let url = format!("{base}/tags/{tag}");
    let _ = client
        .delete(&url)
        .query(&[("access_token", token)])
        .send()
        .await;
}

/// Helper to close a Gitea pull request. Best-effort cleanup.
async fn close_pull_request(client: &Client, token: &str, base: &str, pr_number: u64) {
    let url = format!("{base}/pulls/{pr_number}");
    let _ = client
        .patch(&url)
        .header("Accept", "application/json")
        .query(&[("access_token", token)])
        .json(&json!({"state": "closed"}))
        .send()
        .await;
}

/// Helper to delete a branch on Gitea. Best-effort cleanup.
async fn delete_branch(client: &Client, token: &str, base: &str, branch: &str) {
    let url = format!("{base}/branches/{branch}");
    let _ = client
        .delete(&url)
        .query(&[("access_token", token)])
        .send()
        .await;
}

/// Clean up any leftover release matching the given tag. Best-effort.
async fn cleanup_release_by_tag(client: &Client, token: &str, base: &str, tag: &str) {
    let releases_url = format!("{base}/releases");
    if let Ok(resp) = client
        .get(&releases_url)
        .query(&[("access_token", token)])
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(releases) = resp.json::<Vec<Release>>().await {
                for r in &releases {
                    if r.tag_name == tag {
                        delete_release(client, token, base, r.id).await;
                    }
                }
            }
        }
    }
    delete_tag(client, token, base, tag).await;
}

/// Verify that we can create a release on Gitea, read it back, then delete it.
///
/// This exercises the same JSON serialization and query-parameter auth used in
/// `crates/knope/src/integrations/gitea/create_release.rs`.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn gitea_create_and_delete_release() {
    let (token, host, owner, repo, client) = skip_unless_configured();
    let base = api_base(&host, &owner, &repo);
    let tag = "integration-test-release-v0.0.0";
    let release_name = "Integration Test Release";

    // Clean up leftovers from a previous failed run
    cleanup_release_by_tag(&client, &token, &base, tag).await;

    // --- Create release (matches the production JSON structure) ---
    let releases_url = format!("{base}/releases");
    let create_body = json!({
        "tag_name": tag,
        "name": release_name,
        "body": "Automated integration test release — safe to delete.",
        "prerelease": true,
        "draft": false,
    });

    let resp = client
        .post(&releases_url)
        .query(&[("access_token", token.as_str())])
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

    // Re-fetch and verify
    let resp = client
        .get(&releases_url)
        .query(&[("access_token", token.as_str())])
        .send()
        .await
        .unwrap();

    let releases: Vec<Release> = resp.json().await.unwrap();
    let created = releases
        .iter()
        .find(|r| r.tag_name == tag)
        .expect("Created release should appear in the list");

    assert_eq!(created.name, release_name);
    assert!(created.prerelease);

    // --- Cleanup ---
    delete_release(&client, &token, &base, created.id).await;
    delete_tag(&client, &token, &base, tag).await;
}

/// Verify pull request creation and update on Gitea.
///
/// This test requires the test repo to have a default branch (e.g., `main`)
/// with at least one commit. It creates a branch, opens a PR, updates it,
/// then closes and cleans up.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn gitea_create_and_update_pull_request() {
    let (token, host, owner, repo, client) = skip_unless_configured();
    let base = api_base(&host, &owner, &repo);
    let test_branch = "integration-test-pr-branch";

    // Clean up leftover branch/PR
    delete_branch(&client, &token, &base, test_branch).await;

    // Get default branch to verify it exists
    let branch_url = format!("{base}/branches/main");
    let branch_data: Branch = client
        .get(&branch_url)
        .query(&[("access_token", token.as_str())])
        .send()
        .await
        .expect("Network error fetching main branch")
        .error_for_status()
        .expect("Failed to get main branch (does the test repo have a 'main' branch?)")
        .json()
        .await
        .expect("Should deserialize branch data");
    assert!(
        !branch_data.commit.id.is_empty(),
        "Branch should have a commit SHA"
    );

    // Create a test branch via the Gitea API
    let create_branch_url = format!("{base}/branches");
    let resp = client
        .post(&create_branch_url)
        .query(&[("access_token", token.as_str())])
        .json(&json!({
            "new_branch_name": test_branch,
            "old_branch_name": "main",
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
    let pulls_url = format!("{base}/pulls");
    let resp = client
        .post(&pulls_url)
        .header("Accept", "application/json")
        .query(&[("access_token", token.as_str())])
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

    // Update the PR (same PATCH pattern as production code)
    let pr_url = format!("{base}/pulls/{pr_number}");
    let resp = client
        .patch(&pr_url)
        .header("Accept", "application/json")
        .query(&[("access_token", token.as_str())])
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
        .header("Accept", "application/json")
        .query(&[("access_token", token.as_str())])
        .send()
        .await
        .unwrap();

    let updated_pr: PullRequest = resp.json().await.expect("Should deserialize updated PR");
    assert_eq!(updated_pr.title, "Integration Test PR (Updated)");
    assert_eq!(updated_pr.body, "Updated body from integration test.");

    // --- Cleanup ---
    close_pull_request(&client, &token, &base, pr_number).await;
    delete_branch(&client, &token, &base, test_branch).await;
}

/// Verify that listing existing pull requests with query filters works.
///
/// This exercises the same GET with query parameters pattern used in the
/// production code to check for existing PRs before creating.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn gitea_list_pull_requests() {
    let (token, host, owner, repo, client) = skip_unless_configured();
    let base = api_base(&host, &owner, &repo);

    let pulls_url = format!("{base}/pulls");
    let resp = client
        .get(&pulls_url)
        .header("Accept", "application/json")
        .query(&[
            ("access_token", token.as_str()),
            ("state", "closed"),
            ("limit", "1"),
        ])
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Listing PRs failed: {}",
        resp.text().await.unwrap_or_default()
    );

    let pulls: Vec<PullRequest> = resp.json().await.unwrap();
    assert!(pulls.len() <= 1, "limit=1 should return at most 1 result");
}

/// Verify that listing issues with query-parameter auth and filters works.
///
/// This exercises the same pattern used in
/// `crates/knope/src/integrations/gitea/list_issues.rs`.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn gitea_list_issues() {
    let (token, host, owner, repo, client) = skip_unless_configured();
    let base = api_base(&host, &owner, &repo);

    let issues_url = format!("{base}/issues");
    let resp = client
        .get(&issues_url)
        .header("Accept", "application/json")
        .query(&[
            ("access_token", token.as_str()),
            ("state", "open"),
            ("limit", "30"),
        ])
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_success(),
        "Listing issues failed with status {}: {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );

    // Verify the response deserializes as a list of issues (may be empty).
    let _issues: Vec<Issue> = resp
        .json()
        .await
        .expect("Should deserialize as a list of issues");
}

/// Verify HTTP error handling for bad authentication on Gitea.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn gitea_error_handling() {
    let (_token, host, owner, repo, client) = skip_unless_configured();
    let base = api_base(&host, &owner, &repo);

    // Request with a bad token — Gitea should reject this.
    let issues_url = format!("{base}/issues");
    let resp = client
        .get(&issues_url)
        .header("Accept", "application/json")
        .query(&[("access_token", "bad-token-value"), ("state", "open")])
        .send()
        .await
        .unwrap();

    // Gitea returns 401 for invalid tokens
    assert_eq!(
        resp.status().as_u16(),
        401,
        "Using an invalid token should return 401, got {}",
        resp.status()
    );
}
