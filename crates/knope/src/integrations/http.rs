use miette::Diagnostic;
use reessaie::{
    RetryAfterMiddleware, RetryAfterPolicy,
    reqwest_retry::{
        DefaultRetryableStrategy, Retryable, RetryableStrategy, policies::ExponentialBackoff,
    },
};
use reqwest::{Response, StatusCode, header::RETRY_AFTER};
use reqwest_middleware::ClientBuilder;
use thiserror::Error;
use tracing::warn;

pub(crate) type Client = reqwest_middleware::ClientWithMiddleware;
pub(crate) type Error = reqwest_middleware::Error;

/// Create a standard HTTP client to use for all HTTP requests.
pub(crate) fn http_client() -> Result<Client, ClientCreationError> {
    let client = reqwest::Client::builder()
        .user_agent("Knope")
        .build()
        .map_err(ClientCreationError)?;
    let retry_policy = RetryAfterPolicy::with_policy_and_strategy(
        ExponentialBackoff::builder().build_with_max_retries(5),
        RateLimitLoggingStrategy,
    );
    Ok(ClientBuilder::new(client)
        .with(RetryAfterMiddleware::new_with_policy(retry_policy))
        .build())
}

#[derive(Debug, Diagnostic, Error)]
#[error("Failed to create client")]
#[diagnostic(help(
    "This is a bug, please report it at https://github.com/knope-dev/knope/issues/new"
))]
pub(crate) struct ClientCreationError(#[source] reqwest::Error);

pub async fn handle_response(
    response: Result<Response, reqwest_middleware::Error>,
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
    handle_status(response, service, activity).await
}

async fn handle_status(
    response: Response,
    service: String,
    activity: String,
) -> Result<Response, ApiRequestError> {
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

#[derive(Clone, Copy, Debug, Default)]
struct RateLimitLoggingStrategy;

impl RetryableStrategy for RateLimitLoggingStrategy {
    fn handle(&self, res: &Result<Response, reqwest_middleware::Error>) -> Option<Retryable> {
        if let Ok(response) = res
            && response.status() == StatusCode::TOO_MANY_REQUESTS
            && let Some(retry_after) = response.headers().get(RETRY_AFTER)
            && let Ok(retry_after) = retry_after.to_str()
        {
            if let Ok(delay_secs) = retry_after.parse::<u64>() {
                warn!("API rate limited; retrying in {delay_secs} seconds");
            } else {
                warn!("API rate limited; retrying after {retry_after}");
            }
        }
        DefaultRetryableStrategy.handle(res)
    }
}
