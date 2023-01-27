use crate::app_config::get_or_prompt_for_github_token;
use crate::issues::Issue;
use crate::step::StepError;
use crate::{config, state};

const ISSUES_QUERY: &str = r##"
query($repo: String!, $owner: String!, $labels: [String!]) { 
  repository(name:$repo, owner:$owner) { 
    issues(states:OPEN, first: 30, labels: $labels) {
      nodes {
        number,
        title
      }
    }
  }
}
"##;

#[derive(serde::Deserialize)]
struct ResponseIssue {
    number: usize,
    title: String,
}

pub(crate) fn list_issues(
    github_config: &config::GitHub,
    github_state: state::GitHub,
    labels: Option<&[String]>,
) -> Result<(state::GitHub, Vec<Issue>), StepError> {
    let token = match github_state {
        state::GitHub::Initialized { token } => token,
        state::GitHub::New => get_or_prompt_for_github_token()?,
    };
    let response = ureq::post("https://api.github.com/graphql")
        .set("Authorization", &format!("bearer {token}"))
        .send_json(ureq::json!({
            "query": ISSUES_QUERY,
            "variables": {
                "repo": github_config.repo,
                "owner": github_config.owner,
                "labels": labels
            }
        }))
        .or(Err(StepError::ApiRequestError))?;

    let gh_issues = decode_github_response(response)?;

    let issues = gh_issues
        .into_iter()
        .map(|gh_issue| Issue {
            key: gh_issue.number.to_string(),
            summary: gh_issue.title,
        })
        .collect();

    Ok((state::GitHub::Initialized { token }, issues))
}

fn decode_github_response(response: ureq::Response) -> Result<Vec<ResponseIssue>, StepError> {
    let json_value: serde_json::Value = response.into_json()?;
    let json_issues = json_value.pointer("/data/repository/issues/nodes");
    match json_issues {
        Some(value) => {
            serde_json::from_value(value.clone()).map_err(|e| StepError::ApiResponseError(Some(e)))
        }
        None => Err(StepError::ApiResponseError(None)),
    }
}
