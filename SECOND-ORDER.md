# SECOND-ORDER notebook

One row per assumption, surprise, or open item.
Kind: **divergence** (from the plan), **finding**, or **worklist** (item for the teacher).
Status: **done**, **open**, or **teacher** (assigned to the teacher).

| # | Kind | Note | Status |
|---|---|---|---|
| 1 | finding | Result entries carry the question id as well as topic and correct. The brief lists "topic and whether the answer was correct"; without the id the teacher view cannot name the questions most often missed. The id describes the question, not the student, so no student data is added. | done |
| 2 | finding | Multiple choice only (no free-text answers). Free text would need normalisation rules (spaces, "3:4" vs "3 : 4", equivalent ratios) that the brief does not settle. | done |
| 3 | worklist | The answer key sits in `data/practice-set.json`, which any student can open. This is a practice tool, not a test. | teacher |
