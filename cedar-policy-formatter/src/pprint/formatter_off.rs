/*
 * Copyright Cedar Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! `// @formatter:off` and `// @formatter:on` comment directives (#1559).
//!
//! A policy that starts after an `off` directive, with no `on` directive
//! between them, is copied from the input unchanged: its leading comments, the
//! policy text, and a comment at the end of its last line. An `off` directive
//! without a matching `on` lasts to the end of the input. Directives inside a
//! policy take effect from the next policy.

/// The directives in `src`, in source order, as the byte offset of the comment
/// and whether it turns formatting off.
fn directives(src: &str) -> Vec<(usize, bool)> {
    let bytes = src.as_bytes();
    let mut directives = Vec::new();
    let mut in_string = false;
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        match (in_string, byte) {
            // skip the escaped character
            (true, b'\\') => i += 2,
            (_, b'"') => {
                in_string = !in_string;
                i += 1;
            }
            (false, b'/') if bytes.get(i + 1) == Some(&b'/') => {
                let end = line_end(src, i);
                match src.get(i + 2..end).map(str::trim) {
                    Some("@formatter:off") => directives.push((i, true)),
                    Some("@formatter:on") => directives.push((i, false)),
                    _ => (),
                }
                i = end;
            }
            _ => i += 1,
        }
    }
    directives
}

/// The offset of the end of the line containing `offset`.
fn line_end(src: &str, offset: usize) -> usize {
    src.get(offset..)
        .and_then(|rest| rest.find('\n'))
        .map_or(src.len(), |n| offset + n)
}

/// Where the text that belongs to a policy ending at `end` stops: the end of
/// its line if only whitespace or a comment follows it there, `end` otherwise.
fn region_end(src: &str, end: usize) -> usize {
    let line_end = line_end(src, end);
    match src.get(end..line_end).map(str::trim_start) {
        Some(rest) if rest.is_empty() || rest.starts_with("//") => line_end,
        _ => end,
    }
}

/// For each policy, given as its byte span in `src` (or `None` if unknown),
/// the input text to output in place of the formatted policy, or `None` if it
/// should be formatted. The text starts right after the previous policy's text,
/// so it begins with the whitespace that separated them.
pub(crate) fn verbatim_policies<'a>(
    src: &'a str,
    spans: &[Option<(usize, usize)>],
) -> Vec<Option<&'a str>> {
    let directives = directives(src);
    if !directives.iter().any(|(_, off)| *off) {
        return vec![None; spans.len()];
    }
    let mut region_start = 0;
    spans
        .iter()
        .map(|span| {
            let (start, end) = (*span)?;
            let off = directives
                .iter()
                .rev()
                .find(|(offset, _)| *offset < start)
                .is_some_and(|(_, off)| *off);
            let start_of_region = region_start;
            region_start = region_end(src, end);
            if off {
                src.get(start_of_region..region_start)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pprint::{policies_str_to_pretty, Config};

    #[test]
    fn finds_directives_outside_strings() {
        let src = "// @formatter:off\npermit(principal, action, resource) when { \"// @formatter:on\" };\n  //   @formatter:on  \n// @formatter:offx\n";
        assert_eq!(directives(src), vec![(0, true), (85, false)]);
    }

    #[test]
    fn test_formatter_off() {
        // Policies between `// @formatter:off` and `// @formatter:on` are
        // output exactly as written (#1559).
        let config = Config {
            line_width: 80,
            indent_width: 2,
        };
        let kept = "// @formatter:off\npermit(principal  ==  User::\"alice\",\n       action,\n       resource)   // kept\n  when { resource.owner   ==   principal };\nforbid(principal,action,resource);";
        let src = format!(
            "permit(principal,action,resource);\n\n{kept}\n// @formatter:on\npermit(principal,action,resource)when{{true}};\n"
        );
        assert_eq!(
            policies_str_to_pretty(&src, &config).unwrap(),
            format!(
                "permit (principal, action, resource);\n\n{kept}\n\n// @formatter:on\npermit (principal, action, resource)\nwhen {{ true }};\n"
            )
        );

        // An unmatched `off` lasts to the end of the input
        let src = "permit(principal,action,resource);\n// @formatter:off\npermit(principal,action,resource)when{true};\nforbid ( principal , action , resource ) ;\n";
        assert_eq!(
            policies_str_to_pretty(src, &config).unwrap(),
            "permit (principal, action, resource);\n\n// @formatter:off\npermit(principal,action,resource)when{true};\nforbid ( principal , action , resource ) ;\n"
        );

        // A directive in a string is not a comment
        let src = r#"permit(principal,action,resource)when{"// @formatter:off"=="x"};"#;
        assert_eq!(
            policies_str_to_pretty(src, &config).unwrap(),
            "permit (principal, action, resource)\nwhen { \"// @formatter:off\" == \"x\" };\n"
        );
    }
}
