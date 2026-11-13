//! Parses a GitHub `CODEOWNERS` file and finds the owners of a path.
//!
//! Pure: takes the file's text and paths relative to the repository root, with `/` separators.
//! Matching is done by `codeowners-rs`. On top of it, lines GitHub treats as invalid are skipped
//! entirely and returned as warnings: lines with a parse error, and patterns using gitignore
//! syntax GitHub doesn't support (`!` negation, `[ ]` ranges, `\#` escapes).
//! See <https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners>.

use std::collections::BTreeMap;

use codeowners_rs::{RuleSet, parse};

pub struct CodeOwners {
    rules: RuleSet,
}

/// A skipped line. `line` is 1-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub line: u32,
    pub message: String,
}

impl CodeOwners {
    #[must_use]
    pub fn parse(text: &str) -> (Self, Vec<Warning>) {
        let parsed = parse(text);
        let line_of = |offset: usize| text[..offset.min(text.len())].matches('\n').count() + 1;

        let mut invalid: BTreeMap<usize, String> = BTreeMap::new();
        for error in &parsed.errors {
            invalid
                .entry(line_of(error.span.0))
                .or_insert_with(|| error.message.clone());
        }
        for rule in &parsed.rules {
            let raw = text
                .get(rule.pattern.1.0..rule.pattern.1.1)
                .unwrap_or_default();
            if let Some(message) = unsupported(raw) {
                invalid
                    .entry(line_of(rule.pattern.1.0))
                    .or_insert_with(|| message.to_owned());
            }
        }

        let rules = parsed
            .rules
            .into_iter()
            .filter(|rule| !invalid.contains_key(&line_of(rule.pattern.1.0)))
            .map(Into::into)
            .collect();
        let warnings = invalid
            .into_iter()
            .map(|(line, message)| Warning {
                line: u32::try_from(line).unwrap_or(u32::MAX),
                message,
            })
            .collect();
        (
            Self {
                rules: RuleSet::new(rules),
            },
            warnings,
        )
    }

    /// Owners of `path`. Empty if no line matches, or the last matching line has no owners.
    #[must_use]
    pub fn owners(&self, path: &str) -> Vec<String> {
        self.rules
            .owners(path)
            .unwrap_or_default()
            .iter()
            .map(|owner| owner.value.clone())
            .collect()
    }
}

fn unsupported(pattern: &str) -> Option<&'static str> {
    if pattern.starts_with('!') {
        Some("`!` negation isn't supported in CODEOWNERS")
    } else if pattern.contains('[') || pattern.contains(']') {
        Some("`[ ]` character ranges aren't supported in CODEOWNERS")
    } else if pattern.starts_with("\\#") {
        Some("escaping `#` isn't supported in CODEOWNERS")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::CodeOwners;

    #[test]
    fn github_documentation_example() {
        let (file, warnings) = CodeOwners::parse(
            "# This is a comment.\n\
             *       @global-owner1 @global-owner2\n\
             *.js    @js-owner #This is an inline comment.\n\
             *.go docs@example.com\n\
             *.txt @octo-org/octocats\n\
             /build/logs/ @doctocat\n\
             docs/* docs@example.com\n\
             apps/ @octocat\n\
             /docs/ @doctocat\n\
             /scripts/ @doctocat @octocat\n\
             **/logs @octocat\n\
             /apps/ @octocat\n\
             /apps/github\n",
        );
        assert_eq!(warnings, []);
        assert_eq!(
            file.owners("README.md"),
            ["@global-owner1", "@global-owner2"]
        );
        assert_eq!(file.owners("src/index.js"), ["@js-owner"]);
        assert_eq!(file.owners("main.go"), ["docs@example.com"]);
        assert_eq!(file.owners("notes.txt"), ["@octo-org/octocats"]);
        assert_eq!(file.owners("build/logs/deep/x.md"), ["@octocat"]);
        assert_eq!(file.owners("docs/getting-started.md"), ["@doctocat"]);
        assert_eq!(file.owners("nested/apps/x.md"), ["@octocat"]);
        assert_eq!(file.owners("scripts/run.sh"), ["@doctocat", "@octocat"]);
        assert_eq!(file.owners("deeply/nested/logs/x.md"), ["@octocat"]);
        assert_eq!(file.owners("apps/github/x.md"), Vec::<String>::new());
    }

    #[test]
    fn invalid_lines_are_skipped() {
        let (file, warnings) = CodeOwners::parse(
            "* @everyone\n!src/ @negated\n/src/ not-an-owner\n[ab].md @range\n\\#hash @escaped\n",
        );
        assert_eq!(
            warnings
                .iter()
                .map(|warning| warning.line)
                .collect::<Vec<_>>(),
            [2, 3, 4, 5]
        );
        assert_eq!(file.owners("src/a.md"), ["@everyone"]);
        assert_eq!(file.owners("a.md"), ["@everyone"]);
    }
}
