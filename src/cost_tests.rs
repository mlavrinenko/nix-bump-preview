use super::*;

fn parsed(text: &str) -> Cost {
    parse(text).expect("cost should parse")
}

fn fetched(download: &str, unpacked: &str) -> String {
    format!("these 1 paths will be fetched ({download} download, {unpacked} unpacked):\n")
}

fn assert_malformed(text: &str, needle: &str) {
    let error = parse(text).expect_err("header should be rejected");
    assert!(
        matches!(&error, CostError::Malformed { .. }),
        "unexpected error: {error}"
    );
    let message = error.to_string();
    assert!(
        message.contains("malformed cost header"),
        "unclear error: {message}"
    );
    assert!(message.contains(needle), "missing detail: {message}");
}

fn assert_overflow(text: &str) {
    let error = parse(text).expect_err("overflow should be rejected");
    assert!(
        matches!(&error, CostError::Overflow { .. }),
        "unexpected error: {error}"
    );
    assert!(
        error.to_string().contains("overflows u64"),
        "unclear error: {error}"
    );
}

#[test]
fn parses_the_captured_real_output() {
    let cost = parsed(include_str!("../tests/fixtures/captured/dry-run.txt"));

    assert_eq!(cost.to_build, 1);
    assert_eq!(cost.to_fetch, 0);
    assert_eq!(cost.download_bytes, 0);
    assert_eq!(cost.unpacked_bytes, 0);
}

#[test]
fn parses_both_plural_headers_and_ignores_indented_paths() {
    let cost = parsed(
        "\
these 721 derivations will be built:
  /nix/store/00000000000000000000000000000000-example.drv
these 671 paths will be fetched (3.0 GiB download, 8.4 GiB unpacked):
  /nix/store/00000000000000000000000000000000-example
",
    );

    assert_eq!(cost.to_build, 721);
    assert_eq!(cost.to_fetch, 671);
    assert_eq!(cost.download_bytes, 3_221_225_472);
    assert_eq!(cost.unpacked_bytes, 9_019_431_322);
}

#[test]
fn parses_singular_headers_without_counts() {
    let cost = parsed(
        "\
this derivation will be built:
  /nix/store/00000000000000000000000000000000-example.drv
this path will be fetched (1 B download, 2 B unpacked):
  /nix/store/00000000000000000000000000000000-example
",
    );

    assert_eq!(cost.to_build, 1);
    assert_eq!(cost.to_fetch, 1);
    assert_eq!(cost.download_bytes, 1);
    assert_eq!(cost.unpacked_bytes, 2);
}

#[test]
fn parses_explicit_singular_counts() {
    let cost = parsed(
        "\
these 1 derivations will be built:
this 1 derivation will be built:
these 1 paths will be fetched (1 KiB download, 1 KiB unpacked):
this 1 path will be fetched (2 KiB download, 2 KiB unpacked):
",
    );

    assert_eq!(cost.to_build, 2);
    assert_eq!(cost.to_fetch, 2);
    assert_eq!(cost.download_bytes, 3_072);
    assert_eq!(cost.unpacked_bytes, 3_072);
}

#[test]
fn missing_headers_are_zero() {
    let cost = parsed(
        "\
warning: unrelated diagnostics
  /nix/store/00000000000000000000000000000000-example.drv
this is not a plan header
",
    );

    assert_eq!(cost.to_build, 0);
    assert_eq!(cost.to_fetch, 0);
    assert_eq!(cost.download_bytes, 0);
    assert_eq!(cost.unpacked_bytes, 0);
}

#[test]
fn repeated_headers_sum_counts_and_sizes() {
    let cost = parsed(
        "\
these 2 derivations will be built:
these 3 derivations will be built:
these 1 paths will be fetched (1 KiB download, 2 KiB unpacked):
these 1 paths will be fetched (3 KiB download, 4 KiB unpacked):
",
    );

    assert_eq!(cost.to_build, 5);
    assert_eq!(cost.to_fetch, 2);
    assert_eq!(cost.download_bytes, 4_096);
    assert_eq!(cost.unpacked_bytes, 6_144);
}

#[test]
fn parses_every_binary_size_unit() {
    let cases = [
        ("1 B", 1_u64),
        ("1 KiB", 1_024),
        ("1 MiB", 1_048_576),
        ("1 GiB", 1_073_741_824),
        ("1 TiB", 1_099_511_627_776),
    ];

    for (size, expected) in cases {
        let cost = parsed(&fetched(size, size));
        assert_eq!(cost.download_bytes, expected, "download {size}");
        assert_eq!(cost.unpacked_bytes, expected, "unpacked {size}");
    }
}

#[test]
fn rounds_fractional_sizes_half_up() {
    let cases = [
        ("0.5 B", 1_u64),
        ("0.1 B", 0),
        ("2.5 B", 3),
        ("1.5 KiB", 1_536),
        ("0.1 KiB", 102),
        ("3.0 GiB", 3_221_225_472),
        ("8.4 GiB", 9_019_431_322),
    ];

    for (size, expected) in cases {
        let cost = parsed(&fetched(size, size));
        assert_eq!(cost.download_bytes, expected, "download {size}");
        assert_eq!(cost.unpacked_bytes, expected, "unpacked {size}");
    }
}

#[test]
fn accepts_u64_max_and_rejects_one_byte_more() {
    let cost = parsed(&fetched(&format!("{} B", u64::MAX), "0 B"));

    assert_eq!(cost.download_bytes, u64::MAX);
    assert_overflow(&fetched("18446744073709551616 B", "0 B"));
}

#[test]
fn rejects_overflow_beyond_u64() {
    assert_overflow(&fetched("17179869184 GiB", "0 B"));
    assert_overflow(&fetched("0 B", "18446744073709551616 B"));
}

#[test]
fn rejects_malformed_sizes_with_the_header_line() {
    for size in ["lots GiB", "1 XB", "1.2.3 GiB", "GiB", "1"] {
        assert_malformed(&fetched(size, "0 B"), "these 1 paths");
    }
}

#[test]
fn rejects_malformed_fetch_headers() {
    assert_malformed("these 1 paths will be fetched:\n", "these 1 paths");
    assert_malformed(
        "these 1 paths will be fetched (1 KiB download):\n",
        "these 1 paths",
    );
    assert_malformed(
        "these 1 paths will be fetched (1 KiB, 1 KiB):\n",
        "these 1 paths",
    );
}

#[test]
fn rejects_malformed_counts_with_the_header_line() {
    for header in [
        "these lots derivations will be built:",
        "this nope derivation will be built:",
        "these derivations will be built:",
        "these 18446744073709551616 derivations will be built:",
        "these lots paths will be fetched (0 B download, 0 B unpacked):",
    ] {
        assert_malformed(header, header);
    }
}

#[test]
fn repeated_counts_that_overflow_are_errors() {
    assert_overflow(&format!(
        "these {max} derivations will be built:\nthese {max} derivations will be built:\n",
        max = u64::MAX
    ));
}

#[test]
fn never_panics_on_odd_input() {
    for text in [
        "",
        "these",
        "this ",
        "these\n",
        "this  derivation will be built:",
        "these NaN paths will be fetched (NaN download, NaN unpacked):",
        "these 1 paths will be fetched (,):",
        "\u{0}",
    ] {
        let _ = parse(text);
    }
}
