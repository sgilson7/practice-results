#!/usr/bin/env python3
"""Writes the hand-computed result-file fixtures used by the Rust tests and the
browser check.

The check value is computed here, independently of the Rust core (FNV-1a 64
over the canonical text described in crates/core/src/result.rs), so the core
accepting these files cross-checks its implementation.

Patterns (1 = correct), per topic in set order:
            simplify  share  equivalent  unit-rate
  otter       110      100      111        110     -> 2, 1, 3, 2
  kestrel     111      001      110        110     -> 3, 1, 2, 2
  maple       100      100      111        111     -> 1, 1, 3, 3
  totals (of 9)                                    -> 6, 3, 8, 7
"""
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
SET = json.loads((HERE.parent.parent / "data" / "practice-set.json").read_text(encoding="utf-8"))

PATTERNS = {
    "otter": "110 100 111 110",
    "kestrel": "111 001 110 110",
    "maple": "100 100 111 111",
}


def fnv1a64(data: bytes) -> int:
    h = 0xCBF29CE484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def canonical(fmt: str, set_id: str, nickname: str, entries: list) -> str:
    lines = [fmt, "set:" + set_id, "nickname:" + nickname]
    lines += ["%s|%s|%d" % (e["question"], e["topic"], 1 if e["correct"] else 0) for e in entries]
    return "\n".join(lines) + "\n"


def result(nickname: str, pattern: str) -> dict:
    flags = [c == "1" for c in pattern.replace(" ", "")]
    assert len(flags) == len(SET["questions"])
    entries = [
        {"question": q["id"], "topic": q["topic"], "correct": c}
        for q, c in zip(SET["questions"], flags)
    ]
    doc = {"format": "practice-result/1", "set_id": SET["id"], "nickname": nickname, "entries": entries}
    text = canonical(doc["format"], doc["set_id"], nickname, entries)
    doc["check"] = "fnv1a64:%016x" % fnv1a64(text.encode("utf-8"))
    return doc


def write(name: str, doc: dict) -> None:
    (HERE / name).write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


if __name__ == "__main__":
    for nick, pattern in PATTERNS.items():
        write(nick + ".json", result(nick, pattern))
    # altered: otter's q03 flipped from wrong to right after saving (check value kept)
    altered = result("otter", PATTERNS["otter"])
    altered["entries"][2]["correct"] = True
    write("altered-otter.json", altered)
    (HERE / "not-a-result.json").write_text("this is not a result file\n", encoding="utf-8")
    print("fixtures written to", HERE)
