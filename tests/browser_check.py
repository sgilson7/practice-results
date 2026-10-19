#!/usr/bin/env python3
"""Browser check (Playwright) in Chromium, Firefox and WebKit.

For each engine it serves dist/web on 127.0.0.1 at a free port, waits for the
page to set <body data-ready="true"> (after the wasm module and the practice
set have loaded), idles, then completes the practice set with the "otter"
answers, checks the per-topic scores, saves the result file, loads it with the
kestrel and maple fixtures in the teacher view, checks the hand-computed
rates, suggestions and most-missed questions, changes the threshold, and loads
an altered file and a file that is not a result and sees their messages.

The run fails on (privacy kit, builder-setting kits/privacy):
- a console error or an uncaught page error;
- any request to another origin, at any point;
- any request at all after the page reported ready, same-origin included
  (blob: and data: stay in the browser and are allowed);
- a Content-Security-Policy violation (a blocked attempt is still an attempt);
- a WebSocket, popup, worker or service worker;
- anything left in cookies, localStorage, sessionStorage or IndexedDB.

It also opens the page with WebAssembly removed, and with WebAssembly
compiling refused (what a school policy that blocks the JavaScript JIT does),
and expects the "this browser blocks WebAssembly" message; on a normal run
that message must stay hidden.

A pass shows what the page did on this path, in these engines, on this run.
It does not show that a feature the check never reaches keeps data in.

Run after ./build.sh:
    python tests/browser_check.py

Environment:
    BUILDER_ENGINES  space-separated engines (default "chromium firefox webkit")
    BUILDER_IDLE     seconds to idle after ready (default 10)
"""
import functools
import http.server
import json
import os
import pathlib
import sys
import tempfile
import threading

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parent.parent
DIST = ROOT / "dist" / "web"
FIX = ROOT / "tests" / "fixtures"
SET = json.loads((ROOT / "data" / "practice-set.json").read_text(encoding="utf-8"))
OTTER = "110100111110"  # simplify 110, share 100, equivalent 111, unit-rate 110
TOPICS = ["simplify", "share", "equivalent", "unit-rate"]
ENGINES = os.environ.get("BUILDER_ENGINES", "chromium firefox webkit").split()
IDLE_MS = int(float(os.environ.get("BUILDER_IDLE", "10")) * 1000)

# Recorded from the first script on the page, before the page's own code runs:
# a request the CSP blocks never becomes a request event, so the violation
# itself is the evidence.
CSP_RECORDER = (
    "window.__csp = [];"
    "document.addEventListener('securitypolicyviolation',"
    " e => window.__csp.push(e.violatedDirective + ' ' + e.blockedURI));"
)
# Two ways a browser can refuse WebAssembly. Chrome's DefaultJavaScriptJitSetting
# = block removes or disables it; "absent" deletes the object, "refused" keeps
# it but makes every compile and instantiate reject.
BLOCKED_MODES = {
    "absent": "delete window.WebAssembly;",
    "refused": (
        "(() => { const no = () => Promise.reject(new WebAssembly.CompileError('blocked by policy'));"
        " WebAssembly.compile = no; WebAssembly.instantiate = no; WebAssembly.instantiateStreaming = no;"
        " WebAssembly.validate = () => false; })();"
    ),
}
BLOCKED_TEXT = "This tool could not start because this browser blocks WebAssembly"

failures = []


def check(ok, what):
    print(("PASS " if ok else "FAIL ") + what)
    if not ok:
        failures.append(what)


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".json": "application/json",
    }

    def log_message(self, *args):
        pass


def serve():
    handler = functools.partial(Handler, directory=str(DIST))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def cells(page, selector):
    return page.locator(selector).evaluate_all(
        "rows => rows.map(r => Array.from(r.cells).map(c => c.textContent))"
    )


def topic_rows(page):
    return {row[0]: row[1:] for row in cells(page, "#topic-table tbody tr")}


def wait_summary(page, prefix):
    page.wait_for_function(
        "p => document.querySelector('#report-summary').textContent.startsWith(p)", arg=prefix
    )


