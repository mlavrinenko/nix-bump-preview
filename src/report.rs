use schemars::JsonSchema;
use serde::Serialize;

/// Version of the JSON report shape.
pub const SCHEMA_VERSION: u32 = 1;

/// A machine-readable preview report.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// Version of the report shape; currently 1.
    pub schema_version: u32,
    /// Canonical path to the flake directory.
    pub flake: String,
    /// Selected inputs, sorted by name.
    pub inputs: Vec<InputReport>,
    /// Installables in CLI order.
    pub installables: Vec<InstallableReport>,
    /// Attrs in CLI order; omitted when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attrs: Vec<AttrReport>,
    /// Non-fatal problems; omitted when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// One selected input and its resolved lock entries.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InputReport {
    /// Input name.
    pub name: String,
    /// Whether any resolved field differs between the two locks.
    pub changed: bool,
    /// Entry in the before lock.
    pub before: Locked,
    /// Entry in the after lock.
    pub after: Locked,
}

/// Resolved fields of a lock entry; absent fields are omitted.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Locked {
    /// Resolved revision.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub rev: Option<String>,
    /// Resolution timestamp as a Unix timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub last_modified: Option<u64>,
    /// NAR hash of the resolved source.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nar_hash: Option<String>,
}

/// One installable and its derivation preview.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstallableReport {
    /// Installable attr.
    pub attr: String,
    /// Derivation evaluated against the before lock.
    pub before: Derivation,
    /// Derivation evaluated against the after lock.
    pub after: Derivation,
    /// Package delta; omitted when not computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Delta")]
    pub delta: Option<Delta>,
    /// Build and fetch cost; omitted when not computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Cost")]
    pub cost: Option<Cost>,
}

/// A derivation path evaluated against one lock.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Derivation {
    /// Store path of the `.drv` file.
    pub drv_path: String,
}

/// Package version changes from the build-time derivation graph.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Delta {
    /// Name of the approximation used for the delta.
    pub approximation: String,
    /// Changed packages, sorted by name.
    pub changes: Vec<Change>,
}

/// One package whose version set changed.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    /// Package name.
    pub name: String,
    /// What happened to the package.
    pub kind: ChangeKind,
    /// Versions in the before lock.
    pub before: Vec<String>,
    /// Versions in the after lock.
    pub after: Vec<String>,
}

/// Classification of a package change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    /// The package appeared in the after graph.
    Added,
    /// The package disappeared from the after graph.
    Removed,
    /// The package stayed but its versions differ.
    Changed,
}

/// What the proposed lock would build and fetch.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    /// Derivations to build.
    pub to_build: u64,
    /// Paths to fetch.
    pub to_fetch: u64,
    /// Download size in bytes.
    pub download_bytes: u64,
    /// Unpacked size in bytes.
    pub unpacked_bytes: u64,
}

/// Direct version report for one attr.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttrReport {
    /// Attr whose `.version` was evaluated.
    pub attr: String,
    /// Version under the before lock.
    pub before: String,
    /// Version under the after lock.
    pub after: String,
}

/// Render the report JSON schema as pretty-printed JSON.
///
/// # Errors
///
/// Returns an error if the generated schema cannot be serialized.
pub fn schema_json() -> serde_json::Result<String> {
    let schema = schemars::generate::SchemaSettings::default()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<Report>();
    serde_json::to_string_pretty(&schema)
}

#[cfg(test)]
mod tests {
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
                attr: "nixosConfigurations.example._module.args.pkgs.unstable.zed-editor"
                    .to_owned(),
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
}
