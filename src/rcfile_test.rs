use {
  crate::{
    context::ConfigError,
    rcfile::{semver_group::AnySemverGroup, RawRcfile, Rcfile},
    version_group::{AnyVersionGroup, VersionGroup},
  },
  serde_json::json,
};

#[test]
fn default_format_bugs_is_false() {
  let rcfile = Rcfile::default();
  assert!(!rcfile.format_bugs);
}

#[test]
fn default_format_repository_is_false() {
  let rcfile = Rcfile::default();
  assert!(!rcfile.format_repository);
}

#[test]
fn detects_v13_dependency_types_in_config() {
  let config_json = json!({
    "dependencyTypes": ["prod", "dev"],
    "versionGroups": []
  });
  let rcfile: RawRcfile = serde_json::from_value(config_json).unwrap();
  assert!(rcfile.unknown_fields.contains_key("dependencyTypes"));
}

#[test]
fn detects_v13_specifier_types_in_config() {
  let config_json = json!({
    "specifierTypes": ["exact", "range"],
    "versionGroups": []
  });
  let rcfile: RawRcfile = serde_json::from_value(config_json).unwrap();
  assert!(rcfile.unknown_fields.contains_key("specifierTypes"));
}

#[test]
fn detects_v13_lint_formatting_in_config() {
  let config_json = json!({
    "lintFormatting": true,
    "versionGroups": []
  });
  let rcfile: RawRcfile = serde_json::from_value(config_json).unwrap();
  assert!(rcfile.unknown_fields.contains_key("lintFormatting"));
}

#[test]
fn detects_v13_lint_semver_ranges_in_config() {
  let config_json = json!({
    "lintSemverRanges": true,
    "versionGroups": []
  });
  let rcfile: RawRcfile = serde_json::from_value(config_json).unwrap();
  assert!(rcfile.unknown_fields.contains_key("lintSemverRanges"));
}

#[test]
fn detects_v13_lint_versions_in_config() {
  let config_json = json!({
    "lintVersions": true,
    "versionGroups": []
  });
  let rcfile: RawRcfile = serde_json::from_value(config_json).unwrap();
  assert!(rcfile.unknown_fields.contains_key("lintVersions"));
}

#[test]
fn detects_multiple_v13_properties_in_config() {
  let config_json = json!({
    "dependencyTypes": ["prod", "dev"],
    "specifierTypes": ["exact"],
    "lintFormatting": true,
    "lintSemverRanges": false,
    "lintVersions": true,
    "versionGroups": []
  });
  let rcfile: RawRcfile = serde_json::from_value(config_json).unwrap();
  assert_eq!(rcfile.unknown_fields.len(), 5);
  assert!(rcfile.unknown_fields.contains_key("dependencyTypes"));
  assert!(rcfile.unknown_fields.contains_key("specifierTypes"));
  assert!(rcfile.unknown_fields.contains_key("lintFormatting"));
  assert!(rcfile.unknown_fields.contains_key("lintSemverRanges"));
  assert!(rcfile.unknown_fields.contains_key("lintVersions"));
}

#[test]
fn valid_v14_config_has_no_unknown_fields() {
  let config_json = json!({
    "versionGroups": [],
    "semverGroups": [],
    "indent": "  ",
    "source": ["packages/*/package.json"]
  });
  let rcfile: RawRcfile = serde_json::from_value(config_json).unwrap();
  assert_eq!(rcfile.unknown_fields.len(), 0);
}

#[test]
fn validate_unknown_fields_returns_deprecated_errors() {
  let raw: RawRcfile = serde_json::from_value(json!({
    "dependencyTypes": ["prod"],
    "lintFormatting": true,
  }))
  .unwrap();
  let errors = raw.validate_unknown_fields().unwrap_err();
  assert_eq!(errors.len(), 2);
  assert!(errors
    .iter()
    .any(|e| matches!(e, ConfigError::DeprecatedProperty { property, .. } if property == "dependencyTypes")));
  assert!(errors
    .iter()
    .any(|e| matches!(e, ConfigError::DeprecatedProperty { property, .. } if property == "lintFormatting")));
}

#[test]
fn validate_unknown_fields_returns_unrecognised_errors() {
  let raw: RawRcfile = serde_json::from_value(json!({
    "notARealProperty": true,
  }))
  .unwrap();
  let errors = raw.validate_unknown_fields().unwrap_err();
  assert_eq!(errors.len(), 1);
  assert!(matches!(&errors[0], ConfigError::UnrecognisedProperty { path } if path == "notARealProperty"));
}

#[test]
fn validate_unknown_fields_returns_nested_unrecognised_errors() {
  let raw: RawRcfile = serde_json::from_value(json!({
    "versionGroups": [{ "label": "test", "notReal": true }],
    "semverGroups": [{ "range": "^", "bogus": 1 }],
  }))
  .unwrap();
  let errors = raw.validate_unknown_fields().unwrap_err();
  assert_eq!(errors.len(), 2);
  assert!(errors
    .iter()
    .any(|e| matches!(e, ConfigError::UnrecognisedProperty { path } if path == "versionGroups[0].notReal")));
  assert!(errors
    .iter()
    .any(|e| matches!(e, ConfigError::UnrecognisedProperty { path } if path == "semverGroups[0].bogus")));
}

#[test]
fn validate_unknown_fields_ok_when_valid() {
  let raw: RawRcfile = serde_json::from_value(json!({})).unwrap();
  assert!(raw.validate_unknown_fields().is_ok());
}

#[test]
fn try_from_rejects_invalid_dependency_type() {
  let raw: RawRcfile = serde_json::from_value(json!({
    "versionGroups": [{
      "label": "test",
      "dependencyTypes": ["nonexistent"]
    }]
  }))
  .unwrap();
  let err = Rcfile::try_from(raw).unwrap_err();
  assert!(matches!(err, ConfigError::InvalidDependencyType { name } if name == "nonexistent"));
}

#[test]
fn semver_group_from_config_rejects_missing_required_fields() {
  let group: AnySemverGroup = serde_json::from_value(json!({
    "label": "bad group"
  }))
  .unwrap();
  let err = crate::rcfile::semver_group::SemverGroup::from_config(group).unwrap_err();
  assert!(matches!(err, ConfigError::InvalidSemverGroup));
}

#[test]
fn version_group_from_config_rejects_invalid_policy() {
  let group: AnyVersionGroup = serde_json::from_value(json!({
    "label": "bad",
    "policy": "notAPolicy"
  }))
  .unwrap();
  let packages = crate::packages::Packages::new();
  let err = VersionGroup::from_config(group, &packages).unwrap_err();
  assert!(matches!(err, ConfigError::InvalidVersionGroupPolicy(p) if p == "notAPolicy"));
}
