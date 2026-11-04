use super::*;
use serde_json::json;

#[test]
fn root_inputs_accept_string_and_path_references() {
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

    let roots = root_inputs(&lock).expect("roots should resolve");

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
fn root_inputs_reject_follows_and_missing_nodes() {
    let follows = json!({
        "nodes": {"root": {"inputs": {"dep": {"follows": "nixpkgs"}}}},
        "root": "root"
    });
    let missing = json!({
        "nodes": {"root": {"inputs": {"dep": "absent"}}},
        "root": "root"
    });

    let follows_error = root_inputs(&follows).expect_err("follows must fail");
    let missing_error = root_inputs(&missing).expect_err("a missing node must fail");

    assert!(follows_error.to_string().contains("follows"));
    assert!(
        missing_error
            .to_string()
            .contains("no resolved locked node")
    );
}

#[test]
fn locked_report_omits_absent_fields() {
    let locked = locked_report(&json!({"narHash": "sha256-example"}));

    assert_eq!(locked.rev, None);
    assert_eq!(locked.last_modified, None);
    assert_eq!(locked.nar_hash.as_deref(), Some("sha256-example"));
}
