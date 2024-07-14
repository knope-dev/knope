use relative_path::RelativePathBuf;

/// Actions to take to finish updating a package
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    WriteToFile {
        path: RelativePathBuf,
        content: String,
    },
    RemoveFile {
        path: RelativePathBuf,
    },
    AddTag {
        tag: String,
    },
}

pub(crate) enum ActionSet {
    Single(Action),
    Two([Action; 2]),
}

impl IntoIterator for ActionSet {
    type Item = Action;
    type IntoIter = ActionSetIter;

    fn into_iter(self) -> Self::IntoIter {
        ActionSetIter {
            actions: Some(self),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct ActionSetIter {
    actions: Option<ActionSet>,
}

impl Iterator for ActionSetIter {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        match self.actions.take() {
            None => None,
            Some(ActionSet::Single(action)) => {
                self.actions = None;
                Some(action)
            }
            Some(ActionSet::Two([first, second])) => {
                self.actions = None;
                self.actions = Some(ActionSet::Single(second));
                Some(first)
            }
        }
    }
}