def run_engine(p, engine, origin):
    print(f"\n[{engine}] {origin}/")
    seen = {"console": [], "errors": [], "foreign": [], "after_ready": [], "sockets": [], "popups": [], "workers": []}
    ready = {"yes": False}

    def same_origin(url):
        return url.startswith(origin + "/") or url.startswith("blob:" + origin + "/") or url.startswith("data:")

    def on_request(req):
        if not same_origin(req.url):
            seen["foreign"].append(req.url)
        if ready["yes"] and not req.url.startswith(("blob:", "data:")):
            seen["after_ready"].append(req.url)

    browser = getattr(p, engine).launch()
    context = browser.new_context(accept_downloads=True)
    context.on("request", on_request)

    def route(r):
        if same_origin(r.request.url):
            r.continue_()
        else:
            r.abort()

    context.route("**/*", route)
    context.add_init_script(CSP_RECORDER)
    page = context.new_page()
    page.on("console", lambda m: seen["console"].append(m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: seen["errors"].append(str(e)))
    page.on("websocket", lambda ws: seen["sockets"].append(ws.url))
    page.on("worker", lambda w: seen["workers"].append(w.url))
    page.on("popup", lambda pg: seen["popups"].append(pg.url))

    page.goto(origin + "/index.html")
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    ready["yes"] = True
    check(True, "page reported ready")
    check(page.locator("#wasm-blocked").is_hidden(), "the blocked-WebAssembly message is hidden when WebAssembly works")
    page.wait_for_timeout(IDLE_MS)
    check(not seen["after_ready"], f"no request during {IDLE_MS // 1000}s idle after ready {seen['after_ready']}")
    check("SAMPLE CONTENT" in page.inner_text("#sample-banner"), "sample content is marked on the page")
    check(
        "your school's IT can allow file selection" in page.inner_text("#teacher-view"),
        "the teacher view says a school device may block opening files",
    )

    # ---- student view ----
    page.click("#check-button")
    check(
        "Answer every question before checking" in page.inner_text("#student-message"),
        "checking with nothing answered shows the core's message",
    )
    for i, q in enumerate(SET["questions"]):
        key = q["choices"].index(q["answer"])
        pick = key if OTTER[i] == "1" else (1 if key == 0 else 0)
        page.locator(f"#question-{q['id']} input").nth(pick).check()
    check(page.inner_text("#progress") == "Answered 12 of 12.", "progress shows 12 of 12")
    page.click("#check-button")
    scores = cells(page, "#score-table tbody tr")
    # by hand: simplify 2/3 66%, share 1/3 33%, equivalent 3/3 100%, unit-rate 2/3 66%
    check(
        scores
        == [
            ["Simplifying ratios", "2 of 3", "66%"],
            ["Sharing in a ratio", "1 of 3", "33%"],
            ["Equivalent ratios and missing values", "3 of 3", "100%"],
            ["Unit rates and best buys", "2 of 3", "66%"],
        ],
        f"student per-topic scores match the hand-computed values {scores}",
    )
    check(page.inner_text("#overall") == "All topics: 8 of 12 (66%).", "overall score 8 of 12")
    check(page.locator("#question-q01 input").first.is_disabled(), "answers are locked after checking")
    check(
        "your school's IT can allow downloads" in page.inner_text("#scores"),
        "the save controls say a school device may block saving files",
    )

    page.fill("#nickname", "a/b")
    page.click("#save-button")
    check("A nickname can use" in page.inner_text("#student-message"), "a bad nickname gets the core's message")
    page.fill("#nickname", "otter")
    with page.expect_download() as info:
        page.click("#save-button")
    download = info.value
    saved = pathlib.Path(tempfile.mkdtemp()) / download.suggested_filename
    download.save_as(saved)
    check(download.suggested_filename == "practice-result-ratios-sample-1-otter.json", "suggested file name")
    doc = json.loads(saved.read_text(encoding="utf-8"))
    check(sorted(doc) == ["check", "entries", "format", "nickname", "set_id"], "result file holds only the allowed fields")
    check([e["correct"] for e in doc["entries"]] == [c == "1" for c in OTTER], "result file records the otter answers")
    check(
        saved.read_text(encoding="utf-8") == (FIX / "otter.json").read_text(encoding="utf-8"),
        "saved file equals the independently generated otter fixture",
    )

    # ---- teacher view: three files ----
    page.click("#tab-teacher")
    check(page.input_value("#threshold") == "70", "threshold box starts at 70 (from the core)")
    page.set_input_files("#files", [str(saved), str(FIX / "kestrel.json"), str(FIX / "maple.json")])
    wait_summary(page, "Files counted: 3. Files not counted: 0.")
    rows = topic_rows(page)
    # by hand: simplify 6/9 66%, share 3/9 33%, equivalent 8/9 88%, unit-rate 7/9 77%
    check(
        rows
        == {
            "Simplifying ratios": ["66%", "6 of 9", "Reteach (below 70%)"],
            "Sharing in a ratio": ["33%", "3 of 9", "Reteach (below 70%)"],
            "Equivalent ratios and missing values": ["88%", "8 of 9", "At or above 70%"],
            "Unit rates and best buys": ["77%", "7 of 9", "At or above 70%"],
        },
        f"teacher rates and suggestions at 70% match the hand-computed values {rows}",
    )
    most = {
        t: page.locator(f"#missed-{t} li:has(.badge)").evaluate_all("items => items.map(i => i.value)")
        for t in TOPICS
    }
    check(most == {"simplify": [3], "share": [5], "equivalent": [9], "unit-rate": [12]}, f"most-missed questions {most}")

    page.fill("#threshold", "80")
    page.wait_for_function("() => document.querySelector('#report-summary').textContent.includes('Unit rates')")
    check(
        topic_rows(page)["Unit rates and best buys"][2] == "Reteach (below 80%)",
        "at 80% unit rates (77%) is also suggested for reteaching",
    )
    page.fill("#threshold", "seventy")
    page.wait_for_function("() => document.querySelector('#threshold-message').textContent !== ''")
    check(
        "whole number from 0 to 100" in page.inner_text("#threshold-message"),
        "an invalid threshold gets the core's message",
    )
    check(
        all(r[2] == "—" for r in topic_rows(page).values()),
        "no suggestions while the threshold is invalid",
    )
    page.fill("#threshold", "70")
    wait_summary(page, "Files counted: 3.")

    # ---- teacher view: altered and malformed files are rejected, not counted ----
    page.set_input_files(
        "#files",
        [
            str(saved),
            str(FIX / "kestrel.json"),
            str(FIX / "maple.json"),
            str(FIX / "altered-otter.json"),
            str(FIX / "not-a-result.json"),
        ],
    )
    wait_summary(page, "Files counted: 3. Files not counted: 2.")
    rejected = page.locator("#rejected li").all_inner_texts()
    check(
        len(rejected) == 2
        and rejected[0].startswith("altered-otter.json: Its check value does not match")
        and rejected[1].startswith("not-a-result.json: It is not a result file"),
        f"altered and malformed files are listed with messages {rejected}",
    )
    check(topic_rows(page) == rows, "rejected files do not change the rates")
    page.wait_for_timeout(500)

    csp = page.evaluate("() => window.__csp || []")
    stored = page.evaluate("""async () => {
        const idb = indexedDB.databases ? (await indexedDB.databases()).length : 0;
        return {cookie: document.cookie, local: localStorage.length, session: sessionStorage.length, idb};
    }""")
    cookies = context.cookies()
    sw = page.evaluate("() => navigator.serviceWorker ? navigator.serviceWorker.getRegistrations().then(r => r.length) : 0")
    browser.close()

    check(not csp, f"no Content-Security-Policy violation {csp}")
    check(not seen["after_ready"], f"no request of any kind after the page reported ready {seen['after_ready']}")
    check(
        stored == {"cookie": "", "local": 0, "session": 0, "idb": 0} and not cookies,
        f"nothing kept in cookies or browser storage {stored} {cookies}",
    )
    check(not seen["console"], f"no console errors {seen['console']}")
    check(not seen["errors"], f"no page errors {seen['errors']}")
    check(not seen["foreign"], f"no request to another origin {seen['foreign']}")
    check(not seen["sockets"], f"no WebSocket {seen['sockets']}")
    check(not seen["popups"], f"no popup {seen['popups']}")
    check(not seen["workers"] and sw == 0, f"no worker or service worker {seen['workers']} sw={sw}")


def run_blocked(p, engine, origin, mode):
    """Open the page in a browser that refuses WebAssembly; expect the message."""
    browser = getattr(p, engine).launch()
    context = browser.new_context()
    context.add_init_script(BLOCKED_MODES[mode])
    page = context.new_page()
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.goto(origin + "/index.html")
    try:
        page.wait_for_selector("#wasm-blocked", state="visible", timeout=10000)
        shown = page.inner_text("#wasm-blocked")
    except Exception as exc:  # the message never appeared
        shown = f"(not shown: {exc.__class__.__name__})"
    check(
        BLOCKED_TEXT in shown and "sgilson7.github.io" in shown,
        f"WebAssembly {mode}: the page says the browser blocks WebAssembly and IT can allow it {shown[:80]!r}",
    )
    check(
        page.locator("main").is_hidden() and page.locator("#load-status").is_hidden(),
        f"WebAssembly {mode}: the tool's controls and the loading line are hidden",
    )
    check(not errors, f"WebAssembly {mode}: no uncaught page error {errors}")
    browser.close()


def main():
    if not (DIST / "index.html").exists():
        sys.exit("dist/web is missing: run ./build.sh first")
    server = serve()
    host, port = server.server_address[:2]
    origin = f"http://{host}:{port}"
    with sync_playwright() as p:
        for engine in ENGINES:
            run_engine(p, engine, origin)
            for mode in BLOCKED_MODES:
                run_blocked(p, engine, origin, mode)
    server.shutdown()

    if failures:
        print(f"\nBROWSER CHECK FAILED: {len(failures)} problem(s)")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nBROWSER CHECK PASSED in {', '.join(ENGINES)}")


if __name__ == "__main__":
    main()
