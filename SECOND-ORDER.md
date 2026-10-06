# SECOND-ORDER notebook

One row per assumption, surprise, or open item.
Kind: **divergence** (from the plan), **finding**, or **worklist** (item for the teacher).
Status: **done**, **open**, or **teacher** (assigned to the teacher).

| # | Kind | Note | Status |
|---|---|---|---|
| 1 | finding | Result entries carry the question id as well as topic and correct. The brief lists "topic and whether the answer was correct"; without the id the teacher view cannot name the questions most often missed. The id describes the question, not the student, so no student data is added. | done |
| 2 | finding | Multiple choice only (no free-text answers). Free text would need normalisation rules (spaces, "3:4" vs "3 : 4", equivalent ratios) that the brief does not settle. | done |
| 3 | worklist | The answer key sits in `data/practice-set.json`, which any student can open. This is a practice tool, not a test. | teacher |
| 4 | finding | serde lets a struct be read from a JSON array (`["practice-result/1", ...]`), so a result file in array form would have been accepted. The reader now requires a JSON object (`rejects_array_instead_of_object`). | done |
| 5 | finding | The core's saved file for the "otter" answers is byte-for-byte the file written by the independent Python generator, so the check value and the layout agree across two implementations. | done |
| 6 | finding | The check value is FNV-1a over public content: it catches hand edits and damage, not a determined forger, who can recompute it. A static page cannot keep a secret key. | done |
| 7 | worklist | Treat result files as practice evidence, not marks: a student who wants to can make a file that passes the check. | teacher |
| 8 | finding | A test can pass against a stub for the wrong reason: `no_files_no_data` held for an empty topic list. Breaking every test before keeping it caught this; the test now also asserts all four topics are listed. | done |
| 9 | divergence | Plan said the student "sees a score per topic"; I also considered per-question marks and chose not to show them: with marks, "Start again" lets a student reach 100% by elimination and hand in that file. Per-topic scores only, as the brief says. | done |
| 10 | worklist | Students can start again and save more than one attempt. The teacher view counts each distinct file and flags repeated nicknames; ask students to hand in their first attempt if that is what you want to measure. | teacher |
| 11 | worklist | Two students may pick the same nickname. The report flags it; only the teacher can tell repeated attempts from two students. | teacher |
| 12 | finding | The page sets a Content-Security-Policy (`default-src 'self'`, `script-src 'self' 'wasm-unsafe-eval'`, `connect-src 'self'`), so the browser itself blocks requests to other origins. `'wasm-unsafe-eval'` is needed for WebAssembly; very old browsers may not know it. | done |
| 13 | worklist | Check the tool starts on the browsers your students actually use (school laptops, tablets). It was checked in Chromium only. | teacher |
| 14 | finding | The page reads each chosen file in full before the core checks its size (64 KiB limit). Choosing a very large non-result file by mistake could be slow; it is still rejected with a message. | open |
| 15 | finding | The page makes same-origin requests only: index.html, style.css, app.js, the wasm module and `data/practice-set.json`, all from the host that served the page. None carries student data. Result files are read from disk with the File API and saved through a `blob:` download. No request was added beyond loading the page's own files. | done |
| 16 | worklist | Confirm that each result entry may carry the question id (row 1). If not, the "most missed" list cannot be built. | teacher |
| 17 | worklist | When you change the questions, change the set `id` as well. Otherwise files saved for the old questions still pass every check and are counted against the new ones. | teacher |
| 18 | finding | The shim has no tests of its own because it holds no logic; `shim_decides_nothing` guards that. | done |
| 19 | open | The page-rule scan is a heuristic (numbers other than 0 and 1, arithmetic, number parsing and storage names, absolute URLs). It cannot prove the page holds no rule; `web/app.js` (about 230 lines) should be read once by a person. | open |
| 20 | divergence | Plan M9 listed the browser check as one script; it grew to 22 named checks (adds nickname message, answer lock, invalid threshold, most-missed list) because each was cheap once the page was driven. | done |

## Three assumptions about how a teacher will use the tool, checked against the brief

| # | Assumption | What the brief says | Verdict |
|---|---|---|---|
| A1 | The teacher downloads all handed-in files into one folder and selects them together in one file picker; file names may be changed by the channel (for example `otter (1).json`). | "Students hand the files to the teacher through the class's usual channel. In the teacher view, the teacher opens all the result files at once." | **Holds.** The multi-file picker matches "at once". The tool never relies on file names (they only label messages), and an identical file handed in twice is counted once with a notice. |
| A2 | "Share of correct answers" means correct answers divided by all answers in that topic, pooled across the class, not the average of each student's percentage. | "sees, per topic, the share of correct answers". | **Holds.** The pooled reading is the literal one. Every accepted file must answer every question, so the pooled share equals the mean of the students' shares here; the two readings would differ only if partial files were allowed, and they are not. |
| A3 | "Below the threshold" is strict, so a topic at exactly 70% gets no reteach suggestion, and the teacher compares the shown percentage with the threshold. | "a suggestion to reteach any topic below a threshold the teacher sets (default 70%)". | **Holds, with one thing the teacher should know.** Strict "below" is the brief's wording. Percentages are floored for display (69.9% shows as 69%), so the shown number and the suggestion always agree; this is tested for every count up to 40 and every threshold 0 to 100. The flooring is a decision the brief does not make; it is listed in HANDOFF for the teacher. |
