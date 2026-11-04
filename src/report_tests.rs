use super::*;

const COMPLETE_REPORT_JSON: &str = r#"{
  "schemaVersion": 1,
  "flake": "/tmp/flake",
  "inputs": [
    {
      "name": "unstable",
      "changed": true,
      "before": {
        "rev": "fe132422",
        "lastModified": 1792094642,
        "narHash": "sha256-before"
      },
      "after": {
        "rev": "3ec63636",
        "lastModified": 1792773339,
        "narHash": "sha256-after"
      }
    }
  ],
  "installables": [
    {
      "attr": "nixosConfigurations.example.config.system.build.toplevel",
      "before": {
        "drvPath": "/nix/store/old.drv"
      },
      "after": {
        "drvPath": "/nix/store/new.drv"
      },
      "delta": {
        "approximation": "build-graph",
        "changes": [
          {
            "name": "zed-editor",
            "kind": "changed",
            "before": [
              "1.24.2"
            ],
            "after": [
              "1.25.2"
            ]
          }
        ]
      },
      "cost": {
        "toBuild": 721,
        "toFetch": 671,
        "downloadBytes": 3221225472,
        "unpackedBytes": 9019431322
      }
    }
  ],
  "attrs": [
    {
      "attr": "nixosConfigurations.example._module.args.pkgs.unstable.zed-editor",
      "before": "1.24.2",
      "after": "1.25.2"
    }
  ],
  "warnings": [
    "the delta is the build-time derivation graph, not the runtime closure"
  ]
}"#;

#[test]
fn serializes_the_complete_report_in_struct_order() {
    let report = Report {
        schema_version: SCHEMA_VERSION,
        flake: "/tmp/flake".to_owned(),
        inputs: vec![InputReport {
            name: "unstable".to_owned(),
            changed: true,
            before: Locked {
                rev: Some("fe132422".to_owned()),
                last_modified: Some(1_792_094_642),
                nar_hash: Some("sha256-before".to_owned()),
            },
            after: Locked {
                rev: Some("3ec63636".to_owned()),
                last_modified: Some(1_792_773_339),
                nar_hash: Some("sha256-after".to_owned()),
            },
        }],
        installables: vec![InstallableReport {
            attr: "nixosConfigurations.example.config.system.build.toplevel".to_owned(),
            before: Derivation {
                drv_path: "/nix/store/old.drv".to_owned(),
            },
            after: Derivation {
                drv_path: "/nix/store/new.drv".to_owned(),
            },
            delta: Some(Delta {
                approximation: "build-graph".to_owned(),
                changes: vec![Change {
                    name: "zed-editor".to_owned(),
                    kind: ChangeKind::Changed,
                    before: vec!["1.24.2".to_owned()],
                    after: vec!["1.25.2".to_owned()],
                }],
            }),
            cost: Some(Cost {
                to_build: 721,
                to_fetch: 671,
                download_bytes: 3_221_225_472,
                unpacked_bytes: 9_019_431_322,
            }),
        }],
        attrs: vec![AttrReport {
            attr: "nixosConfigurations.example._module.args.pkgs.unstable.zed-editor".to_owned(),
            before: "1.24.2".to_owned(),
            after: "1.25.2".to_owned(),
        }],
        warnings: vec![
            "the delta is the build-time derivation graph, not the runtime closure".to_owned(),
        ],
    };

    let json = serde_json::to_string_pretty(&report).expect("report should serialize");

    assert_eq!(json, COMPLETE_REPORT_JSON);
}

#[test]
fn omits_optional_and_empty_fields() {
    let report = Report {
        schema_version: SCHEMA_VERSION,
        flake: "/tmp/flake".to_owned(),
        inputs: vec![InputReport {
            name: "path".to_owned(),
            changed: false,
            before: Locked {
                rev: None,
                last_modified: None,
                nar_hash: None,
            },
            after: Locked {
                rev: None,
                last_modified: Some(1_792_094_642),
                nar_hash: None,
            },
        }],
        installables: vec![InstallableReport {
            attr: "example".to_owned(),
            before: Derivation {
                drv_path: "/nix/store/old.drv".to_owned(),
            },
            after: Derivation {
                drv_path: "/nix/store/new.drv".to_owned(),
            },
            delta: None,
            cost: None,
        }],
        attrs: Vec::new(),
        warnings: Vec::new(),
    };

    let value = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(
        value,
        serde_json::json!({
            "schemaVersion": 1,
            "flake": "/tmp/flake",
            "inputs": [
                {
                    "name": "path",
                    "changed": false,
                    "before": {},
                    "after": { "lastModified": 1_792_094_642 }
                }
            ],
            "installables": [
                {
                    "attr": "example",
                    "before": { "drvPath": "/nix/store/old.drv" },
                    "after": { "drvPath": "/nix/store/new.drv" }
                }
            ]
        })
    );
}

