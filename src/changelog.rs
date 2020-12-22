use markdown::{generate_markdown, tokenize, Block, ListItem, Span};

#[derive(Debug)]
pub(crate) struct Changelog {
    header: Vec<Block>,
    rest: Vec<Block>,
}

impl Changelog {
    pub(crate) fn into_markdown(self) -> String {
        let Self { header, rest } = self;
        let blocks = header.into_iter().chain(rest.into_iter()).collect();
        generate_markdown(blocks)
    }

    pub(crate) fn from_markdown(text: &str) -> Self {
        let blocks = tokenize(text);
        let (header, rest) = parse_header(blocks);

        Self { header, rest }
    }

    pub(crate) fn add_version(self, version: Version) -> Self {
        let Self { header, rest } = self;
        Self {
            header,
            rest: version
                .into_markdown_blocks()
                .into_iter()
                .chain(rest.into_iter())
                .collect(),
        }
    }
}

fn parse_header(mut blocks: Vec<Block>) -> (Vec<Block>, Vec<Block>) {
    let end_index = blocks.iter().enumerate().find_map(|(idx, block)| {
        if matches!(block, Block::Header(_, 2)) {
            Some(idx)
        } else {
            None
        }
    });
    match end_index {
        Some(index) => {
            let rest = blocks.split_off(index);
            (blocks, rest)
        }
        None => (blocks, Vec::new()),
    }
}

#[derive(Clone)]
pub(crate) struct Version {
    pub(crate) title: String,
    pub(crate) fixes: Vec<String>,
    pub(crate) features: Vec<String>,
    pub(crate) breaking_changes: Vec<String>,
}

impl Version {
    fn into_markdown_blocks(self) -> Vec<Block> {
        let headers_size = 4;
        let Self {
            title,
            fixes,
            features,
            breaking_changes,
        } = self;
        let mut blocks = Vec::with_capacity(
            fixes.len() + features.len() + breaking_changes.len() + headers_size,
        );

        blocks.push(header_block(title, 2));
        if !breaking_changes.is_empty() {
            blocks.push(header_block("Breaking Changes".to_string(), 3));
            blocks.push(unordered_list(breaking_changes));
        }
        if !features.is_empty() {
            blocks.push(header_block("Features".to_string(), 3));
            blocks.push(unordered_list(features));
        }
        if !fixes.is_empty() {
            blocks.push(header_block("Fixes".to_string(), 3));
            blocks.push(unordered_list(fixes));
        }
        blocks
    }
}

fn header_block(text: String, level: usize) -> Block {
    Block::Header(vec![Span::Text(text)], level)
}

fn unordered_list(items: Vec<String>) -> Block {
    Block::UnorderedList(
        items
            .into_iter()
            .map(|note| ListItem::Simple(vec![Span::Text(note)]))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use markdown::{generate_markdown, Block, ListItem, Span};

    #[test]
    fn changelog_from_markdown() {
        let markdown = r##"
# Changelog
Some details about the keepachangelog format

Sometimes a second paragraph

## 0.1.0 - 2020-12-25
### Features
- Initial version

[link]: some footer details
"##;
        let changelog = super::Changelog::from_markdown(markdown);
        println!("{:#?}", changelog);
        assert_eq!(changelog.header.len(), 3);
        assert_eq!(changelog.rest.len(), 4);
    }

    #[test]
    fn changelog_into_markdown() {
        let expected = r##"# Changelog

Some details about the keepachangelog format

## 0.1.0 - 2020-12-25

### Features

* Initial version

[link]: some footer details"##;
        let changelog = super::Changelog {
            header: vec![
                Block::Header(vec![Span::Text("Changelog".to_string())], 1),
                Block::Paragraph(vec![Span::Text(
                    "Some details about the keepachangelog format".to_string(),
                )]),
            ],
            rest: vec![
                Block::Header(vec![Span::Text("0.1.0 - 2020-12-25".to_string())], 2),
                Block::Header(vec![Span::Text("Features".to_string())], 3),
                Block::UnorderedList(vec![ListItem::Simple(vec![Span::Text(
                    "Initial version".to_string(),
                )])]),
                Block::Paragraph(vec![Span::Text("[link]: some footer details".to_string())]),
            ],
        };
        assert_eq!(changelog.into_markdown(), expected);
    }

    #[test]
    fn changelog_add_version() {
        let markdown = r##"
# Changelog
Some details about the keepachangelog format

Sometimes a second paragraph

## 0.1.0 - 2020-12-25
### Features
- Initial version

[link]: some footer details
"##;
        let changelog = super::Changelog::from_markdown(markdown);
        let version = super::Version {
            title: "0.2.0 - 2020-12-31".to_string(),
            fixes: vec!["Fixed something".to_string()],
            features: vec![],
            breaking_changes: vec![],
        };
        let changelog = changelog.add_version(version.clone());
        assert_eq!(changelog.rest.len(), 7);
        assert_eq!(changelog.rest[0], version.into_markdown_blocks()[0])
    }

    #[test]
    fn version_into_blocks() {
        let version = super::Version {
            title: "0.2.0 - 2020-12-31".to_string(),
            fixes: vec![
                "Fixed something".to_string(),
                "Fixed something else".to_string(),
            ],
            features: vec![
                "Added something".to_string(),
                "Added something else".to_string(),
            ],
            breaking_changes: vec![
                "Broke something".to_string(),
                "Broke something else".to_string(),
            ],
        };
        let expected = r##"## 0.2.0 - 2020-12-31

### Breaking Changes

* Broke something
* Broke something else

### Features

* Added something
* Added something else

### Fixes

* Fixed something
* Fixed something else"##;

        let blocks = version.into_markdown_blocks();
        assert_eq!(generate_markdown(blocks), expected);
    }
}
