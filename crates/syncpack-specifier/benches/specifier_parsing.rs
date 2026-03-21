use {
  criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion},
  syncpack_specifier::Specifier,
};

/// Inputs covering every Specifier variant
const INPUTS: &[(&str, &str)] = &[
  // Simple semver
  ("exact", "1.2.3"),
  ("exact_tag", "1.2.3-beta.1"),
  ("major", "1"),
  ("minor", "1.2"),
  ("latest_star", "*"),
  ("latest_keyword", "latest"),
  // Ranges
  ("range_caret", "^1.2.3"),
  ("range_tilde", "~1.2.3"),
  ("range_gt", ">1.2.3"),
  ("range_gte", ">=1.2.3"),
  ("range_lt", "<1.2.3"),
  ("range_lte", "<=1.2.3"),
  ("range_caret_tag", "^1.2.3-beta.1"),
  ("range_major_caret", "^1"),
  ("range_minor_tilde", "~1.2"),
  // Complex
  ("complex_or", ">=1.0.0 <2.0.0 || >=3.0.0"),
  ("complex_and", ">=1.0.0 <2.0.0"),
  // Workspace protocol
  ("workspace_star", "workspace:*"),
  ("workspace_caret", "workspace:^1.2.3"),
  // Non-semver
  ("catalog", "catalog:react18"),
  ("alias", "npm:lodash@^4.17.21"),
  ("tag", "beta"),
  ("git_github", "github:user/repo#v1.2.3"),
  ("file", "file:../packages/foo"),
  ("link", "link:../packages/foo"),
  ("url", "https://example.com/package.tgz"),
  ("unsupported", "}wat{"),
  ("empty", ""),
];

fn bench_specifier_create(c: &mut Criterion) {
  let mut group = c.benchmark_group("Specifier::create");
  for &(name, input) in INPUTS {
    group.bench_with_input(BenchmarkId::new("create", name), input, |b, input| {
      b.iter(|| Specifier::create(black_box(input)))
    });
  }
  group.finish();
}

fn bench_specifier_new_cached(c: &mut Criterion) {
  let mut group = c.benchmark_group("Specifier::new_cached");
  for &(name, input) in INPUTS {
    // Prime the cache
    let _ = Specifier::new(input);
    group.bench_with_input(BenchmarkId::new("cached", name), input, |b, input| {
      b.iter(|| Specifier::new(black_box(input)))
    });
  }
  group.finish();
}

fn bench_specifier_new_cold(c: &mut Criterion) {
  let mut group = c.benchmark_group("Specifier::new_cold");
  for &(name, input) in INPUTS {
    group.bench_with_input(BenchmarkId::new("cold", name), input, |b, input| {
      // Clear cache each iteration by creating unique input
      b.iter(|| Specifier::create(black_box(input)))
    });
  }
  group.finish();
}

fn bench_parser_functions(c: &mut Criterion) {
  use syncpack_specifier::parser;

  let mut group = c.benchmark_group("parser");

  let semver_inputs = &[
    ("exact", "1.2.3"),
    ("range_caret", "^1.2.3"),
    ("range_tilde", "~1.2.3"),
    ("major", "1"),
    ("minor", "1.2"),
    ("latest", "latest"),
    ("non_match", "github:user/repo"),
  ];

  for &(name, input) in semver_inputs {
    group.bench_with_input(BenchmarkId::new("is_simple_semver", name), input, |b, input| {
      b.iter(|| parser::is_simple_semver(black_box(input)))
    });
    group.bench_with_input(BenchmarkId::new("is_exact", name), input, |b, input| {
      b.iter(|| parser::is_exact(black_box(input)))
    });
    group.bench_with_input(BenchmarkId::new("is_range", name), input, |b, input| {
      b.iter(|| parser::is_range(black_box(input)))
    });
  }

  let complex_inputs = &[
    ("simple_range", "^1.2.3"),
    ("or_range", ">=1.0.0 <2.0.0 || >=3.0.0"),
    ("and_range", ">=1.0.0 <2.0.0"),
    ("non_match", "latest"),
  ];

  for &(name, input) in complex_inputs {
    group.bench_with_input(BenchmarkId::new("is_complex_range", name), input, |b, input| {
      b.iter(|| parser::is_complex_range(black_box(input)))
    });
  }

  group.finish();
}

fn bench_batch_parsing(c: &mut Criterion) {
  // Simulate realistic workload: parsing many specifiers as syncpack would
  let realistic_inputs: Vec<&str> = vec![
    "^1.2.3",
    "~2.0.0",
    "1.0.0",
    ">=1.0.0",
    "^3.4.5",
    "latest",
    "workspace:*",
    "^0.1.0",
    "~0.2.0",
    ">=1.0.0 <2.0.0",
    "^16.8.0",
    "^17.0.0",
    "^18.0.0",
    "~4.3.0",
    "1.2.3",
    "^5.0.0-beta.1",
    "npm:lodash@^4.17.21",
    "^1.0.0",
    "~1.0.0",
    ">=2.0.0",
  ];

  c.bench_function("batch_20_specifiers", |b| {
    b.iter(|| {
      for input in &realistic_inputs {
        black_box(Specifier::create(black_box(input)));
      }
    })
  });
}

criterion_group!(
  benches,
  bench_specifier_create,
  bench_specifier_new_cached,
  bench_specifier_new_cold,
  bench_parser_functions,
  bench_batch_parsing,
);
criterion_main!(benches);
