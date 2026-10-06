# PLAN: practice results by topic

Goal: a static browser tool with two views. Student view: practise a set, see a
score per topic, save a result file (nickname, set id, one entry per question:
question id, topic, correct). Teacher view: open many result files at once and
see, per topic, the share of correct answers, the questions most often missed,
and a suggestion to reteach any topic below a threshold (default 70%).
Everything runs on the device that opens the page. Nothing is sent anywhere.

Each step below gives the decision, the reason, and the test that shows it
holds. Steps are in build order. Each core rule lands in the same commit as its
test. Every new test is first seen failing (by a stub, a wrong expected value or
an injected violation); the failing run is logged in TEST-LOG.md.

## M0. Skeleton

| Decision | Reason | Test |
|---|---|---|
| Cargo workspace with `crates/core` (lib `practice_core`) and `crates/wasm` (cdylib `practice_wasm`). `target/` and `dist/` in `.gitignore`. Commit `Cargo.lock`. | The brief's shared technical setting; the lock file makes builds repeatable. | `cargo test` at the root runs every test. |

## M1. Boundary first

| Decision | Reason | Test |
|---|---|---|
| A test reads `crates/core/Cargo.toml` and fails on any dependency (any `*dependencies` section, including dev and build) other than `serde` and `serde_json`. | Rule (a). Checking it from the start means no later step can slip in a crate. | `boundary.rs::core_dependencies_are_only_serde_and_serde_json` |
| A test scans every `.rs` file under `crates/core/src` (comments and string literals removed) and fails on `f32`/`f64`, a float literal, `HashMap`/`HashSet`/hash-map paths, clocks (`SystemTime`, `Instant`, `std::time`, `chrono`), randomness, and input/output (`std::fs`, `std::io`, `std::net`, `std::env`, `std::process`, `println!` and friends). The message names file, line and token. | Rules (e) and "the core performs no input or output". | `boundary.rs::core_source_has_no_float_hashmap_clock_or_io`, plus scanner self-tests that prove the scanner catches each kind and ignores comments, strings, hex and ranges. |
| A test scans `crates/wasm/src` for decision keywords (`if`, `else`, `match`, `for`, `while`, `loop`, `?`). | Rule (b): the shim only forwards. | `boundary.rs::shim_decides_nothing` |
| A test scans `web/*.js` for rule-like code: numeric literals other than 0 and 1, `Math.`, `parseInt`, `parseFloat`, `toFixed`, `Number(`, and any `http:`/`https:` URL. | Rule (c) and (d): thresholds, percentages and counts come from the core; the page fetches only same-origin relative paths. | `boundary.rs::page_keeps_no_rule_and_no_foreign_url` (added in M8 with the page) |

## M2. Practice set (content in `data/practice-set.json`)

| Decision | Reason | Test |
|---|---|---|
| Set format: `format`, `id`, `title`, `sample`, `note`, `topics[{id,name}]`, `questions[{id,topic,prompt,choices[],answer}]`; the answer key is the text of the correct choice. Multiple choice only. | Multiple choice makes marking exact and unambiguous (no free-text normalisation rules to argue about) and keeps the answer key readable when a teacher edits the file. | `set::tests::sample_set_has_12_questions_in_4_topics` |
| The core validates a set: unique topic and question ids, every question's topic exists, every topic has a question, at least two distinct choices, the answer is one of the choices; unknown fields are rejected. | A broken set would silently mis-mark every student. | `set::tests::rejects_*` (answer not in choices, unknown topic, duplicate id, empty topic, malformed JSON) |
| The sample set is clearly marked: `"sample": true`, a title containing "SAMPLE", and a note; the page shows a banner from these fields. | The brief asks for clearly marked sample content. | `set::tests::sample_set_is_marked_as_sample`; browser check reads the banner. |

## M3. Rates and threshold (integers only)

| Decision | Reason | Test |
|---|---|---|
| A rate is kept as `correct / attempts` (two integers). The shown percentage is `floor(100 * correct / attempts)`. | No floats (rule e). Flooring means the shown figure can never round up across an integer threshold: 69.5% shows 69%, not 70%. | `rate::tests::percent_floor_hand_values` (6/9 = 66, 7/9 = 77, 8/9 = 88, 3/9 = 33, 7/10 = 70, 139/200 = 69, 0/0 = none) |
| "Below threshold" is `100 * correct < threshold * attempts` (exact). Exactly 70% is not below 70. No attempts gives no suggestion. | Exact comparison; the boundary is stated, not left to rounding. Consistent with the floored display for whole-number thresholds. | `rate::tests::below_threshold_boundary` |
| Threshold is typed by the teacher as text and parsed by the core: a whole number 0 to 100; anything else gives a message. Default 70 comes from the core. | Rule (c): the page holds no default and no range. | `rate::tests::parse_threshold_*`, `rate::tests::default_threshold_is_70` |

## M4. Student flow

