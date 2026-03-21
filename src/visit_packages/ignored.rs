use {
  super::indent::{L1, L2},
  crate::{context::Context, instance::ValidInstance},
  log::debug,
};

#[cfg(test)]
#[path = "ignored_test.rs"]
mod ignored_test;

pub fn visit(dependency: &crate::dependency::Dependency, ctx: &Context) {
  debug!("visit ignored version group");
  debug!("{L1}visit dependency '{}'", dependency.internal_name);
  for &idx in &dependency.instances {
    let instance = &ctx.instances[idx.0];
    let actual_specifier = &instance.descriptor.specifier;
    debug!("{L2}visit instance '{}' ({actual_specifier:?})", instance.id);
    instance.mark_valid(ValidInstance::IsIgnored, &instance.descriptor.specifier);
  }
}
