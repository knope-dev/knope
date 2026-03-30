//! Integration tests that exercise Knope workflows against real GitHub and Gitea APIs.
//!
//! These tests are `#[ignore]`d by default and only run in dedicated CI jobs
//! where the required secrets (tokens, repo config) are available.
//!
//! Run with: `mise run integration-test`
//!
//! Required environment variables:
//!
//! **GitHub tests:**
//! - `KNOPE_INTEGRATION_GITHUB_TOKEN` — A GitHub personal access token with `repo` scope
//! - `KNOPE_INTEGRATION_GITHUB_OWNER` — Owner of the test repository (user or org)
//! - `KNOPE_INTEGRATION_GITHUB_REPO`  — Name of the test repository
//!
//! **Gitea tests:**
//! - `KNOPE_INTEGRATION_GITEA_TOKEN` — A Gitea/Codeberg personal access token
//! - `KNOPE_INTEGRATION_GITEA_HOST`  — Gitea instance URL (e.g. `https://codeberg.org`)
//! - `KNOPE_INTEGRATION_GITEA_OWNER` — Owner of the test repository
//! - `KNOPE_INTEGRATION_GITEA_REPO`  — Name of the test repository

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod integration_gitea;
mod integration_github;
