pub mod semver_group;

#[cfg(test)]
#[path = "rcfile_test.rs"]
mod rcfile_test;

use {
  crate::{
    dependency::DependencyType,
    group_selector::GroupSelector,
    packages::Packages,
    version_group::{AnyVersionGroup, VersionGroup},
  },
  log::error,
  semver_group::{AnySemverGroup, SemverGroup},
  serde::Deserialize,
  serde_json::Value,
  std::{collections::HashMap, mem, process::exit},
};

pub fn compute_all_dependency_types(custom_types: &HashMap<String, CustomType>) -> Vec<DependencyType> {
  let default_types = HashMap::from([
    (
      String::from("dev"),
      CustomType {
        strategy: String::from("versionsByName"),
        name_path: None,
        path: String::from("devDependencies"),
        unknown_fields: HashMap::new(),
      },
    ),
    (
      String::from("local"),
      CustomType {
        strategy: String::from("name~version"),
        name_path: Some(String::from("name")),
        path: String::from("version"),
        unknown_fields: HashMap::new(),
      },
    ),
    (
      String::from("overrides"),
      CustomType {
        strategy: String::from("versionsByName"),
        name_path: None,
        path: String::from("overrides"),
        unknown_fields: HashMap::new(),
      },
    ),
    (
      String::from("peer"),
      CustomType {
        strategy: String::from("versionsByName"),
        name_path: None,
        path: String::from("peerDependencies"),
        unknown_fields: HashMap::new(),
      },
    ),
    (
      String::from("pnpmOverrides"),
      CustomType {
        strategy: String::from("versionsByName"),
        name_path: None,
        path: String::from("pnpm.overrides"),
        unknown_fields: HashMap::new(),
      },
    ),
    (
      String::from("prod"),
      CustomType {
        strategy: String::from("versionsByName"),
        name_path: None,
        path: String::from("dependencies"),
        unknown_fields: HashMap::new(),
      },
    ),
    (
      String::from("resolutions"),
      CustomType {
        strategy: String::from("versionsByName"),
        name_path: None,
        path: String::from("resolutions"),
        unknown_fields: HashMap::new(),
      },
    ),
  ]);
  default_types
    .iter()
    .chain(custom_types.iter())
    .map(|(name, custom_type)| DependencyType::new(name, custom_type))
    .collect()
}

mod discovery;
mod error;
mod javascript;
mod json;
mod package_json;
mod yaml;

fn empty_custom_types() -> HashMap<String, CustomType> {
  HashMap::new()
}

fn default_max_concurrent_requests() -> usize {
  12
}

fn default_true() -> bool {
  true
}

fn default_false() -> bool {
  false
}

fn default_indent() -> Option<String> {
  None
}

fn default_sort_az() -> Vec<String> {
  vec![
    "bin".to_string(),
    "contributors".to_string(),
    "dependencies".to_string(),
    "devDependencies".to_string(),
    "keywords".to_string(),
    "peerDependencies".to_string(),
    "resolutions".to_string(),
    "scripts".to_string(),
  ]
}

fn default_sort_exports() -> Vec<String> {
  vec![
    "types".to_string(),
    "node-addons".to_string(),
    "node".to_string(),
    "browser".to_string(),
    "module".to_string(),
    "import".to_string(),
    "require".to_string(),
    "svelte".to_string(),
    "development".to_string(),
    "production".to_string(),
    "script".to_string(),
    "default".to_string(),
  ]
}

fn sort_first() -> Vec<String> {
  vec![
    "name".to_string(),
    "description".to_string(),
    "version".to_string(),
    "author".to_string(),
  ]
}

