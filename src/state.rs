use crate::workflow::Project;

#[derive(Default)]
pub struct State {
    pub projects: Vec<Project>,
    pub selected_issue_key: Option<String>,
}
