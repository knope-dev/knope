use std::borrow::Cow;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// A template string and the variables that should be replaced in it.
pub struct Template {
    template: String,
    #[serde(default = "default_variables")]
    variables: IndexMap<Cow<'static, str>, Variable>,
}

impl Template {
    pub fn new(template: String, variables: Option<IndexMap<Cow<'static, str>, Variable>>) -> Self {
        Self {
            template,
            variables: variables.unwrap_or_else(default_variables),
        }
    }

    /// Return the template with all configured variables replaced.
    ///
    /// # Errors
    /// - If the `lookup` function errors.
    pub fn replace_variables<Lookup, Error>(&self, mut lookup: Lookup) -> Result<String, Error>
    where
        Lookup: FnMut(Variable) -> Result<String, Error>,
    {
        let mut res = self.template.clone();
        for (var_name, var_type) in &self.variables {
            if !res.contains(var_name.as_ref()) {
                continue; // Don't error for invalid variables that aren't used
            }
            let var_value = lookup(*var_type)?;
            res = res.replace(var_name.as_ref(), &var_value);
        }
        Ok(res)
    }
}

fn default_variables() -> IndexMap<Cow<'static, str>, Variable> {
    [
        (Cow::Borrowed("$version"), Variable::Version),
        (Cow::Borrowed("$changelog"), Variable::ChangelogEntry),
    ]
    .into_iter()
    .collect()
}

/// Describes a value that can replace an arbitrary string in certain steps.
///
/// <https://knope.tech/reference/config-file/variables//>
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Variable {
    /// The version of the package, if only a single package is configured (error if multiple).
    Version,
    /// The generated branch name for the selected issue. Note that this means the workflow must
    /// already be in [`state::Issue::Selected`] when this variable is used.
    IssueBranch,
    /// Get the current changelog entry from the latest release.
    ChangelogEntry,
}
