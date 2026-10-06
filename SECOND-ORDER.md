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
