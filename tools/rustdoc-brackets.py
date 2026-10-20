#!/usr/bin/env python3
"""Answer rustdoc's two mechanical classes of broken link, from its own error log.

    env RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items -p <crate> \\
        > <log> 2>&1
    python3 -I -B tools/rustdoc-brackets.py [--apply] [--root DIR] <log>
    python3 -I -B tools/rustdoc-brackets.py --self-test

(`-B`, because `-I` ignores `PYTHONDONTWRITEBYTECODE`, and a module imported from `tools/` would
leave a `tools/__pycache__` that the workspace glob `tools/*` reads as a crate: trap 114.)

ADR 1938 gives each class of documentation warning one answer, so that a crate joining the docs
gate is mechanical. Two of those answers need no judgement, and this script makes them:

- **a letter a quotation lowered**, `"[t]he`: rustdoc reads the bracket as a link to `t` and
  reports it unresolved, with its help line about escaping. The bracket is escaped, `"\\[t]he`,
  which keeps `CLAUDE.md`'s mark of the change and which the blockquote gate's `quote::normalise`
  drops with the brackets. Only a bracket whose text holds no backtick is taken: a backticked name
  that does not resolve is a link somebody meant, and it is listed for a hand.
- **a public page linking a private item**: the link `` [`name`] `` becomes the code span
  `` `name` ``. Only that exact shape is taken — a link with an explicit target, `[..](..)` or
  `[..][..]`, or one a reference definition in the same file names, is listed for a hand, because
  the definition would be left behind.

Everything else in the log — a name that is two kinds of item, a stale path, a test-only item, a
redundant target — is printed as a list for a hand, with its file, line, column and message.

**Where an edit is made** (habit 135): rustdoc reports the column of the link's *text*, inside the
bracket and the backticks, not the bracket itself. The column is a character column (the lines
carry `§` and `—`, which a byte column would miscount). From it the script walks back over at most
one backtick and asserts a `[` is there and that the line is a `///` or `//!` comment line with the
bracket after the marker; a location that fails either is a failure of the whole run, and nothing
is written. The same location reported twice (an item re-exported, documented once) is one edit.
Edits on one line are made from the right, so an earlier edit never moves a later column, and the
count of edits made is asserted against the count of errors taken.

Without `--apply` the run writes nothing and prints what it would do. Exit 0 when every mechanical
error was answered (or would be), 1 when an anchor did not hold, 2 on a usage error.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

LOCATION = re.compile(r"^\s*--> (?P<path>[^:]+):(?P<line>\d+):(?P<column>\d+)\s*$")
UNRESOLVED = re.compile(r"^error: unresolved link to `(?P<name>.*)`$")
PRIVATE = re.compile(r"^error: public documentation for `.*` links to private item `(?P<name>.*)`$")
ESCAPE_HELP = "to escape `[` and `]` characters"
COMMENT = re.compile(r"^\s*//[/!]")
DEFINITION = re.compile(r"^\s*//[/!]\s*\[(?P<label>[^\]]+)\]:")


@dataclass(frozen=True)
class Error:
    """One error block of the log: its first line, where it points, and its help lines."""

    message: str
    path: str
    line: int
    column: int
    helps: tuple[str, ...]


@dataclass(frozen=True)
class Edit:
    """One planned change: at character `bracket` of `line` in `path`, the class it answers."""

    kind: str
    path: str
    line: int
    bracket: int
    close: int


def parse(log: str) -> list[Error]:
    """Every `error:` block of a cargo log that points at a file, in the log's order."""
    errors: list[Error] = []
    block: list[str] = []

    def close_block() -> None:
        if not block:
            return
        location = next((m for m in map(LOCATION.match, block[1:]) if m), None)
        if location is not None:
            helps = tuple(text.strip() for text in block if text.strip().startswith("= help:"))
            errors.append(
                Error(
                    message=block[0],
                    path=location["path"],
                    line=int(location["line"]),
                    column=int(location["column"]),
                    helps=helps,
                )
            )
        block.clear()

    for text in log.splitlines():
        if text.startswith(("error", "warning")):
            close_block()
            if text.startswith("error"):
                block.append(text)
        elif block:
            block.append(text)
    close_block()
    return errors


