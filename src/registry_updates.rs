use {
  crate::{
    dependency::UpdateUrl,
    registry_client::{AllPackageVersions, RegistryClient, RegistryError},
    specifier::Specifier,
    version_group::VersionGroup,
  },
  indicatif::{MultiProgress, ProgressBar, ProgressStyle},
  log::debug,
  std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
    time::Duration,
  },
  tokio::{
    sync::Semaphore,
    task::{spawn, JoinHandle},
  },
};

/// The result of fetching package versions from the npm registry.
pub struct RegistryUpdates {
  /// All updates from the npm registry keyed by internal dependency name
  pub updates_by_internal_name: HashMap<String, Vec<Rc<Specifier>>>,
  /// The internal names of all failed updates
  pub failed: Vec<String>,
}

impl RegistryUpdates {
  /// Fetch every version specifier ever published for all updateable
  /// dependencies in the project.
  pub async fn fetch(client: &Arc<dyn RegistryClient>, version_groups: &[VersionGroup], max_concurrent_requests: usize) -> Self {
    let client = Arc::clone(client);
    let semaphore = Arc::new(Semaphore::new(max_concurrent_requests));
    let progress_bars = Arc::new(MultiProgress::new());
    let mut handles: Vec<(String, JoinHandle<Result<AllPackageVersions, RegistryError>>)> = vec![];
    let mut updates_by_internal_name: HashMap<String, Vec<Rc<Specifier>>> = HashMap::new();
    let mut failed: Vec<String> = vec![];

    for update_url in get_unique_update_urls(version_groups) {
      let permit = Arc::clone(&semaphore).acquire_owned().await;
      let client = Arc::clone(&client);
      let progress_bars = Arc::clone(&progress_bars);

      handles.push((
        update_url.internal_name.clone(),
        spawn(async move {
          let _permit = permit;
          let progress_bar = progress_bars.add(ProgressBar::new_spinner());
          progress_bar.enable_steady_tick(Duration::from_millis(100));
          progress_bar.set_style(ProgressStyle::default_spinner());
          progress_bar.set_message(update_url.internal_name.clone());
          let package_meta = client.fetch(&update_url).await;
          progress_bar.finish_and_clear();
          progress_bars.remove(&progress_bar);
          package_meta
        }),
      ));
    }

    for (internal_name, handle) in handles {
      match handle.await {
        Ok(result) => match result {
          Ok(package_meta) => {
            let all_updates = updates_by_internal_name.entry(internal_name.clone()).or_default();
            for version in package_meta.versions.iter() {
              if !version.contains("created") && !version.contains("modified") {
                all_updates.push(Specifier::new(version));
              }
            }
          }
          Err(err) => {
            debug!("{err}");
            failed.push(internal_name);
          }
        },
        Err(err) => {
          debug!("{err}");
          failed.push(internal_name);
        }
      }
    }

    Self {
      updates_by_internal_name,
      failed,
    }
  }
}

/// Return a list of every dependency we should query the registry for
/// updates. We use internal names in order to support dependency groups,
/// where many dependencies can be aliased as one.
fn get_unique_update_urls(version_groups: &[VersionGroup]) -> HashSet<UpdateUrl> {
  version_groups.iter().fold(HashSet::new(), |mut unique_update_urls, group| {
    group.get_update_urls().inspect(|update_urls| {
      update_urls.iter().for_each(|url| {
        unique_update_urls.insert(url.clone());
      });
    });
    unique_update_urls
  })
}
