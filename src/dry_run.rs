use std::io::Write;

/// A helpful type to prevent all the duplication. When some, no file or network I/O should happen.
/// Instead, write what _would_ happen to stdout.
pub(crate) type DryRun<'a> = &'a mut Option<Box<dyn Write>>;

#[cfg(test)]
pub(crate) fn fake_dry_run() -> Option<Box<dyn Write>> {
    Some(Box::new(Vec::new()))
}