def bracket_of(text: str, column: int) -> int | None:
    """The index of the `[` a reported column's link text opens with, or `None`."""
    index = column - 1
    if index < 1 or index > len(text):
        return None
    index -= 1
    if index >= 0 and text[index] == "`":
        index -= 1
    if index >= 0 and text[index] == "[":
        return index
    return None


def comment_marker_end(text: str) -> int | None:
    """Where a `///` or `//!` line's text starts, or `None` for any other line."""
    match = COMMENT.match(text)
    return match.end() if match else None


def classify(
    error: Error, lines: list[str], definitions: set[str]
) -> tuple[str, Edit | None, str]:
    """The class of one error: `letter`, `private` or `hand`, its edit, and why it failed if so.

    A failed anchor is the class `anchor`, which stops the run (habit 135).
    """
    unresolved = UNRESOLVED.match(error.message)
    private = PRIVATE.match(error.message)
    if not unresolved and not private:
        return "hand", None, ""
    if unresolved and not any(ESCAPE_HELP in help_line for help_line in error.helps):
        return "hand", None, ""
    if error.line < 1 or error.line > len(lines):
        return "anchor", None, "the line is past the file's end"
    text = lines[error.line - 1]
    if is_definition(lines, error.line - 1):
        # A reference definition, `/// [label]: target`: rustdoc points at the target, which is
        # a path to re-point or a definition to remove with its uses, and both are a hand's.
        return "hand", None, ""
    marker = comment_marker_end(text)
    bracket = bracket_of(text, error.column)
    if bracket is None:
        return "anchor", None, "no `[` at the column or one backtick before it"
    if marker is None or bracket < marker:
        return "anchor", None, "the bracket is not inside a `///` or `//!` comment"
    close = text.find("]", bracket + 1)
    if close == -1:
        return "hand", None, ""
    inside = text[bracket + 1 : close]
    after = text[close + 1 : close + 2]
    if unresolved:
        if "`" in inside or "[" in inside:
            return "hand", None, ""
        return "letter", Edit("letter", error.path, error.line, bracket, close), ""
    shape = re.fullmatch(r"`([^`]+)`", inside)
    if shape is None or after in ("(", "[") or inside in definitions:
        return "hand", None, ""
    return "private", Edit("private", error.path, error.line, bracket, close), ""


def is_definition(lines: list[str], index: int) -> bool:
    """Whether line `index` is a reference definition, `/// [label]: target`.

    CommonMark's definition cannot interrupt a paragraph, so a line that only looks like one —
    a sentence's second line opening with a link and a colon — is one only after a blank comment
    line, another definition, or the start of the comment.
    """
    if not DEFINITION.match(lines[index]):
        return False
    if index == 0:
        return True
    previous = lines[index - 1]
    marker = comment_marker_end(previous)
    return marker is None or not previous[marker:].strip() or is_definition(lines, index - 1)


def reference_definitions(lines: list[str]) -> set[str]:
    """The labels a file's doc comments define as references, `/// [label]: target`."""
    labels = set()
    for index, text in enumerate(lines):
        match = DEFINITION.match(text)
        if match and is_definition(lines, index):
            labels.add(match["label"])
    return labels


def apply_edits(text: str, edits: list[Edit]) -> str:
    """One line with its edits made from the right, so no edit moves another's column."""
    for edit in sorted(edits, key=lambda e: e.bracket, reverse=True):
        if edit.kind == "letter":
            assert text[edit.bracket] == "[", "the anchor moved under an earlier edit"
            text = text[: edit.bracket] + "\\" + text[edit.bracket :]
        else:
            inside = text[edit.bracket + 1 : edit.close]
            assert text[edit.bracket] == "[" and text[edit.close] == "]", "the anchor moved"
            text = text[: edit.bracket] + inside + text[edit.close + 1 :]
    return text


