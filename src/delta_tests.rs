use super::*;

fn rows(delta: &Delta) -> Vec<(&str, ChangeKind, Vec<String>, Vec<String>)> {
    delta
        .changes
        .iter()
        .map(|change| {
            (
                change.name.as_str(),
                change.kind,
                change.before.clone(),
                change.after.clone(),
            )
        })
        .collect()
}

fn versions(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

const DOCS_EXAMPLES: &str = "\
zed-editor: 1.24.2 -> 1.25.2, +12.3 MiB
libfoo: 1.0, 1.1 -> 1.2
newpkg: ∅ -> 2.0
oldpkg: 1.0 -> ∅
empty: ε -> 1.0
";

#[test]
fn parses_every_documented_example() {
    let (delta, warnings) = parse(DOCS_EXAMPLES);

    assert_eq!(delta.approximation, "build-graph");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(
        rows(&delta),
        vec![
            (
                "empty",
                ChangeKind::Added,
                versions(&[]),
                versions(&["1.0"]),
            ),
            (
                "libfoo",
                ChangeKind::Changed,
                versions(&["1.0", "1.1"]),
                versions(&["1.2"]),
            ),
            (
                "newpkg",
                ChangeKind::Added,
                versions(&[]),
                versions(&["2.0"]),
            ),
            (
                "oldpkg",
                ChangeKind::Removed,
                versions(&["1.0"]),
                versions(&[]),
            ),
            (
                "zed-editor",
                ChangeKind::Changed,
                versions(&["1.24.2"]),
                versions(&["1.25.2"]),
            ),
        ]
    );
}

#[test]
fn groups_names_and_unions_versions_in_first_seen_order() {
    let (delta, warnings) = parse(
        "\
pkg: 1.0, 2.0 -> 3.0
pkg: 2.0, 1.1 -> 3.1, 3.0
same: 1.0 -> 1.0
same: 1.0 -> 1.0
",
    );

    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(
        rows(&delta),
        vec![(
            "pkg",
            ChangeKind::Changed,
            versions(&["1.0", "2.0", "1.1"]),
            versions(&["3.0", "3.1"]),
        )]
    );
}

#[test]
fn omits_equal_version_sets_in_any_order() {
    let (delta, warnings) = parse("same: 1.0, 2.0 -> 2.0, 1.0\n");

    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert!(delta.changes.is_empty());
}

#[test]
fn strips_size_suffixes_and_skips_size_only_lines() {
    let (delta, warnings) = parse(
        "\
grew: 1.0 -> 1.1, +12.3 MiB
unsigned: 1.0 -> 1.1, 12.3 MiB
same: 1.0 -> 1.0, -1.0 GiB
+12.3 MiB
\t-1.0 KiB\x20\x20
zero: 1.0 -> 1.1, +0 B
",
    );

    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(
        rows(&delta),
        vec![
            (
                "grew",
                ChangeKind::Changed,
                versions(&["1.0"]),
                versions(&["1.1"]),
            ),
            (
                "unsigned",
                ChangeKind::Changed,
                versions(&["1.0"]),
                versions(&["1.1"]),
            ),
            (
                "zero",
                ChangeKind::Changed,
                versions(&["1.0"]),
                versions(&["1.1"]),
            ),
        ]
    );
}

#[test]
fn skips_real_size_only_lines_with_package_names() {
    let (delta, warnings) = parse(
        "\
bluez-qt: +12.6 KiB
same: 58.6 KiB
same: -58.6 KiB
\t  padded:   +12.6 KiB\x20\x20
bare: 58.6 KiB
named-list: 58.6 KiB, -1.0 MiB
58.6 KiB, +1.0 MiB
",
    );

    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert!(delta.changes.is_empty());
}

#[test]
fn warns_on_malformed_size_only_lines() {
    let (delta, warnings) = parse("same: big KiB\nsame: 58.6 XB\nsame:\n");

    assert!(delta.changes.is_empty());
    assert_eq!(
        warnings,
        vec![
            "ignored malformed diff line: same: big KiB".to_owned(),
            "ignored malformed diff line: same: 58.6 XB".to_owned(),
            "ignored malformed diff line: same:".to_owned(),
        ]
    );
}

#[test]
fn warns_on_malformed_lines_in_input_order_without_prefix() {
    let (delta, warnings) = parse(
        "\
pkg: 1.0 -> 1.1
not a line
pkg 1.0 -> 1.1
: 1.0 -> 1.1
pkg: 1.0 ->
pkg: -> 1.0
pkg: 1.0, -> 1.1
pkg: 1.0 -> 1.1, +1 MiB,
->
",
    );

    assert_eq!(
        rows(&delta),
        vec![(
            "pkg",
            ChangeKind::Changed,
            versions(&["1.0"]),
            versions(&["1.1"]),
        )]
    );
    assert_eq!(
        warnings,
        vec![
            "ignored malformed diff line: not a line".to_owned(),
            "ignored malformed diff line: pkg 1.0 -> 1.1".to_owned(),
            "ignored malformed diff line: : 1.0 -> 1.1".to_owned(),
            "ignored malformed diff line: pkg: 1.0 ->".to_owned(),
            "ignored malformed diff line: pkg: -> 1.0".to_owned(),
            "ignored malformed diff line: pkg: 1.0, -> 1.1".to_owned(),
            "ignored malformed diff line: pkg: 1.0 -> 1.1, +1 MiB,".to_owned(),
            "ignored malformed diff line: ->".to_owned(),
        ]
    );
}

#[test]
fn parses_the_captured_real_output() {
    let captured = include_str!("../tests/fixtures/captured/diff-closures.txt");
    let (delta, warnings) = parse(captured);

    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(
        rows(&delta),
        vec![
            (
                "libfoo",
                ChangeKind::Changed,
                versions(&["1.0.drv"]),
                versions(&["1.1.drv"]),
            ),
            (
                "newpkg",
                ChangeKind::Added,
                versions(&[]),
                versions(&["2.0.drv"]),
            ),
            (
                "oldpkg",
                ChangeKind::Removed,
                versions(&["3.0.drv"]),
                versions(&[]),
            ),
        ]
    );
}

#[test]
fn never_panics_on_odd_input() {
    for text in [
        "",
        "   ",
        "\n\n",
        ":",
        "->",
        "→",
        "pkg: ∅ -> ε",
        "pkg: , -> ,",
        "pkg: 1.0 → 1.1 → 1.2",
        "pkg: \u{0} -> \u{1}",
        "pkg: 1.0\t->\t1.1",
        "pkg: 1.0->1.1",
        "pkg:1.0→1.1",
    ] {
        let _ = parse(text);
    }
}
