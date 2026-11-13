//! Parses a GitHub `CODEOWNERS` file and finds the owners of a path.
//!
//! Pure: takes the file's text and paths relative to the repository root, with `/` separators.
//! Each line is parsed on its own, so an invalid line is skipped, with a warning, without
//! affecting the others. Lines using gitignore syntax GitHub doesn't support (`!` negation,
//! `[ ]` ranges, `\#` escapes) are invalid too.
//! See <https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners>.

mod pattern;

use pattern::Pattern;

pub struct CodeOwners {
    rules: Vec<Rule>,
}

struct Rule {
    pattern: Pattern,
    owners: Vec<String>,
}

/// A skipped line. `line` is 1-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub line: u32,
    pub problem: Problem,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    #[error("`!` negation isn't supported in CODEOWNERS")]
    Negation,
    #[error("`[ ]` character ranges aren't supported in CODEOWNERS")]
    Range,
    #[error("escaping `#` isn't supported in CODEOWNERS")]
    Escape,
    #[error("'{0}' isn't a @user, @org/team or email address")]
    Owner(String),
}

impl CodeOwners {
    #[must_use]
    pub fn parse(text: &str) -> (Self, Vec<Warning>) {
        let mut rules = Vec::new();
        let mut warnings = Vec::new();
        for (index, line) in lines(text).enumerate() {
            match rule(line) {
                Ok(Some(rule)) => rules.push(rule),
                Ok(None) => {}
                Err(problem) => warnings.push(Warning {
                    line: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    problem,
                }),
            }
        }
        (Self { rules }, warnings)
    }

    /// Owners of `path`. Empty if no line matches, or the last matching line has no owners.
    #[must_use]
    pub fn owners(&self, path: &str) -> Vec<String> {
        let segments: Vec<&str> = path.split('/').collect();
        self.rules
            .iter()
            .rev()
            .find(|rule| rule.pattern.matches(&segments))
            .map(|rule| rule.owners.clone())
            .unwrap_or_default()
    }
}

/// LF, CRLF and CR all end a line.
fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n')
        .flat_map(|line| line.strip_suffix('\r').unwrap_or(line).split('\r'))
}

/// `Ok(None)` for blank and comment lines.
fn rule(line: &str) -> Result<Option<Rule>, Problem> {
    let mut tokens = line
        .split_whitespace()
        .take_while(|token| !token.starts_with('#'));
    let Some(pattern) = tokens.next() else {
        return Ok(None);
    };
    if pattern.starts_with('!') {
        return Err(Problem::Negation);
    }
    if pattern.contains(['[', ']']) {
        return Err(Problem::Range);
    }
    if pattern.starts_with("\\#") {
        return Err(Problem::Escape);
    }
    let owners = tokens
        .map(|owner| {
            is_owner(owner)
                .then(|| owner.to_owned())
                .ok_or_else(|| Problem::Owner(owner.to_owned()))
        })
        .collect::<Result<_, _>>()?;
    Ok(Some(Rule {
        pattern: Pattern::new(pattern),
        owners,
    }))
}

/// `@user`, `@org/team` or `local@domain.tld`.
fn is_owner(owner: &str) -> bool {
    let name = |part: &str, extra: &[char]| {
        !part.is_empty()
            && part
                .chars()
                .all(|char| char.is_ascii_alphanumeric() || char == '-' || extra.contains(&char))
    };
    if let Some(handle) = owner.strip_prefix('@') {
        return match handle.split_once('/') {
            Some((org, team)) => name(org, &[]) && name(team, &['_', '.']),
            None => name(handle, &[]),
        };
    }
    match owner.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && !domain.contains('@')
                && domain
                    .split_once('.')
                    .is_some_and(|(host, tld)| !host.is_empty() && !tld.is_empty())
        }
        None => false,
    }
}

#[cfg(test)]
mod tests;
