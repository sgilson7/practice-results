# Practice results by topic

A static browser tool with two views.

- **Student view**: practise a set of multiple-choice questions on your own
  device, check your answers to see a score per topic, and save a result file
  under a nickname you choose (no name, no account).
- **Teacher view**: open all the result files the students handed in, at once,
  and see per topic the share of correct answers, the questions most often
  missed, and a suggestion to reteach any topic below a threshold you set
  (default 70%).

Everything is computed in the browser on the device that opens the page.
Nothing is sent anywhere. The page sets a Content-Security-Policy that blocks
requests to any other origin.

The practice set in `data/practice-set.json` is **sample content** (12 ratio
questions in 4 topics, with answer keys) written to test the tool. Check or
replace it before classroom use.

## Layout

| Path | What it is |
|---|---|
| `crates/core` | Every rule: set validation, marking, per-topic scores, result files and their checks, the teacher report. No input or output; depends only on serde and serde_json; no floats, hash maps or clocks. |
| `crates/wasm` | wasm-bindgen shim (`wasm-bindgen = "=0.2.127"`). One forwarding line per function; decides nothing. |
| `web/` | `index.html`, `app.js`, `style.css`. No framework, no bundler, no npm. Draws what the core returns. |
| `data/practice-set.json` | The practice set (sample content). |
| `build.sh` | Builds the wasm module and assembles `dist/web/`. |
| `tests/browser_check.py` | Playwright check in Chromium. |
| `tests/fixtures/` | Hand-computed result files and their generator, `make_fixtures.py`. |

## Requirements

- Rust (stable) with the `wasm32-unknown-unknown` target:
  `rustup target add wasm32-unknown-unknown`
- `wasm-bindgen-cli` **0.2.127**, which must match the crate version:
  `cargo install wasm-bindgen-cli --version 0.2.127`
- Python 3, to serve the folder locally and to run the browser check.
- Only for the browser check: Playwright for Python with Chromium:
  `pip install playwright && python -m playwright install chromium`

## Build

```sh
./build.sh
```

This runs `cargo build --release --target wasm32-unknown-unknown -p practice_wasm`,
then `wasm-bindgen --target web` into `dist/web/pkg/`, and copies `web/*` and
`data/*.json` into `dist/web/`.

## Test

```sh
cargo test                        # 71 tests: core rules, hand-computed cases, boundary tests
./build.sh && python tests/browser_check.py   # browser check (Chromium)
```

- The core tests include hand-computed cases: per-topic rates for three result
  files (`tests/fixtures/otter.json`, `kestrel.json`, `maple.json`: 66%, 33%,
  88%, 77%) and which topics fall below 70% (simplifying, sharing) or 80% (plus
  unit rates).
- Malformed or altered result files (not JSON, cut off, wrong fields or types,
  another set, unknown, missing or repeated questions, a flipped answer,
  10 000-deep nesting, oversized) are rejected with a message, never a crash.
- `crates/core/tests/boundary.rs` reads the core's `Cargo.toml` and source and
  fails on a dependency other than serde and serde_json, or on a float, hash
  map, clock, randomness or input/output in the core. It also fails if the shim
  contains a decision (`if`, `match`, loops, `?`) or if `web/*.js` contains
  rule-like code (numbers other than 0 and 1, `Math`, number parsing, storage)
  or any absolute URL.
- The browser check serves `dist/web` on 127.0.0.1 at a free port, completes
  the set, saves the result file, loads it with two fixtures in the teacher
  view, checks the rates, suggestions and most-missed questions, loads an
  altered file and a non-result file, and fails on any console error, page
  error, or request to another origin.

`TEST-LOG.md` records how each test was first seen failing.

## Serve

Any static file host works (school web space, an intranet server, GitHub
Pages). The page must be served over HTTP(S); opening `index.html` straight
from disk does not work, because browsers do not load modules and WebAssembly
from `file://`. To try it locally:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory dist/web
```

then open <http://127.0.0.1:8000/>. Serve `.wasm` as `application/wasm` if your
host lets you choose; otherwise the module still loads, a little more slowly.

## Use

**Students**: open the page, answer every question, choose **Check my
answers** (answers then lock), read the score per topic, type a nickname that
is not their real name, and choose **Save result file**. They hand the file in
through the class's usual channel. **Start again** begins a new attempt.

**Teacher**: open the page, choose **Teacher**, select all the result files at
once, and set the threshold if 70% is not what you want. Files that cannot be
counted are listed with the reason. Notices point out files handed in twice
and nicknames used in more than one file.

## Result file

```json
{
  "format": "practice-result/1",
  "set_id": "ratios-sample-1",
  "nickname": "otter",
  "entries": [ { "question": "q01", "topic": "simplify", "correct": true }, ... ],
  "check": "fnv1a64:ef4d07ce68cf4deb"
}
```

`check` is FNV-1a 64 over the content. It catches files edited by hand or
damaged in transit. It is **not** a security seal: the algorithm is public, so
a determined student can make a file that passes. Treat the files as practice
evidence, not marks.

## Changing the practice set

Edit `data/practice-set.json` (each question: `id`, `topic`, `prompt`,
`choices`, and `answer`, which must be exactly one of the choices), then run
`./build.sh`. **Change the set `id` whenever you change the questions**, so
files saved for the old version are rejected with a message instead of being
counted against different questions. For real content, set `"sample": false`
and remove "SAMPLE" from the title. The core checks the set when the page
loads and shows a message if something is wrong. After changing the set, also
rerun `python3 tests/fixtures/make_fixtures.py` and update the hand-computed
numbers in the tests, which are written for the sample set.

## Notes

`PLAN.md` holds the decisions and their tests, `SECOND-ORDER.md` the
assumptions and open items, `TEST-LOG.md` the failing runs and corrections,
and `HANDOFF.md` the state at handover.
