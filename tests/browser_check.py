#!/usr/bin/env python3
"""Browser check (Playwright, Chromium).

Serves dist/web on 127.0.0.1 at a free port, completes the practice set with
the "otter" answers, checks the per-topic scores, saves the result file, loads
it with the kestrel and maple fixtures in the teacher view, checks the
hand-computed rates, suggestions and most-missed questions, changes the
threshold, loads an altered file and a file that is not a result and sees
their messages, and fails on any console error, page error, or request to
another origin.

Run after ./build.sh:
    python tests/browser_check.py
"""
import functools
import http.server
import json
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


def main():
    if not (DIST / "index.html").exists():
        sys.exit("dist/web is missing: run ./build.sh first")
    server = serve()
    host, port = server.server_address[:2]
    origin = f"http://{host}:{port}"
    console_errors, page_errors, foreign = [], [], []

    def same_origin(url):
        return url.startswith(origin + "/") or url.startswith("blob:" + origin + "/") or url.startswith("data:")

    with sync_playwright() as p:
        browser = p.chromium.launch()
        context = browser.new_context(accept_downloads=True)
        context.on("request", lambda req: None if same_origin(req.url) else foreign.append(req.url))

        def route(r):
            if same_origin(r.request.url):
                r.continue_()
            else:
                foreign.append(r.request.url)
                r.abort()

        context.route("**/*", route)
        page = context.new_page()
        page.on("console", lambda m: console_errors.append(m.text) if m.type == "error" else None)
        page.on("pageerror", lambda e: page_errors.append(str(e)))

        page.goto(origin + "/index.html")
        page.wait_for_selector("body[data-ready='true']", timeout=15000)
        check("SAMPLE CONTENT" in page.inner_text("#sample-banner"), "sample content is marked on the page")

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
        browser.close()
    server.shutdown()

    check(not console_errors, f"no console errors {console_errors}")
    check(not page_errors, f"no page errors {page_errors}")
    check(not foreign, f"no request to another origin {foreign}")
    if failures:
        print(f"\nBROWSER CHECK FAILED: {len(failures)} problem(s)")
        sys.exit(1)
    print("\nBROWSER CHECK PASSED")


if __name__ == "__main__":
    main()
