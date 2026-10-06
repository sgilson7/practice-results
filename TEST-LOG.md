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

### M2 practice set (crates/core/src/set.rs), 9 tests

Broken by: `parse_set` stubbed to return `Err("stub")`.

| Test | Failed at | Expected | Got |
|---|---|---|---|
| sample_set_has_12_questions_in_4_topics | set.rs:172 | sample set parses | `Err("stub")` |
| sample_set_is_marked_as_sample | set.rs:172 | sample set parses | `Err("stub")` |
| rejects_malformed_json | set.rs:209 | message starting "The practice set is not valid" | "stub" |
| rejects_unknown_field | set.rs:216 | message naming `extra` | "stub" |
| rejects_answer_not_in_choices | set.rs:224 | message naming q01 | "stub" |
| rejects_unknown_topic | set.rs:232 | message naming q01 and "fractions" | "stub" |
| rejects_duplicate_question_id | set.rs:240 | `Question "q01" appears more than once.` | "stub" |
| rejects_topic_without_questions | set.rs:248 | `Topic "percent" has no questions.` | "stub" |
| rejects_id_unsafe_for_file_names | set.rs:256 | message naming the bad id | "stub" |

Restored: 9 passed.

### M3 rates and threshold (crates/core/src/rate.rs), 6 tests

Broken by: `percent_floor` and `below_threshold` stubbed to `None`,
`parse_threshold` to `Err("stub")`, `DEFAULT_THRESHOLD` set to 0.

| Test | Failed at | Expected | Got |
|---|---|---|---|
| percent_floor_hand_values | rate.rs:71 | `Some(66)` for 6/9 | `None` |
| below_threshold_boundary | rate.rs:90 | `Some(false)` for 7/10 at 70 | `None` |
| shown_percent_and_suggestion_never_disagree | rate.rs:98 | a percentage | panic on `None` |
| parse_threshold_accepts_whole_numbers_0_to_100 | rate.rs:113 | `Ok(70)` | `Err("stub")` |
| parse_threshold_rejects_everything_else | rate.rs:121 | message "whole number from 0 to 100" | "stub" |
| default_threshold_is_70 | rate.rs:127 | 70 | 0 |

Restored: 15 passed (with M2).

### M4 student flow and result file (crates/core/src/student.rs, result.rs), 13 tests

The tests were first run green once by mistake, then broken before commit:
every new function stubbed (`fnv1a64` = 0, `canonical_text` = "",
`clean_nickname` accepts anything, `result_file_name` = "", `build_result`
returns an empty file, `start` = no answers, `answer`/`check` return the state
unchanged, `correct_flags`/`topic_scores` empty, `view`/`save` = `Err("stub")`).

| Test | Failed at | Expected | Got |
|---|---|---|---|
| fnv1a64_matches_reference_vectors | result.rs:173 | 0xcbf29ce484222325 (14695981039346656037) | 0 |
| canonical_text_layout | result.rs:184 | the five-line canonical text | "" |
| nickname_rules | result.rs:192 | "" rejected | accepted |
| file_name_uses_set_id_and_nickname | result.rs:208 | practice-result-ratios-sample-1-blue-fox.json | "" |
| saved_file_holds_only_allowed_fields | result.rs:218 | JSON with five keys | empty text, parse panic |
| build_rejects_bad_nickname_and_wrong_length | result.rs:237 | Err for " " | Ok |
| new_attempt_has_no_answers_and_is_unlocked | student.rs:272 | 12 × None | [] |
| answer_records_choice_and_rejects_out_of_range | student.rs:285 | answers[0] = Some(1) | index panic (answer not recorded) |
| check_requires_every_answer | student.rs:297 | "Not answered yet: 4, 7, 11." | index panic (no answers kept) |
| answers_are_locked_after_check | student.rs:309 | checked = true | false |
| topic_scores_hand_computed | student.rs:322 | 2/3, 1/3, 3/3, 2/3 | view Err("stub") |
| save_requires_check | student.rs:345 | "Check your answers before saving." | "stub" |
| tampered_state_is_rejected | student.rs:355 | message "11 answers" | "stub" |

Restored: 28 passed.

## 2. Corrective dialogue
