pub mod git;
pub mod gitea;
pub mod github;
pub(crate) mod http;

use http::ApiRequestError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct PullRequest {
    url: String,
    number: u32,
}

#[derive(Serialize)]
struct CreateReleaseInput<'a> {
    tag_name: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
    prerelease: bool,
    /// Whether to automatically generate the body for this release.
    /// If body is specified, the body will be pre-pended to the automatically generated notes.
    generate_release_notes: bool,
    /// true to create a draft (unpublished) release, false to create a published one.
    draft: bool,
    /// The commitish value that determines where the Git tag is created from.
    #[serde(skip_serializing_if = "Option::is_none")]
    target_commitish: Option<&'a str>,
}

impl<'a> CreateReleaseInput<'a> {
    fn new(
        tag_name: &'a str,
        name: &'a str,
        body: &'a str,
        prerelease: bool,
        draft: bool,
        target_commitish: Option<&'a str>,
    ) -> Self {
        let body = if body.is_empty() { None } else { Some(body) };
        Self {
            generate_release_notes: body.is_none(),
            tag_name,
            name,
            body,
            prerelease,
            draft,
            target_commitish,
        }
    }
}

#[derive(Deserialize)]
struct CreateReleaseResponse {
    url: String,
    upload_url: String,
}

#[derive(serde::Deserialize)]
struct ResponseIssue {
    number: usize,
    title: String,
}
