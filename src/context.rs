use {
  crate::{
    catalogs::CatalogsByName,
    cli::Cli,
    config::Config,
    instance::{Instance, InstanceIdx},
    packages::Packages,
    version_group::VersionGroup,
  },
  log::debug,
};

/// The central data structure that owns all project data.
#[derive(Debug)]
pub struct Context {
  /// If present, the contents of each bun or pnpm catalog. The default catalog
  /// is keyed under "default" and named by their names.
  ///
  /// - https://pnpm.io/catalogs
  /// - https://bun.sh/docs/pm/catalogs
  #[allow(dead_code)]
  pub catalogs: Option<CatalogsByName>,
  /// All default configuration with user config applied
  pub config: Config,
  /// Every instance in the project (arena — owns all instances).
  pub instances: Vec<Instance>,
  /// Every package.json in the project
  pub packages: Packages,
  /// All version groups, their dependencies, and their instances
  pub version_groups: Vec<VersionGroup>,
}

impl Context {
  /// Read all configuration and package.json files, collect all dependency
  /// instances, and assign them to version groups.
  pub fn create(config: Config, packages: Packages, catalogs: Option<CatalogsByName>) -> Self {
    let mut instances = vec![];
    let all_dependency_types = config.rcfile.get_all_dependency_types();
    let cli_filters = config.cli.get_filters(&packages, &all_dependency_types);
    let dependency_groups = config.rcfile.get_dependency_groups(&packages, &all_dependency_types);
    let semver_groups = config.rcfile.get_semver_groups(&packages, &all_dependency_types);
    let mut version_groups = config.rcfile.get_version_groups(&packages, &all_dependency_types);

    packages.get_all_instances(&all_dependency_types, |mut descriptor| {
      let dependency_group = dependency_groups.iter().find(|alias| alias.can_add(&descriptor));

      if let Some(group) = dependency_group {
        descriptor.internal_name = group.label.clone();
      }

      descriptor.matches_cli_filter = match cli_filters.as_ref() {
        Some(filters) => filters.can_add(&descriptor),
        None => true,
      };

      if !descriptor.matches_cli_filter {
        return;
      }

      let preferred_semver_range = semver_groups
        .iter()
        .find(|group| group.selector.can_add(&descriptor))
        .and_then(|group| group.range.clone());

      let version_group = version_groups.iter_mut().find(|group| group.selector.can_add(&descriptor));

      let instance = Instance::new(descriptor, preferred_semver_range);
      let idx = InstanceIdx(instances.len());
      instances.push(instance);

      if let Some(group) = version_group {
        group.add_instance(idx, &instances[idx.0]);
      }
    });

    Self {
      catalogs,
      config,
      instances,
      packages,
      version_groups,
    }
  }

  pub fn from_cli(cli: Cli) -> Self {
    let config = Config::from_cli(cli);

    debug!("Command: {:?}", config.cli.subcommand);
    debug!("{:#?}", config.cli);
    debug!("{:#?}", config.rcfile);

    let packages = Packages::from_config(&config);
    let catalogs: Option<CatalogsByName> = None; // catalogs::from_config(&config);

    Context::create(config, packages, catalogs)
  }
}