fn default_source() -> Vec<String> {
  vec![]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomType {
  pub strategy: String,
  pub name_path: Option<String>,
  pub path: String,
  #[serde(flatten)]
  pub unknown_fields: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyGroup {
  pub alias_name: String,
  #[serde(default)]
  pub dependencies: Vec<String>,
  #[serde(default)]
  pub dependency_types: Vec<String>,
  #[serde(default)]
  pub packages: Vec<String>,
  #[serde(default)]
  pub specifier_types: Vec<String>,
  #[serde(flatten)]
  pub unknown_fields: HashMap<String, Value>,
}

/// Raw deserialized config file. Converted to `Rcfile` via `From<RawRcfile>`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawRcfile {
  #[serde(rename = "$schema", skip_serializing)]
  _schema: Option<serde::de::IgnoredAny>,
  #[serde(default = "empty_custom_types")]
  pub custom_types: HashMap<String, CustomType>,
  #[serde(default)]
  pub dependency_groups: Vec<DependencyGroup>,
  #[serde(default = "default_false")]
  pub format_bugs: bool,
  #[serde(default = "default_false")]
  pub format_repository: bool,
  #[serde(default = "default_indent")]
  pub indent: Option<String>,
  #[serde(default = "default_max_concurrent_requests")]
  pub max_concurrent_requests: usize,
  #[serde(default)]
  pub semver_groups: Vec<AnySemverGroup>,
  #[serde(default = "default_sort_az")]
  pub sort_az: Vec<String>,
  #[serde(default = "default_sort_exports")]
  pub sort_exports: Vec<String>,
  #[serde(default = "sort_first")]
  pub sort_first: Vec<String>,
  #[serde(default = "default_true")]
  pub sort_packages: bool,
  #[serde(default = "default_source")]
  pub source: Vec<String>,
  #[serde(default = "default_false")]
  pub strict: bool,
  #[serde(default)]
  pub version_groups: Vec<AnyVersionGroup>,
  #[serde(flatten)]
  pub unknown_fields: HashMap<String, Value>,
}

impl RawRcfile {
  /// Handle config that is no longer supported or was hallucinated by an LLM
  pub fn visit_unknown_rcfile_fields(&self) {
    let mut is_valid = true;
    self.unknown_fields.iter().for_each(|(key, _)| match key.as_str() {
      "dependencyTypes" => {
        error!("Config property 'dependencyTypes' is deprecated");
        error!("Use CLI flag instead: --dependency-types prod,dev,peer");
        is_valid = false;
      }
      "specifierTypes" => {
        error!("Config property 'specifierTypes' is deprecated");
        error!("Use CLI flag instead: --specifier-types exact,range");
        is_valid = false;
      }
      "lintFormatting" => {
        error!("Config property 'lintFormatting' is deprecated");
        error!("Use 'syncpack format --check' to validate formatting");
        is_valid = false;
      }
      "lintSemverRanges" => {
        error!("Config property 'lintSemverRanges' is deprecated");
        error!("Semver range checking is always enabled in 'syncpack lint'");
        is_valid = false;
      }
      "lintVersions" => {
        error!("Config property 'lintVersions' is deprecated");
        error!("Version checking is always enabled in 'syncpack lint'");
        is_valid = false;
      }
      _ => {
        error!("Config property '{key}' is not recognised");
        is_valid = false;
      }
    });
    self.custom_types.iter().for_each(|(custom_type_name, value)| {
      value.unknown_fields.iter().for_each(|(field_name, _)| {
        error!("Config property 'customTypes.{custom_type_name}.{field_name}' is not recognised");
        is_valid = false;
      });
    });
    self.dependency_groups.iter().enumerate().for_each(|(index, value)| {
      value.unknown_fields.iter().for_each(|(key, _)| {
        error!("Config property 'dependencyGroups[{index}].{key}' is not recognised");
        is_valid = false;
      });
    });
    self.semver_groups.iter().enumerate().for_each(|(index, value)| {
      value.unknown_fields.iter().for_each(|(key, _)| {
        error!("Config property 'semverGroups[{index}].{key}' is not recognised");
        is_valid = false;
      });
    });
    self.version_groups.iter().enumerate().for_each(|(index, value)| {
      value.unknown_fields.iter().for_each(|(key, _)| {
        error!("Config property 'versionGroups[{index}].{key}' is not recognised");
        is_valid = false;
      });
    });
    if !is_valid {
      error!("syncpack will exit due to an invalid config file, see https://syncpack.dev for documentation");
      exit(1);
    }
  }
}

fn validate_or_exit(result: Result<(), String>) {
  if let Err(msg) = result {
    error!("{msg}");
    error!("check your syncpack config file");
    exit(1);
  }
}

fn validate_raw_dep_types(raw: &[String], all: &[DependencyType]) -> Result<(), String> {
  for s in raw {
    let name = s.trim_start_matches('!');
    if name != "**" && !all.iter().any(|dt| dt.name == name) {
      return Err(format!(
        "dependencyType '{name}' does not match any of syncpack or your customTypes"
      ));
    }
  }
  Ok(())
}

impl From<RawRcfile> for Rcfile {
  fn from(raw: RawRcfile) -> Self {
    let all_dependency_types = compute_all_dependency_types(&raw.custom_types);
    let dependency_groups = raw
      .dependency_groups
      .into_iter()
      .map(|dg| {
        let selector = GroupSelector::new(dg.dependencies, dg.dependency_types, dg.alias_name, dg.packages, dg.specifier_types);
        validate_or_exit(selector.validate_dependency_types(&all_dependency_types));
        selector
      })
      .collect();
    let mut semver_groups = vec![SemverGroup::get_exact_local_specifiers()];
    for group_config in raw.semver_groups {
      let semver_group = SemverGroup::from_config(group_config);
      validate_or_exit(semver_group.selector.validate_dependency_types(&all_dependency_types));
      semver_groups.push(semver_group);
    }
    semver_groups.push(SemverGroup::get_catch_all());
    raw.version_groups.iter().for_each(|group| {
      validate_or_exit(validate_raw_dep_types(&group.dependency_types, &all_dependency_types));
    });

    Rcfile {
      dependency_groups,
      format_bugs: raw.format_bugs,
      format_repository: raw.format_repository,
      indent: raw.indent,
      max_concurrent_requests: raw.max_concurrent_requests,
      semver_groups,
      sort_az: raw.sort_az,
      sort_exports: raw.sort_exports,
      sort_first: raw.sort_first,
      sort_packages: raw.sort_packages,
      source: raw.source,
      strict: raw.strict,
      version_groups: raw.version_groups,
      all_dependency_types,
    }
  }
}

#[derive(Debug)]
pub struct Rcfile {
  pub dependency_groups: Vec<GroupSelector>,
  pub format_bugs: bool,
  pub format_repository: bool,
  pub indent: Option<String>,
  pub max_concurrent_requests: usize,
  pub semver_groups: Vec<SemverGroup>,
  pub sort_az: Vec<String>,
  pub sort_exports: Vec<String>,
  pub sort_first: Vec<String>,
  pub sort_packages: bool,
  pub source: Vec<String>,
  pub strict: bool,
  pub version_groups: Vec<AnyVersionGroup>,
  /// All dependency types (built-in + custom). Computed after deserialization.
  pub all_dependency_types: Vec<DependencyType>,
}

impl Default for Rcfile {
  fn default() -> Self {
    serde_json::from_str::<RawRcfile>("{}")
      .expect("An empty object should produce a default Rcfile")
      .into()
  }
}

impl Rcfile {
  /// Create every version group defined in the rcfile.
  pub fn get_version_groups(&mut self, packages: &Packages) -> Vec<VersionGroup> {
    let mut all_groups: Vec<VersionGroup> = mem::take(&mut self.version_groups)
      .into_iter()
      .map(|group_config| VersionGroup::from_config(group_config, packages))
      .collect();
    all_groups.push(VersionGroup::get_catch_all());
    all_groups
  }
}
