use miette::Diagnostic;
pub(crate) use reqwest::Client;
use reqwest::Response;
use thiserror::Error;

/// Create a standard HTTP client to use for all HTTP requests.
pub(crate) fn http_client() -> Result<Client, ClientCreationError> {
    Client::builder()
        .user_agent("Knope")
        .build()
        .map_err(ClientCreationError)
}

#[derive(Debug, Diagnostic, Error)]
#[error("Failed to create client")]
#[diagnostic(help(
    "This is a bug, please report it at https://github.com/knope-dev/knope/issues/new"
))]
pub(crate) struct ClientCreationError(#[source] reqwest::Error);

pub async fn handle_response(
    response: Result<Response, reqwest::Error>,
    service: String,
    activity: String,
) -> Result<Response, ApiRequestError> {
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
    if let Err(status_err) = response.error_for_status_ref() {
        return Err(ApiRequestError {
            service,
            err: format!(
                "{status_err} with body: {}",
                response.text().await.unwrap_or_default()
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
        "There was a problem communicating with {service}, this may be a network issue or a permissions issue."
    )
)]
pub(crate) struct ApiRequestError {
    pub(crate) service: String,
    pub(crate) err: String,
    pub(crate) activity: String,
}
