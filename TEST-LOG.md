# TEST-LOG

Two parts:

1. **Watched failing**: every new test was first run while broken (stub, wrong
   expected value, or injected violation). Each entry: the test, how it was
   broken, file:line of the failure, expected vs. got, and the fix.
2. **Corrective dialogue**: real failures found while building, written down as
   file and line, what was expected, what happened, and the fix (rules are
   fixed in the core, never in the page).

## 1. Watched failing

### M1 boundary (crates/core/tests/boundary.rs), 7 tests

Run 1: violations injected into real files (`itoa = "1"` in core Cargo.toml;
`f64`, `0.5`, `HashMap`, `std::time::SystemTime` in core lib.rs; an `if` in the shim).

| Test | Failed at | Expected | Got |
|---|---|---|---|
| core_dependencies_are_only_serde_and_serde_json | boundary.rs:357 | no dependency outside [serde, serde_json] | `["itoa"]` |
| core_source_has_no_float_hashmap_clock_or_io | boundary.rs:380 | no violations | lib.rs:5 `f64`, `0.5`; lib.rs:6 `HashMap`; lib.rs:7 `time`, `SystemTime` |
| shim_decides_nothing | boundary.rs:393 | no decision words | wasm lib.rs:2 `if`, `else` |

Run 2a: scanners mutated to report nothing; dotted `[x-dependencies.name]` form ignored.

| Test | Failed at | Expected | Got |
|---|---|---|---|
| scanner_catches_each_kind | boundary.rs:421 | a `float` violation in `let x = 0.5;` | `[]` |
| dependency_reader_sees_every_form | boundary.rs:401 | `[serde, regex, cc, libc]` | `[serde, regex, libc]` |
| shim_scanner_catches_decisions | boundary.rs:449 | violations for `if`/`else` | none |

Run 2b: comment/string stripping disabled.

| Test | Failed at | Expected | Got |
|---|---|---|---|
| scanner_ignores_comments_strings_hex_and_ranges | boundary.rs:443 | no false positives | 11 false positives from comments and strings |

Restored: 7 passed.

## 2. Corrective dialogue
