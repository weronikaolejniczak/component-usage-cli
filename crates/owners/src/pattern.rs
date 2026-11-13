//! `CODEOWNERS` patterns, matched against file paths split on `/`.
//!
//! As in `.gitignore`: a pattern without a `/`, other than a trailing one, matches at any depth;
//! otherwise it's relative to the repository root. A trailing `/` matches only what's inside the
//! folder. `*` and `?` don't match `/`; a `**` segment matches any number of folders.
//! Unlike `.gitignore`, a pattern whose last segment has a wildcard matches files only, not
//! folders, so `docs/*` doesn't match `docs/deep/file`.

pub(crate) struct Pattern {
    segments: Vec<Segment>,
    /// Whether the pattern also matches folders, and so every file inside them.
    folders: bool,
}

enum Segment {
    /// `**`
    AnyDepth,
    Glob(String),
}

impl Pattern {
    pub(crate) fn new(pattern: &str) -> Self {
        let (pattern, inside) = match pattern.strip_suffix('/') {
            Some(pattern) => (pattern, true),
            None => (pattern, false),
        };
        let anchored = pattern.contains('/');
        let pattern = pattern.strip_prefix('/').unwrap_or(pattern);
        let mut segments: Vec<Segment> = Vec::new();
        if pattern.is_empty() {
            return Self {
                segments,
                folders: false,
            };
        }
        if !anchored {
            segments.push(Segment::AnyDepth);
        }
        segments.extend(
            pattern
                .split('/')
                .filter(|part| !part.is_empty())
                .map(|part| {
                    if part == "**" {
                        Segment::AnyDepth
                    } else {
                        Segment::Glob(part.to_owned())
                    }
                }),
        );
        let last_is_literal =
            matches!(segments.last(), Some(Segment::Glob(glob)) if !glob.contains(['*', '?']));
        if inside {
            segments.push(Segment::Glob("*".to_owned()));
        }
        Self {
            segments,
            folders: inside || last_is_literal,
        }
    }

    pub(crate) fn matches(&self, path: &[&str]) -> bool {
        if self.segments.is_empty() {
            return false;
        }
        if self.folders {
            (1..=path.len()).any(|len| matches(&self.segments, &path[..len]))
        } else {
            matches(&self.segments, path)
        }
    }
}

fn matches(segments: &[Segment], path: &[&str]) -> bool {
    match segments.split_first() {
        None => path.is_empty(),
        Some((Segment::AnyDepth, [])) => !path.is_empty(),
        Some((Segment::AnyDepth, rest)) => {
            (0..=path.len()).any(|skip| matches(rest, &path[skip..]))
        }
        Some((Segment::Glob(glob), rest)) => path
            .split_first()
            .is_some_and(|(part, path)| wildcard(glob, part) && matches(rest, path)),
    }
}

/// `*` matches any run of characters, `?` any one character.
fn wildcard(glob: &str, text: &str) -> bool {
    let glob: Vec<char> = glob.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut g, mut t) = (0, 0);
    let mut backtrack = None;
    while t < text.len() {
        match glob.get(g) {
            Some('*') => {
                backtrack = Some((g, t));
                g += 1;
            }
            Some(&char) if char == '?' || char == text[t] => {
                g += 1;
                t += 1;
            }
            _ => match backtrack {
                Some((star, matched)) => {
                    g = star + 1;
                    t = matched + 1;
                    backtrack = Some((star, matched + 1));
                }
                None => return false,
            },
        }
    }
    glob[g..].iter().all(|&char| char == '*')
}
