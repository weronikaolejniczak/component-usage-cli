use crate::{CodeOwners, Problem};

fn lines(warnings: &[crate::Warning]) -> Vec<u32> {
    warnings.iter().map(|warning| warning.line).collect()
}

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
fn patterns() {
    let owned = |pattern: &str, path: &str| {
        let (file, warnings) = CodeOwners::parse(&format!("{pattern} @o\n"));
        assert_eq!(warnings, [], "{pattern}");
        !file.owners(path).is_empty()
    };
    assert!(owned("docs/*", "docs/a.md"));
    assert!(!owned("docs/*", "docs/deep/a.md"));
    assert!(owned("/src/**", "src/a/b.ts"));
    assert!(!owned("/src/**", "src"));
    assert!(owned("a?c.md", "x/abc.md"));
    assert!(!owned("a?c.md", "x/ac.md"));
    assert!(owned("src/*.ts", "src/index.ts"));
    assert!(!owned("src/*.ts", "lib/src/index.ts"));
    assert!(owned("lib", "a/lib/b.ts"));
    assert!(owned("lib", "lib"));
    assert!(!owned("lib/", "lib"));
    assert!(owned("*a*b*", "xxaxxbxx"));
    assert!(!owned("*a*b*", "xxbxxaxx"));
    assert!(!owned("/", "a.ts"));
}

#[test]
fn invalid_lines_are_skipped_alone() {
    let (file, warnings) = CodeOwners::parse(
        "* @everyone\n!src/ @negated\n/src/ not-an-owner\n[ab].md @range\n\\#hash @escaped\n/b/ @b\n",
    );
    assert_eq!(
        warnings
            .iter()
            .map(|warning| warning.problem.clone())
            .collect::<Vec<_>>(),
        [
            Problem::Negation,
            Problem::Owner("not-an-owner".to_owned()),
            Problem::Range,
            Problem::Escape,
        ]
    );
    assert_eq!(lines(&warnings), [2, 3, 4, 5]);
    assert_eq!(file.owners("src/a.md"), ["@everyone"]);
    assert_eq!(file.owners("b/a.md"), ["@b"]);
}

#[test]
fn owner_syntax() {
    let (_, warnings) = CodeOwners::parse(concat!(
        "a @user @org/team-name @org/team_name.x dev@example.engineering\n",
        "b @\n",
        "c @org/\n",
        "d @a/b/c\n",
        "e @user!\n",
        "f user@\n",
        "g user@localhost\n",
        "h a@b@c.d\n",
    ));
    assert_eq!(lines(&warnings), [2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn warning_lines_with_cr_and_crlf() {
    let (_, warnings) = CodeOwners::parse("* @everyone\r\n!a @x\r!b @y\n");
    assert_eq!(lines(&warnings), [2, 3]);
}