| Decision | Reason | Test |
|---|---|---|
| The page holds an opaque state (`answers[]`, `checked`) that only the core creates and changes (`start`, `answer`, `check`). | Rule (c): the page draws the view the core returns and decides nothing. | `student::tests::*` |
| Checking needs every question answered; the message lists the unanswered question numbers. | Every saved entry is a real attempt, so "not correct" never means "skipped". | `student::tests::check_requires_every_answer` |
| Checking locks the answers; "Start again" begins a fresh attempt. | Otherwise a student can flip answers until every topic shows 100% and then save, and the teacher's rates would be meaningless. | `student::tests::answers_are_locked_after_check` |
| Per-topic score in the student view uses the same rate rule as the teacher view. | One rule, one place. | `student::tests::topic_scores_hand_computed` (pattern "otter": 2/3, 1/3, 3/3, 2/3) |
| Nickname: trimmed, 1 to 24 characters, letters, digits, space, `-`, `_`, `.` only. The page warns not to use a real name. | The brief: a nickname, no name, no account. Limiting characters keeps file names safe. | `student::tests::nickname_rules` |
| Result file: `format: "practice-result/1"`, `set_id`, `nickname`, `entries[{question, topic, correct}]`, `check`. Nothing else (no time stamp, no device data). File name `practice-result-<set id>-<nickname>.json`. | Exactly the data the brief allows. The question id is needed to find the most-missed questions; it describes the question, not the student. | `student::tests::saved_file_holds_only_allowed_fields`, `student::tests::saved_file_matches_fixture` |
| `check` is FNV-1a 64 over a canonical text of the content. | Detects hand edits and damaged files. It is not a security seal: anyone can recompute it. Stated in README and HANDOFF. | `result::tests::rejects_flipped_answer`; fixtures are made by an independent Python implementation and must be accepted. |

## M5. Reading result files (teacher side)

| Decision | Reason | Test |
|---|---|---|
| Each file is checked on its own and is either accepted or rejected with a message naming the file: too large (over 64 KiB), not JSON, unknown or missing fields, wrong format tag, other set, bad nickname, unknown/duplicate/missing question, topic not matching the set, check value not matching. | A malformed or altered file must give a message, never a crash, and must not count. | `result::tests::rejects_*` (one test per case), including 10 000 nested brackets (no stack overflow). |
| Fixtures `tests/fixtures/{otter,kestrel,maple}.json` and bad variants are written by `tests/fixtures/make_fixtures.py`, which implements the check value independently. | Cross-checks the core's checksum and gives hand-computed cases. | `result::tests::fixtures_are_accepted` |

## M6. Teacher report

| Decision | Reason | Test |
|---|---|---|
| Per topic (in set order): correct, attempts, floored percent, reteach flag. | The brief's main view. | `report::tests::three_files_hand_computed` (simplify 6/9 = 66%, share 3/9 = 33%, equivalent 8/9 = 88%, unit-rate 7/9 = 77%) |
| Reteach when below the threshold. | The brief, default 70. | `report::tests::reteach_at_70` (simplify, share), `reteach_at_80` (+ unit-rate), `reteach_at_0` (none) |
| Per topic, questions sorted by times missed (ties in set order); those with the highest non-zero count are marked "most missed". | "Questions most often missed", with ties shown rather than hidden. | `report::tests::most_missed_per_topic` (q03, q05, q09, q12) |
| A file with the same nickname and the same entries as an earlier one is counted once, with a notice naming both. Same nickname with different entries counts twice, with a notice. | A file handed in twice would otherwise inflate the counts; the teacher sees what happened. | `report::tests::duplicate_counted_once`, `report::tests::same_nickname_notice` |
| No accepted files: no percentage, no suggestion. Invalid threshold: message, no suggestions. | No division by zero; no suggestion from a bad setting. | `report::tests::no_files_no_data`, `report::tests::invalid_threshold_no_suggestions` |

## M7. JSON boundary and shim

| Decision | Reason | Test |
|---|---|---|
| `practice_core::api` has string-in, string-out functions returning `{"ok": ...}` or `{"error": "..."}`. The shim is one `#[wasm_bindgen]` line per function that calls `api`. | Keeps the shim free of decisions (rule b); errors reach the page as messages, not exceptions. | `api::tests::*` (round trip, garbage input gives `error`), `boundary.rs::shim_decides_nothing` |

## M8. Page and build

| Decision | Reason | Test |
|---|---|---|
| `web/index.html`, `web/app.js`, `web/style.css`; ES module imports `./pkg/practice_wasm.js`; the set is fetched from `data/practice-set.json` (same origin). Result files are read with the File API and saved with a `blob:` download. | No framework, bundler or npm; no other origin. | Browser check. |
| A Content-Security-Policy meta tag: `default-src 'self'`, `script-src 'self' 'wasm-unsafe-eval'`, `connect-src 'self'`, `object-src 'none'`, `base-uri 'none'`, `form-action 'none'`. | The browser itself refuses requests to another origin (rule d), even if a later edit adds one. | Browser check (no foreign request, no console error). |
| `build.sh`: cargo release build for wasm32, `wasm-bindgen --target web`, then copy `web/*` and `data/*.json` into `dist/web/`. | The brief's build. | Running `build.sh` produces `dist/web/index.html` and `dist/web/pkg/*.wasm`. |

## M9. Browser check

| Decision | Reason | Test |
|---|---|---|
| `tests/browser_check.py` (Playwright, Chromium) serves `dist/web` on 127.0.0.1 at a free port, completes the set with the "otter" pattern, checks the per-topic scores, saves the file, loads it with the kestrel and maple fixtures in the teacher view, checks the hand-computed rates and suggestions, loads an altered file and sees its message, changes the threshold, and fails on any console error, page error, or request to another origin. | The brief's browser check. | The script exits 0. |

## M10. Docs

README (build, test, serve), SECOND-ORDER.md (kept during the work; three
teacher-use assumptions checked against the brief at the end), TEST-LOG.md,
HANDOFF.md, TIMING.md finish time.
