/// Utilities for working with JSONC (JSON with comments).
#[must_use]
pub fn strip_json_comments(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if !escaped => {
                in_string = !in_string;
                result.push(ch);
            }
            '\\' if in_string => {
                escaped = !escaped;
                result.push(ch);
            }
            '/' if !in_string && !escaped => {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '/' => {
                            chars.next();
                            for ch in chars.by_ref() {
                                if ch == '\n' {
                                    result.push(ch);
                                    break;
                                }
                            }
                        }
                        '*' => {
                            chars.next();
                            while let Some(ch) = chars.next() {
                                if ch == '*' {
                                    if let Some(&'/') = chars.peek() {
                                        chars.next();
                                        break;
                                    }
                                }
                            }
                        }
                        _ => result.push(ch),
                    }
                } else {
                    result.push(ch);
                }
            }
            _ => {
                result.push(ch);
                if ch != '\\' {
                    escaped = false;
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::strip_json_comments;

    #[test]
    fn removes_line_comments() {
        let input = "{\n  // comment\n  \"key\": \"value\"\n}";
        let expected = "{\n  \n  \"key\": \"value\"\n}";
        assert_eq!(strip_json_comments(input), expected);
    }

    #[test]
    fn removes_block_comments() {
        let input = "{/* comment */\n\"key\": \"value\"\n}";
        let expected = "{\n\"key\": \"value\"\n}";
        assert_eq!(strip_json_comments(input), expected);
    }

    #[test]
    fn preserves_comments_inside_strings() {
        let input = "{\n  \"regex\": \"/\\\\/\"\n}";
        assert_eq!(strip_json_comments(input), input);
    }
}
