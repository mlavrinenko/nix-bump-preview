use super::*;
use serde_json::json;

#[test]
fn selected_inputs_accept_string_and_path_references() {
    let lock = json!({
        "nodes": {
            "root": {
                "inputs": {
                    "alpha": "node-alpha",
                    "beta": {"path": "node-beta"}
                }
            },
            "node-alpha": {"locked": {"rev": "aaa"}},
            "node-beta": {"locked": {"rev": "bbb"}}
        },
        "root": "root"
    });
    let names = vec!["alpha".to_owned(), "beta".to_owned()];

    let roots = selected_inputs(&lock, &names).expect("roots should resolve");

    assert_eq!(roots.len(), 2);
    assert_eq!(
        roots
            .get("alpha")
            .and_then(|value| value.get("rev"))
            .and_then(Value::as_str),
        Some("aaa")
    );
    assert_eq!(
        roots
            .get("beta")
            .and_then(|value| value.get("rev"))
            .and_then(Value::as_str),
        Some("bbb")
    );
}

#[test]
fn selected_inputs_reject_a_missing_node() {
    let lock = json!({
        "nodes": {"root": {"inputs": {"dep": "absent"}}},
        "root": "root"
    });
    let names = vec!["dep".to_owned()];

    let error = selected_inputs(&lock, &names).expect_err("a missing node must fail");

    assert!(error.to_string().contains("no resolved locked node"));
}

#[test]
fn follows_resolve_recursively() {
    let lock = json!({
        "nodes": {
            "root": {
                "inputs": {
                    "first": {"follows": "second"},
                    "second": {"follows": "target"},
                    "target": "node"
                }
            },
            "node": {"locked": {"rev": "ccc"}}
        },
        "root": "root"
    });
    let names = vec!["first".to_owned(), "second".to_owned(), "target".to_owned()];

    let roots = selected_inputs(&lock, &names).expect("follows should resolve");

    for name in ["first", "second", "target"] {
        assert_eq!(
            roots
                .get(name)
                .and_then(|value| value.get("rev"))
                .and_then(Value::as_str),
            Some("ccc")
        );
    }
}

#[test]
fn follows_missing_target_is_an_error() {
    let lock = json!({
        "nodes": {"root": {"inputs": {"dep": {"follows": "absent"}}}},
        "root": "root"
    });
    let names = vec!["dep".to_owned()];

    let error = selected_inputs(&lock, &names).expect_err("a missing follow target must fail");

    assert!(error.to_string().contains("follows"));
    assert!(error.to_string().contains("absent"));
}

#[test]
fn follows_cycle_is_an_error() {
    let lock = json!({
        "nodes": {
            "root": {
                "inputs": {
                    "first": {"follows": "second"},
                    "second": {"follows": "first"}
                }
            }
        },
        "root": "root"
    });
    let names = vec!["first".to_owned(), "second".to_owned()];

    let error = selected_inputs(&lock, &names).expect_err("a follows cycle must fail");

    assert!(error.to_string().contains("cycle"));
}

#[test]
fn empty_root_has_no_names() {
    let lock = json!({"nodes": {"root": {}}, "root": "root"});

    let names = root_names(&lock).expect("an empty root is valid");

    assert!(names.is_empty());
}

#[test]
fn lock_without_a_root_node_is_an_error() {
    let lock = json!({"nodes": {}});

    let error = root_names(&lock).expect_err("a missing root node must fail");

    assert!(error.to_string().contains("root node"));
}

#[test]
fn locked_report_omits_absent_fields() {
    let locked = locked_report(&json!({"narHash": "sha256-example"}));

    assert_eq!(locked.rev, None);
    assert_eq!(locked.last_modified, None);
    assert_eq!(locked.nar_hash.as_deref(), Some("sha256-example"));
}
