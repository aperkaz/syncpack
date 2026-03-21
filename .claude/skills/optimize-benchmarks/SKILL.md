---
name: optimize-benchmarks
description: Iterative performance optimisation loop for syncpack-specifier. Runs benchmarks, identifies bottlenecks, applies optimisations, verifies tests pass and benchmarks improve, then repeats.
---

# Optimize Benchmarks

Iterative loop: benchmark, optimise, test, verify improvement, repeat.

## Workflow

### 1. Baseline

```bash
cargo bench -p syncpack-specifier -- --save-baseline before 2>&1 | tail -40
```

Save the output. Identify which variants are slowest.

### 2. Identify Bottleneck

From the full baseline, focus on the **slowest** benchmarks first.

Priority order for specifier parsing:

1. `Specifier::create` — the main parse function, called for every version string
2. `parser::is_range` — checks 12 regexes sequentially
3. `parser::is_exact` — checks 4 regexes sequentially
4. `parser::is_complex_range` — splits, collects, iterates
5. Individual regex matches in `regexes.rs`

### 3. Apply ONE Optimisation

Make a **single, focused change**. Do NOT bundle multiple optimisations — each must be independently measurable.

### 4. Verify Tests Pass

```bash
cargo test -p syncpack-specifier 2>&1 | tail -5
```

If tests fail, fix or revert. Never proceed with failing tests.

### 5. Benchmark Against Baseline

```bash
cargo bench -p syncpack-specifier -- --baseline before 2>&1 | tail -40
```

Look for `[-XX.XXX% ...]` (improvement) or `[+XX.XXX% ...]` (regression).

### 6. Evaluate

- **Improved:** Report the gains. Update baseline: `cargo bench -p syncpack-specifier -- --save-baseline before`. Continue to step 2.
- **No change:** Revert and try a different approach.
- **Regressed:** Revert immediately.

### 7. Repeat

Go to step 2. Stop when:

- User says stop
- No bottlenecks remain
- Gains are <1% across all benchmarks

## Known Optimisation Opportunities

### High Impact

**Replace regex with char-based parsing in `parser.rs` / `regexes.rs`**

Most regexes in `regexes.rs` match simple patterns like `^[0-9]+\.[0-9]+\.[0-9]+$` (exact semver). These can be replaced with byte/char iteration:

```rust
// Instead of regex EXACT: r"^[0-9]+\.[0-9]+\.[0-9]+$"
fn is_exact_version(s: &str) -> bool {
  let mut dots = 0;
  let bytes = s.as_bytes();
  if bytes.is_empty() { return false; }
  for &b in bytes {
    match b {
      b'0'..=b'9' => {},
      b'.' => dots += 1,
      _ => return false,
    }
  }
  dots == 2
}
```

Regex `is_match()` has overhead even for simple patterns: engine setup, capture group allocation. Char-based parsing for these patterns is 5-20x faster.

**Reduce sequential regex attempts in `parser::is_range`**

`is_range` tries 12 regexes. Instead, match on first char(s) to dispatch:

```rust
fn is_range(s: &str) -> bool {
  match s.as_bytes().first() {
    Some(b'^') => is_semver_after(s, 1) || is_semver_tag_after(s, 1),
    Some(b'~') => is_semver_after(s, 1) || is_semver_tag_after(s, 1),
    Some(b'>') => { /* check >= vs > then validate remainder */ },
    Some(b'<') => { /* check <= vs < then validate remainder */ },
    _ => false,
  }
}
```

**Consolidate related regex patterns**

Many regexes are pairs: `EXACT` + `EXACT_TAG`, `CARET` + `CARET_TAG`, etc. Merge each pair into one function that handles both cases:

```rust
fn is_exact(s: &str) -> bool {
  // Parse digits.digits.digits, then optionally -tag
  let rest = parse_semver_triple(s)?;
  rest.is_empty() || rest.starts_with('-')
}
```

### Medium Impact

**Replace `lazy_static` with `std::sync::OnceLock`**

`lazy_static` uses an extra indirection layer. `OnceLock` (stable since Rust 1.80) is zero-cost after init:

```rust
use std::sync::OnceLock;

fn exact_regex() -> &'static Regex {
  static RE: OnceLock<Regex> = OnceLock::new();
  RE.get_or_init(|| Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+$").unwrap())
}
```

But if regex is being replaced with char-based parsing, this becomes irrelevant.

**Reorder checks in `Specifier::create` by frequency**

In a typical monorepo, most specifiers are `^x.y.z` (range) or `x.y.z` (exact). The current order already checks exact first, then range — good. But `is_exact` tries 4 regex patterns. A single fast char check can short-circuit:

```rust
// Fast path: first char is digit → likely exact or major or minor
// Fast path: first char is ^ or ~ → likely range
```

**Avoid `String` allocation in `strip_semver_range`**

`strip_semver_range` returns `&str` (already good), but callers like `Range::create` then `.to_string()` the result. Consider whether the allocation can be deferred.

### Low Impact

- Replace `HashMap` in caches with `FxHashMap` (faster hashing for short strings)
- Use `SmallString` or stack-allocated strings for short specifiers
- Pre-size cache `HashMap` with expected capacity

## Architecture Notes

Key files in `crates/syncpack-specifier/src/`:

| File                         | Role                                                        |
| ---------------------------- | ----------------------------------------------------------- |
| `lib.rs`                     | `Specifier` enum, `create()` dispatch, caches               |
| `parser.rs`                  | `is_exact()`, `is_range()`, etc. — classification functions |
| `regexes.rs`                 | All `lazy_static` regex patterns                            |
| `exact.rs`, `range.rs`, etc. | Variant constructors calling `node_semver`                  |
| `semver_range.rs`            | `SemverRange` enum, `parse()`                               |

The hot path is: `Specifier::create()` → `parser::is_*()` → `regexes::*` → variant `::create()` → `node_semver` parsing.

Optimising the `parser::is_*` layer gives the biggest wins because it runs for **every** specifier, and most of the time most checks return `false` (only one branch matches).

## Rules

- ONE change per iteration
- Always verify tests pass before benchmarking
- Always compare against baseline
- Report numbers: `before → after (% change)`
- Revert regressions immediately
- Don't optimise what doesn't show up in benchmarks
- Use **fast iteration** (`"batch"` filter) during the loop, **full suite** only at start and end
