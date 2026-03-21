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
#[cfg(test)]
mod package_json_test;
mod packages;
#[cfg(test)]
mod packages_test;
mod pattern_matcher;
mod rcfile;
#[cfg(test)]
mod rcfile_test;
mod registry_client;
#[cfg(test)]
mod registry_client_test;
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
      let ctx = visit_packages(ctx);
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
      let ctx = visit_packages(ctx);
      lint::run(ctx)
    }
    Subcommand::Update => {
      let mut ctx = ctx;
      let client: Arc<dyn registry_client::RegistryClient> = Arc::new(LiveRegistryClient::new());
      ctx.fetch_all_updates(&client).await;
      let ctx = visit_packages(ctx);
      update::run(ctx)
    }
    Subcommand::List => {
      let ctx = visit_packages(ctx);
      list::run(ctx)
    }
    Subcommand::Json => {
      let ctx = visit_packages(ctx);
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
