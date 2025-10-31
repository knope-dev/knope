use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ureq::{Body, http};

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
    fn new(tag_name: &'a str, name: &'a str, body: &'a str, prerelease: bool, draft: bool) -> Self {
        let body = if body.is_empty() { None } else { Some(body) };
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

fn handle_response(
    response: Result<http::Response<Body>, ureq::Error>,
    service: String,
    activity: String,
) -> Result<http::Response<Body>, ApiRequestError> {
    let response = match response {
        Ok(response) => response,
        Err(source) => {
            return Err(ApiRequestError {
                service,
                err: source.to_string(),
                activity,
            });
        }
    };
    let status = response.status().as_u16();
    if status >= 400 {
        return Err(ApiRequestError {
            service,
            err: format!(
                "Got HTTP status {status} with body: {}",
                response.into_body().read_to_string().unwrap_or_default()
            ),
            activity,
        });
    }
    Ok(response)
}

#[derive(Clone, Debug, Diagnostic, Error)]
#[error("Trouble communicating with {service} while {activity}: {err}")]
#[diagnostic(
    code(api_request_error),
    help(
        "There was a problem communicating with GitHub, this may be a network issue or a permissions issue."
    )
)]
pub(crate) struct ApiRequestError {
    service: String,
    err: String,
    activity: String,
}
