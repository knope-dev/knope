use serde::{Deserialize, Serialize};

pub mod git;
pub mod gitea;
pub mod github;

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
}

impl<'a> CreateReleaseInput<'a> {
    fn new(
        tag_name: &'a str,
        name: &'a str,
        body: Option<&'a str>,
        prerelease: bool,
        draft: bool,
    ) -> Self {
        Self {
            generate_release_notes: body.is_none(),
            tag_name,
            name,
            body,
            prerelease,
            draft,
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

fn ureq_err_to_string(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, response) => {
            format!("{}: {}", code, response.into_string().unwrap_or_default())
        }
        ureq::Error::Transport(err) => format!("Transport error: {err}"),
    }
}
