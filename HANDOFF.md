# HANDOFF

## What was built

A static page with a student view and a teacher view, built from a Rust core.

- **Student view.** The student answers 12 multiple-choice ratio questions
  (sample content, 4 topics), checks them, and sees a score per topic. Checking
  needs every question answered and then locks the answers, so the saved file
  is the attempt that was checked. The student saves a result file under a
  nickname; the file holds only the nickname, the set id, one entry per
  question (question id, topic, correct), a format tag and a check value.
- **Teacher view.** The teacher selects all result files at once. Per topic the
  page shows the share of correct answers (pooled, floored to a whole percent),
  "6 of 9" counts, a reteach suggestion when the share is below the threshold
  (default 70, typed by the teacher), and the topic's questions ordered by how
  often they were missed, with the most-missed marked. Files that cannot be
  counted are listed with the reason; duplicate files and repeated nicknames
  get a notice.
- **Where the rules live.** Every rule is in `crates/core` (no input or output;
  serde and serde_json only; no floats, hash maps or clocks). `crates/wasm`
  forwards strings and decides nothing. `web/app.js` draws the JSON the core
  returns.

State at handover:

- `cargo test`: 73 tests, all passing (62 in the core library, 11 boundary tests).
- `./build.sh` produces `dist/web/` (index.html, app.js, style.css,
  pkg/practice_wasm.js, pkg/practice_wasm_bg.wasm, data/practice-set.json).
- `tests/browser_check.py` passes in Chromium, Firefox and WebKit
  (39 checks per engine), with no console errors, no page
  errors, no request to another origin, no request after ready, no CSP
  violation and nothing in browser storage; with WebAssembly removed or
  refused, the "blocks WebAssembly" message is shown.
- `tests/prove-privacy.sh`: PRIVACY PROOF PASSED, 10 of 10 planted leaks caught.
- Every test was seen failing once before it was kept (`TEST-LOG.md`).

## What is open

- **Engines, not devices.** Chromium, Firefox and WebKit (Playwright builds)
  were checked; real iPads, Safari and managed Chromebooks were not. The page
  needs WebAssembly and a browser that understands `'wasm-unsafe-eval'` in its
  Content-Security-Policy. Where a school policy blocks WebAssembly the page
  says so (emulated in the check, not seen under a real policy).
- **The check value is not a seal.** It catches hand edits and damaged files,
  not a student who recomputes it. A static page cannot keep a secret.
- **The answer key is public.** Anyone who opens `data/practice-set.json` can
  read it. This is fine for practice, not for a test.
- **Multiple choice only.** Free-text answers would need normalisation rules
  (for example "3:4" and "3 : 4") that the brief does not settle.
- **Repeated attempts.** Students can start again and save more than one file.
  Identical files count once; different files under one nickname all count,
  with a notice. Which attempt should count is the teacher's decision.
- **Large files.** The page reads a chosen file in full before the core
  rejects anything over 64 KiB, so picking a huge file by mistake is slow (it
  is still rejected with a message).
- **The page-rule scan is a heuristic.** It catches numbers, arithmetic,
  number parsing, storage and absolute URLs in `web/*.js`, but it cannot prove
  the page holds no rule.

## What the teacher should check by hand

1. **The sample questions and answer keys** in `data/practice-set.json`. They
   were written by an AI agent to test the tool and have not been reviewed by
   a teacher. Replace them (and the set `id`) before classroom use.
2. **The hand-computed case.** Open the page, choose Teacher, and select
   `tests/fixtures/otter.json`, `kestrel.json` and `maple.json`. You should see
   Simplifying 66% (6 of 9, reteach), Sharing 33% (3 of 9, reteach),
   Equivalent 88% (8 of 9), Unit rates 77% (7 of 9); most missed q03, q05, q09,
   q12. The working is in `tests/fixtures/make_fixtures.py`. Add
   `altered-otter.json` and it should be listed as not counted.
3. **The threshold rule.** "Below" is strict: exactly 70% is not suggested for
   reteaching. Percentages are rounded down, so 69.9% shows as 69%. Decide
   whether that matches how you read the numbers.
4. **The result file contents.** Each entry carries the question id as well as
   topic and correct, so the teacher view can name the most-missed questions.
   Confirm that this is acceptable to hand in.
5. **Your students' devices.** Open the page on the browsers your students use
   and complete one attempt, including saving the file.
6. **Nicknames.** Remind students not to use their real names; the tool limits
   characters and length but cannot tell a nickname from a name.
7. **`web/app.js`.** Read it once to confirm it only draws what the core sends.

## Rules from the brief

All met, with two notes. (d): the page loads its own files from the host that
served it (the wasm module and `data/practice-set.json` included); nothing goes
to another origin and the CSP blocks that. (c): checked by a heuristic scan
plus reading the code, not proved.
