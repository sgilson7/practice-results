#!/usr/bin/env bash
# Privacy proof (builder-setting kits/privacy): plants each leak from
# tests/plant_leak.py in a fresh copy of the repository and requires the
# boundary tests or the browser check (Chromium) to catch it. Takes minutes,
# so it is run by hand after a change to the page's data path or to the checks.
#     PYTHON=/path/to/python-with-playwright tests/prove-privacy.sh
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PY="${PYTHON:-python3}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

leaks=$(python3 "$ROOT/tests/plant_leak.py" --list) || { echo "could not list the leaks"; exit 1; }
# An empty list would pass by proving nothing.
[ -n "$leaks" ] || { echo "no leaks to plant: the proof would prove nothing"; exit 1; }

missed=0
for leak in $leaks; do
  rm -rf "$WORK/copy"
  rsync -a --exclude .git --exclude target --exclude dist "$ROOT/" "$WORK/copy/"
  python3 "$ROOT/tests/plant_leak.py" "$WORK/copy" "$leak" >/dev/null || { echo "  could not plant $leak"; missed=1; continue; }
  caught=""
  (cd "$WORK/copy" && cargo test -q --test boundary >/dev/null 2>&1) || caught="boundary tests"
  if (cd "$WORK/copy" && ./build.sh >/dev/null 2>&1); then
    out=$(cd "$WORK/copy" && BUILDER_ENGINES=chromium BUILDER_IDLE=1 "$PY" tests/browser_check.py 2>&1)
    if ! grep -q "BROWSER CHECK PASSED" <<<"$out"; then
      why=$(grep '^FAIL' <<<"$out" | sed 's/^FAIL //' | cut -c1-90 | paste -sd '|' -)
      caught="${caught:+$caught; }browser check ($why)"
    fi
  else
    echo "  build failed for $leak"; missed=1; continue
  fi
  if [ -n "$caught" ]; then echo "  caught $leak by $caught"; else echo "  MISSED $leak"; missed=1; fi
done
[ "$missed" = 0 ] && echo "PRIVACY PROOF PASSED" || { echo "PRIVACY PROOF FAILED"; exit 1; }