#[test]
fn renders_utc_dates_from_timestamps() {
    assert_eq!(format_date(0), "1970-01-01");
    assert_eq!(format_date(1_792_094_642), "2026-10-15");
    assert_eq!(format_date(1_792_773_339), "2026-10-23");
    assert_eq!(format_date(1_709_164_800), "2024-02-29");
    assert_eq!(format_date(1_798_761_600), "2027-01-01");
}

#[test]
fn renders_sizes_in_the_largest_binary_unit() {
    assert_eq!(format_size(0), "0.0 B");
    assert_eq!(format_size(1_023), "1023.0 B");
    assert_eq!(format_size(1_024), "1.0 KiB");
    assert_eq!(format_size(1_536), "1.5 KiB");
    assert_eq!(format_size(3_221_225_472), "3.0 GiB");
    assert_eq!(format_size(9_019_431_322), "8.4 GiB");
    assert_eq!(format_size(1_099_511_627_776), "1.0 TiB");
}

fn locked(rev: Option<&str>, last_modified: Option<u64>, nar_hash: Option<&str>) -> Locked {
    Locked {
        rev: rev.map(ToOwned::to_owned),
        last_modified,
        nar_hash: nar_hash.map(ToOwned::to_owned),
    }
}

#[test]
fn renders_revision_dates_and_documented_fallbacks() {
    let report = Report {
        schema_version: SCHEMA_VERSION,
        flake: "/tmp/flake".to_owned(),
        inputs: vec![
            InputReport {
                name: "rev".to_owned(),
                changed: true,
                before: locked(Some("fe132422aa"), Some(1_792_094_642), None),
                after: locked(Some("3ec63636"), None, None),
            },
            InputReport {
                name: "hash".to_owned(),
                changed: true,
                before: locked(None, Some(1_792_094_642), Some("sha256-before")),
                after: locked(None, None, None),
            },
        ],
        installables: Vec::new(),
        attrs: Vec::new(),
        warnings: Vec::new(),
    };

    assert_eq!(
        render_text(&report),
        "\
input rev
  fe132422 (2026-10-15) -> 3ec63636
input hash
  sha256-b (2026-10-15) -> ?
"
    );
}

#[test]
fn renders_attrs_once_after_every_installable() {
    let report = Report {
        schema_version: SCHEMA_VERSION,
        flake: "/tmp/flake".to_owned(),
        inputs: Vec::new(),
        installables: vec![
            InstallableReport {
                attr: "first".to_owned(),
                before: Derivation {
                    drv_path: "/nix/store/old.drv".to_owned(),
                },
                after: Derivation {
                    drv_path: "/nix/store/new.drv".to_owned(),
                },
                delta: Some(Delta {
                    approximation: "build-graph".to_owned(),
                    changes: Vec::new(),
                }),
                cost: None,
            },
            InstallableReport {
                attr: "second".to_owned(),
                before: Derivation {
                    drv_path: "/nix/store/old.drv".to_owned(),
                },
                after: Derivation {
                    drv_path: "/nix/store/new.drv".to_owned(),
                },
                delta: None,
                cost: Some(Cost {
                    to_build: 0,
                    to_fetch: 0,
                    download_bytes: 0,
                    unpacked_bytes: 0,
                }),
            },
        ],
        attrs: vec![AttrReport {
            attr: "packages.example".to_owned(),
            before: "1.0".to_owned(),
            after: "2.0".to_owned(),
        }],
        warnings: Vec::new(),
    };

    assert_eq!(
        render_text(&report),
        "\
installable first
  delta (build graph): 0 changes
installable second
  cost: 0 to build, 0 to fetch (0.0 B download, 0.0 B unpacked)
  attr packages.example: 1.0 -> 2.0
"
    );
}
