#!/usr/bin/env python3
"""How much of `CLAUDE.md`'s comment rule is still owed, counted by shape.

`CLAUDE.md` ("Where knowledge lives") names the sweep that measures it:

    grep -rE "hundred-and-|session" --include=*.rs crates tools

and that grep is printed here first, unchanged, so the number a reader of `CLAUDE.md` would get is
the number this prints. It is a reading list rather than a count of defects, because `session` is
also a word this program uses for itself — a viewer's `Session`, a FUSE session, a desktop's session
bus — and in code every hit is an identifier. So each hit is sorted by shape, first match wins:

  code          the hit is outside a `//` comment: an identifier or a string, never history.
  legitimate    a comment naming a session that is this program's or the desktop's (the shapes
                are `LEGITIMATE` below, each one a use the rule does not reach).
  history       a comment carrying a session by ordinal or number ("the four-hundred-and-
                seventy-first session", "session 945", "for four hundred sessions") or by
                pointing at one ("this session", "the session that landed it").
  unread        a comment the two lists above do not decide — a reading list, printed by name
                with `--list unread`, because whether "a session that reads it" is advice or
                history is a question about English.

A comment carries the current reason and cites the ADR that argued it; the session that changed it
is `doc/history/`'s (ADR 1023). This script counts; it rewrites nothing. `raster/` is counted apart,
because `CLAUDE.md`'s grep does not reach it and its hits are the same rule's (ADR 1403).

    tools/comment-history.py                  # the counts
    tools/comment-history.py --list history   # every hit of one class, file:line: text
"""

import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

ORDINAL = (
    r"(?:first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth|eleventh|twelfth"
    r"|[a-z]+teenth|[a-z]+tieth|hundredth|thousandth"
    r"|[a-z]+-(?:first|second|third|fourth|fifth|sixth|seventh|eighth|ninth))"
)
NUMBER = (
    r"(?:[0-9]+|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|[a-z]+teen"
    r"|twenty|thirty|forty|fifty|sixty|seventy|eighty|ninety|hundred|thousand|dozen"
    r"|several|many|few|consecutive|hundreds of|thirty-odd|[a-z]+-(?:one|two|three|four|five"
    r"|six|seven|eight|nine))"
)

# A session this program or the desktop has, not one of this project's rounds. Checked first, so a
# phrase such as "the session bus" is never read as a pointer at a round.
LEGITIMATE = re.compile(
    r"session bus|(?:wayland|x11|desktop|login|graphical|user|fuse|viewing|editing) session"
    r"|session's (?:clipboard|bus)|clipboard of the session|<session>"
    r"|(?-i:\bSession\b)|for the session\b|a long session|in every session|session may run"
    r"|this session (?:added|attached|detached)|session (?:has one|offers)"
    r"|where the session|neither protocol answers|about the session|a session neither"
    r"|generalised to a session|a \*session\* visits|with no session|within a session"
    r"|(?:kde|measuring|real|slow) session|the whole session|'s session does"
    r"|by this session|never seen this session|session in which a person"
    r"|edit of a session|a session opens|session's$|session is not asked|offers it to the session"
    r"|a session that (?:measures|felt)|rest of the session|from their session"
    r"|session that fixed it, for the history file|with this session, and|name for this session"
    r"|session bookkeeping",
    re.IGNORECASE,
)

HISTORY = re.compile(
    r"hundred-and-|thousand-and-"
    rf"|\b{ORDINAL}\W{{0,3}}sessions?\b"
    r"|\bsessions? [0-9]"
    rf"|\b{NUMBER}\W{{0,3}}sessions\b"
    r"|\b(?:this|that|the same|a later|an earlier|the previous|the next|the last|one|whose"
    r"|which|the|each|every|some|a) session\b"
    r"|\bsessions? (?:of|later|earlier|ago|before|since|after|in a row|while|with|on|were)\b"
    r"|\buntil session|\bsince session|\bin session\b",
    re.IGNORECASE,
)

SWEEP = re.compile(r"hundred-and-|session")


def hits(roots):
    """Every line CLAUDE.md's grep matches under `roots`, as (path, line number, text)."""
    for top in roots:
        for directory, subdirectories, files in os.walk(os.path.join(ROOT, top)):
            subdirectories[:] = sorted(d for d in subdirectories if d not in ("target", ".git"))
            for name in sorted(files):
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(directory, name)
                with open(path, encoding="utf-8", errors="replace") as source:
                    previous = ""
                    for number, text in enumerate(source, 1):
                        text = text.rstrip("\n")
                        if SWEEP.search(text):
                            yield os.path.relpath(path, ROOT), number, previous, text
                        previous = text


def comment_of(text):
    """The comment part of a line, without its `//`, `///` or `//!`, or `None` for code alone."""
    at = text.find("//")
    return None if at < 0 else text[at:].lstrip("/!")


def strip_code_spans(comment):
    """A comment with its code spans removed; an unpaired backtick is left where it is."""
    return re.sub(r"`[^`]*`", "", comment)


def classify(previous, text):
    """The class of one hit; see the module comment for the order and the reasons.

    A comment wraps at a hundred columns, so "the four-hundred-and-seventy-first" often ends one
    line and "session" begins the next: the tail of the line before, when it is a comment too, is
    read with the hit so that a wrapped ordinal is seen as one.
    """
    comment = comment_of(text)
    if comment is None or not SWEEP.search(comment):
        return "code"
    # Code spans go first — a `session` in backticks is an identifier — and each line's are paired
    # left to right on its own, so the prose between two spans is never mistaken for one.
    comment = strip_code_spans(comment)
    before = comment_of(previous)
    if before is not None:
        comment = strip_code_spans(before)[-60:] + " " + comment.lstrip()
    # A legitimate phrase is removed rather than excusing its line, so a line naming the session
    # bus and a round's ordinal is still history.
    rest = LEGITIMATE.sub("", comment)
    if HISTORY.search(rest):
        return "history"
    if SWEEP.search(rest):
        return "unread"
    return "legitimate"


def main():
    listing = None
    if len(sys.argv) == 3 and sys.argv[1] == "--list":
        listing = sys.argv[2]
    elif len(sys.argv) != 1:
        sys.exit(__doc__)

    # CLAUDE.md's own command, run as written, so its number is not this script's reading of it.
    grep = subprocess.run(
        ["grep", "-rE", "hundred-and-|session", "--include=*.rs", "crates", "tools"],
        cwd=ROOT, capture_output=True, text=True, check=False,
    )
    if grep.returncode > 1:
        sys.exit(f"grep failed: {grep.stderr}")
    print(f"{len(grep.stdout.splitlines())} lines match CLAUDE.md's sweep over crates and tools")

    for label, roots in (("crates and tools", ("crates", "tools")), ("raster", ("raster",))):
        counts = {"code": 0, "legitimate": 0, "history": 0, "unread": 0}
        for path, number, previous, text in hits(roots):
            kind = classify(previous, text)
            counts[kind] += 1
            if kind == listing:
                print(f"{path}:{number}: {text.strip()}")
        if listing is None:
            print(
                f"{label}: {counts['history']} comment line(s) of history, "
                f"{counts['unread']} unread, {counts['legitimate']} legitimate, "
                f"{counts['code']} in code"
            )


if __name__ == "__main__":
    main()
