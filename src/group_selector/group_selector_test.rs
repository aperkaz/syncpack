use {
  crate::{
    dependency::{DependencyType, Strategy},
    group_selector::GroupSelector,
    instance::InstanceDescriptor,
    package_json::PackageJson,
    specifier::Specifier,
  },
  serde_json::json,
  std::{cell::RefCell, path::PathBuf, rc::Rc},
};

fn make_dep_type() -> DependencyType {
  DependencyType {
    name: "prod".to_string(),
    name_path: None,
    path: "/dependencies".to_string(),
    strategy: Strategy::VersionsByName,
  }
}

fn make_package(name: &str) -> Rc<RefCell<PackageJson>> {
  Rc::new(RefCell::new(PackageJson {
    name: name.to_string(),
    file_path: PathBuf::from(format!("/packages/{name}/package.json")),
    formatting_mismatches: RefCell::new(vec![]),
    raw: RefCell::new("{}".to_string()),
    contents: RefCell::new(json!({"name": name})),
    detected_indent: "  ".to_string(),
    detected_newline: "\n".to_string(),
  }))
}

fn descriptor(name: &str, is_local_dependency: bool) -> InstanceDescriptor {
  InstanceDescriptor {
    dependency_type: make_dep_type(),
    internal_name: name.to_string(),
    is_local_dependency,
    matches_cli_filter: false,
    name: name.to_string(),
    package: make_package("owner-pkg"),
    specifier: Specifier::new("1.0.0"),
  }
}

fn selector(deps: Vec<&str>) -> GroupSelector {
  GroupSelector::new(
    deps.into_iter().map(|s| s.to_string()).collect(),
    vec![],
    "test".to_string(),
    vec![],
    vec![],
  )
}

#[test]
fn local_keyword_includes_local_deps() {
  let s = selector(vec!["$LOCAL"]);
  assert!(s.can_add(&descriptor("bar", true)));
  assert!(!s.can_add(&descriptor("baz", false)));
}

#[test]
fn not_local_keyword_excludes_local_deps() {
  let s = selector(vec!["!$LOCAL"]);
  assert!(!s.can_add(&descriptor("bar", true)));
  assert!(s.can_add(&descriptor("baz", false)));
}

#[test]
fn local_plus_named_pattern_matches_either() {
  // $LOCAL OR named pattern — either is sufficient
  let s = selector(vec!["$LOCAL", "react"]);
  assert!(s.can_add(&descriptor("bar", true))); // local dep
  assert!(s.can_add(&descriptor("react", false))); // matches pattern
  assert!(!s.can_add(&descriptor("webpack", false))); // neither
}

#[test]
fn not_local_excludes_even_if_named_pattern_matches() {
  // !$LOCAL exclusion wins over named include patterns
  let s = selector(vec!["!$LOCAL", "react"]);
  assert!(!s.can_add(&descriptor("react", true))); // local: excluded
  assert!(s.can_add(&descriptor("react", false))); // not local: ok
  assert!(!s.can_add(&descriptor("webpack", false))); // neither matches include
}

#[test]
fn empty_includes_with_not_local_includes_non_local_excludes_local() {
  // !$LOCAL only → non-local included by default, local excluded
  let s = selector(vec!["!$LOCAL"]);
  assert!(s.can_add(&descriptor("anything", false)));
  assert!(!s.can_add(&descriptor("local-pkg", true)));
}

#[test]
fn dollar_local_literal_not_matched_as_pattern() {
  // A dep literally named "$LOCAL" should not be matched by the $LOCAL keyword
  // (it won't exist in practice, but verify it's not treated as a glob pattern)
  let s = selector(vec!["$LOCAL"]);
  // "$LOCAL" as a name with is_local_dependency=false should NOT match
  assert!(!s.can_add(&descriptor("$LOCAL", false)));
  // A real local dep should match
  assert!(s.can_add(&descriptor("some-local-pkg", true)));
}
