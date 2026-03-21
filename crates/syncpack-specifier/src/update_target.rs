#[derive(Debug)]
pub enum UpdateTarget {
  /// "*.*.*"
  Latest,
  /// "1.*.*"
  Minor,
  /// "1.2.*"
  Patch,
}
