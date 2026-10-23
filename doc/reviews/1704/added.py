#!/usr/bin/env python3
"""Review 1704 section 1.3: the lines a span added that match a pattern, at their line in the span's end.

Usage: python3 -I -B doc/reviews/1704/added.py <first> <last> <regex> <path>...
Prints `file:line: text` for every added line of `git diff -U0 <first>..<last> -- <path>...` whose
text matches. A test path or a `#[cfg(test)]` module is not filtered here; the review's commands
filter by path with grep and read each remaining hit in its file.
"""
import re, subprocess, sys

first, last, pattern = sys.argv[1], sys.argv[2], re.compile(sys.argv[3])
out = subprocess.run(["git", "diff", "-U0", f"{first}..{last}", "--"] + sys.argv[4:],
                     capture_output=True, text=True, check=True).stdout
path, line = None, 0
for text in out.split("\n"):
    if text.startswith("+++ "):
        path = text[6:] if text.startswith("+++ b/") else None
        continue
    hunk = re.match(r"@@ -\S+ \+(\d+)(?:,\d+)? @@", text)
    if hunk:
        line = int(hunk.group(1))
        continue
    if path and text.startswith("+"):
        if pattern.search(text[1:]):
            print(f"{path}:{line}: {text[1:].strip()[:150]}")
        line += 1