def run(log_text: str, root: Path, apply: bool, out=sys.stdout) -> int:
    """Plan every edit the log's errors call for, and make them when `apply` is set."""
    errors = parse(log_text)
    seen: set[tuple[str, int, int]] = set()
    unique = []
    for error in errors:
        key = (error.path, error.line, error.column)
        if key not in seen:
            seen.add(key)
            unique.append(error)

    files: dict[str, list[str]] = {}
    plans: dict[tuple[str, int], list[Edit]] = {}
    counts = {"letter": 0, "private": 0, "hand": 0, "anchor": 0}
    hand: list[Error] = []
    failures: list[tuple[Error, str]] = []
    for error in unique:
        if error.path not in files:
            path = root / error.path
            if not path.is_file():
                failures.append((error, "the file does not exist under the root"))
                counts["anchor"] += 1
                continue
            files[error.path] = path.read_text(encoding="utf-8").split("\n")
        lines = files[error.path]
        kind, edit, why = classify(error, lines, reference_definitions(lines))
        counts[kind] += 1
        if kind == "hand":
            hand.append(error)
        elif kind == "anchor":
            failures.append((error, why))
        elif edit is not None:
            plans.setdefault((edit.path, edit.line), []).append(edit)

    print(
        f"{len(errors)} errors in the log, {len(unique)} locations: "
        f"{counts['letter']} letters escaped, {counts['private']} private links made code spans, "
        f"{counts['hand']} for a hand, {counts['anchor']} anchors that did not hold",
        file=out,
    )
    for error, why in failures:
        print(f"  anchor  {error.path}:{error.line}:{error.column}  {why}  ({error.message})", file=out)
    if failures:
        print("nothing written: an anchor did not hold (habit 135)", file=out)
        return 1
    for error in hand:
        print(f"  hand    {error.path}:{error.line}:{error.column}  {error.message}", file=out)

    made = 0
    for (path, line), edits in plans.items():
        lines = files[path]
        lines[line - 1] = apply_edits(lines[line - 1], edits)
        made += len(edits)
    assert made == counts["letter"] + counts["private"], "an edit was lost between plan and line"
    if apply:
        for path in sorted({path for path, _ in plans}):
            (root / path).write_text("\n".join(files[path]), encoding="utf-8")
        print(f"{made} edits written to {len({path for path, _ in plans})} files", file=out)
    else:
        print(f"{made} edits planned; --apply writes them", file=out)
    return 0


# The self-test plants a file with one of each class and each failure, and a log that names them
# as rustdoc does — the column at the link's text, the same location twice — and asserts the
# script's answer for each (trap 13: the instrument is shown to find what was planted).
PLANTED_SOURCE = "\n".join(
    [
        '//! §14.1 says: "[t]he whole clause" and "[i]f each" — two letters on one line.',
        "//! A private link [`helper`] and a [`kept`](crate::kept) explicit one.",
        "/// A stale name [`Gone`] is somebody's link.",
        "fn helper() {}",
        "let x = [y];",
        "//! A labelled link [`labelled`] whose definition stays.",
        "//!",
        "//! [`labelled`]: crate::labelled",
        "/// A sentence whose second line opens with a link,",
        '/// [`Self::Cleared`]: "[t]he default", which is no definition.',
        "",
    ]
)

PLANTED_LOG = """\
    Checking planted v0.1.0
error: unresolved link to `t`
 --> src/lib.rs:1:19
  |
1 | //! §14.1 says: "[t]he whole clause"
  |                  ^ no item named `t` in scope
  |
  = help: to escape `[` and `]` characters, add '\\' before them like `\\[` or `\\]`

error: unresolved link to `i`
 --> src/lib.rs:1:44
  |
  = help: to escape `[` and `]` characters, add '\\' before them like `\\[` or `\\]`

error: unresolved link to `i`
 --> src/lib.rs:1:44
  |
  = help: to escape `[` and `]` characters, add '\\' before them like `\\[` or `\\]`

error: public documentation for `planted` links to private item `helper`
 --> src/lib.rs:2:22
  |
  = note: this link will resolve properly if you pass `--document-private-items`

error: public documentation for `planted` links to private item `kept`
 --> src/lib.rs:2:39
  |

error: unresolved link to `Gone`
 --> src/lib.rs:3:20
  |
  = help: to escape `[` and `]` characters, add '\\' before them like `\\[` or `\\]`

error: public documentation for `planted` links to private item `labelled`
 --> src/lib.rs:6:23
  |

error: public documentation for `planted` links to private item `crate::labelled`
 --> src/lib.rs:8:21
  |

error: unresolved link to `t`
 --> src/lib.rs:10:26
  |
  = help: to escape `[` and `]` characters, add '\\' before them like `\\[` or `\\]`

error: could not document `planted`
"""

