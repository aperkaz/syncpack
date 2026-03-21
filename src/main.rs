use {
  crate::{
    cli::{Cli, ReporterKind, Subcommand},
    commands::{
      fix, fix_mismatches, format, json, lint, lint_semver_ranges, list, list_mismatches, prompt,
      reporter::{JsonFixReporter, JsonFormatReporter, PrettyFixReporter, PrettyFormatReporter},
      set_semver_ranges, update,
    },
    context::Context,
    registry_client::LiveRegistryClient,
    registry_updates::RegistryUpdates,
    visit_formatting::visit_formatting,
    visit_packages::visit_packages,
  },
  std::{process::exit, sync::Arc},
};

#[cfg(test)]
#[path = "test/test.rs"]
mod test;

mod catalogs;
mod cli;
mod commands;
mod config;
mod context;
mod dependency;
mod dependency_type;
mod group_selector;
mod instance;
mod instance_state;
mod logger;
mod package_json;
mod packages;
mod pattern_matcher;
mod rcfile;
mod registry_client;
mod registry_updates;
mod semver_group;
mod semver_range;
mod specifier;
mod version_group;
mod visit_formatting;
mod visit_packages;

#[tokio::main]
async fn main() {
  let cli = Cli::parse();

  logger::init(&cli);

  let ctx = Context::from_cli(cli);

  let exit_code = match ctx.config.cli.subcommand {
    Subcommand::Fix => {
      let ctx = visit_packages(ctx, None);
      let pretty = PrettyFixReporter;
      let json_reporter = JsonFixReporter;
      let reporter: &dyn commands::reporter::FixReporter = match ctx.config.cli.reporter {
        ReporterKind::Pretty => &pretty,
        ReporterKind::Json => &json_reporter,
      };
      fix::run(ctx, reporter)
    }
    Subcommand::Format => {
      let ctx = visit_formatting(ctx);
      let pretty = PrettyFormatReporter;
      let json_reporter = JsonFormatReporter;
      let reporter: &dyn commands::reporter::FormatReporter = match ctx.config.cli.reporter {
        ReporterKind::Pretty => &pretty,
        ReporterKind::Json => &json_reporter,
      };
      format::run(ctx, reporter)
    }
    Subcommand::Lint => {
      let ctx = visit_packages(ctx, None);
      lint::run(ctx)
    }
    Subcommand::Update => {
      let client: Arc<dyn registry_client::RegistryClient> = Arc::new(LiveRegistryClient::new());
      let updates = RegistryUpdates::fetch(
        &client,
        &ctx.version_groups,
        &ctx.instances,
        ctx.config.rcfile.max_concurrent_requests,
      )
      .await;
      let ctx = visit_packages(ctx, Some(&updates));
      update::run(ctx, &updates)
    }
    Subcommand::List => {
      let ctx = visit_packages(ctx, None);
      list::run(ctx)
    }
    Subcommand::Json => {
      let ctx = visit_packages(ctx, None);
      json::run(ctx)
    }
    Subcommand::ListMismatches => list_mismatches::run(),
    Subcommand::LintSemverRanges => lint_semver_ranges::run(),
    Subcommand::FixMismatches => fix_mismatches::run(),
    Subcommand::SetSemverRanges => set_semver_ranges::run(),
    Subcommand::Prompt => prompt::run(),
  };

  exit(exit_code);
}
