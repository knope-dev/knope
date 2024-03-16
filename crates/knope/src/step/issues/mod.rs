use std::fmt;

pub(crate) mod gitea;
pub(crate) mod github;
pub(crate) mod jira;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Issue {
    pub(crate) key: String,
    pub(crate) summary: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.summary)
    }
}