EXPECTED_SOURCE = PLANTED_SOURCE.replace('"[t]he', '"\\[t]he').replace('"[i]f', '"\\[i]f').replace(
    "[`helper`]", "`helper`"
)


def self_test() -> int:
    """Run the planted cases; 0 when every one holds."""
    fails = 0

    def check(name: str, condition: bool, detail: str = "") -> None:
        nonlocal fails
        if not condition:
            fails += 1
            print(f"  FAIL {name} {detail}")

    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / "src").mkdir()
        source = root / "src/lib.rs"

        source.write_text(PLANTED_SOURCE, encoding="utf-8")
        report: list[str] = []

        class Sink:
            def write(self, text: str) -> None:
                report.append(text)

        status = run(PLANTED_LOG, root, apply=False, out=Sink())
        check("a dry run exits 0", status == 0, str(status))
        check("a dry run writes nothing", source.read_text(encoding="utf-8") == PLANTED_SOURCE)
        status = run(PLANTED_LOG, root, apply=True, out=Sink())
        text = "".join(report)
        check("an applied run exits 0", status == 0, str(status))
        check(
            "two letters on a line, a private link, and nothing else",
            source.read_text(encoding="utf-8") == EXPECTED_SOURCE,
            repr(source.read_text(encoding="utf-8")),
        )
        check(
            "the duplicate is one location",
            "9 errors in the log, 8 locations: 3 letters escaped, 1 private links" in text,
            text,
        )
        check("an explicit target is a hand's", "src/lib.rs:2:39" in text, text)
        check("a backticked unresolved name is a hand's", "src/lib.rs:3:20" in text, text)
        check("a labelled link is a hand's", "src/lib.rs:6:23" in text, text)
        check("a reference definition's target is a hand's", "src/lib.rs:8:21" in text, text)

        # A column one to the right of the link's text: no bracket behind it, so the run stops
        # and writes nothing, rather than splitting the text and leaving the bracket.
        source.write_text(PLANTED_SOURCE, encoding="utf-8")
        report.clear()
        shifted = PLANTED_LOG.replace("src/lib.rs:1:19", "src/lib.rs:1:20")
        status = run(shifted, root, apply=True, out=Sink())
        check("a column off the bracket exits 1", status == 1, str(status))
        check("a column off the bracket writes nothing", source.read_text(encoding="utf-8") == PLANTED_SOURCE)

        # A bracket in code, not in a comment: the same refusal.
        report.clear()
        in_code = PLANTED_LOG.replace("src/lib.rs:3:20", "src/lib.rs:5:10").replace(
            "unresolved link to `Gone`", "unresolved link to `y`"
        )
        status = run(in_code, root, apply=True, out=Sink())
        check("a bracket in code exits 1", status == 1, str(status))
        check("a bracket in code is named", "not inside a `///` or `//!` comment" in "".join(report))

    if fails:
        print(f"rustdoc-brackets self-test: {fails} case(s) failed")
        return 1
    print("rustdoc-brackets self-test: every case holds")
    return 0


def main() -> int:
    """Parse the arguments and run, or run the self-test."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("log", nargs="?", help="the cargo doc log, standard output and error")
    parser.add_argument("--apply", action="store_true", help="write the edits")
    parser.add_argument("--root", default=".", help="the directory the log's paths are under")
    parser.add_argument("--self-test", action="store_true", help="run the planted cases")
    arguments = parser.parse_args()
    if arguments.self_test:
        return self_test()
    if arguments.log is None:
        parser.print_usage(sys.stderr)
        return 2
    log_text = Path(arguments.log).read_text(encoding="utf-8")
    return run(log_text, Path(arguments.root), arguments.apply)


if __name__ == "__main__":
    sys.exit(main())
