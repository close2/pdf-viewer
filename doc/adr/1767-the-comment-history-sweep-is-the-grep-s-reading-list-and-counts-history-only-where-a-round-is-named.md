# 1767 — The comment-history sweep is the grep's reading list, and counts history only where a round is named

Session 1466. Status: **accepted** and **built**. Restates what ADR 1403 built `tools/comment-history.py`
for, now that the rule's debts are gates; amends ADR 1403's `history` class. Supersedes nothing.
Context: ADRs 1023, 1403, 1680, 1698; traps 13, 39, 69.
Code: `tools/comment-history.py` (its module comment, `HISTORY`, `LEGITIMATE`, `strip_quotations`).

## 1. What it printed

`python3 tools/comment-history.py` read `crates and tools: 22 comment line(s) of history`, and every
one of the 22 was this program's or the machine's session: `viewer-ffi`'s session that supplies no
runner (2), the session `setsid` starts and the batch branch's first session number (`batch.rs`, 6),
`ledger_notes.rs` and `round_numbers.rs` describing the shapes they hold (13), and a record's session
number (`state_sections.rs`, 1). ADR 1680 counted the same class at 18 and found the same: not one a
round's history. A count above zero that is never a finding is a signal that always fires (trap 39).

## 2. Why it is kept, and for what

ADR 1680 owed the script a `round <number>` arm; ADR 1698 built `round_numbers.rs` instead, which lexes
comments and holds that form at zero, and `spelled_ordinals.rs` holds the `-and-` ordinal and
`Session <number>` at zero. So the script is no longer where a debt is counted. It stays because
`CLAUDE.md` names its grep as the measure of what is owed and the grep's count is mostly this program's
nouns: the script is the reading of that count, and it still sees the shapes no gate holds (a lowercase
`session <number>`, a small ordinal before `session`, a pointer such as "this session"). Retiring it
would leave the grep's number unread and those shapes uncounted.

## 3. The class, narrowed

- `history` is a session named or counted — an ordinal or a number tied to the word, `session` and
  digits — or a pointer that can only be at one round ("this", "that", "the same", "a later", "an
  earlier", "the previous", "the next", "the last session"). The article forms ("a", "the", "every",
  "each", "which" session) move to `unread`, the reading list, because whether "a session supplies no
  runner" is a round is a question about English.
- A double-quoted span is a mention and is removed as a code span is, after a wrapped line is joined to
  its neighbour, so a quotation that wraps pairs its own marks.
- `LEGITIMATE` gains the setsid form ("a session of its own") and the branch's first session number.

**Calibrated** (trap 13), in the function and not in the tree: of fifteen planted lines — the `-and-`
ordinal wrapped across two lines and whole, `session 945`, "for four hundred sessions", "a later
session", "the fifth session", a legitimate phrase beside a numbered one, two article forms, four
legitimate shapes and a line of code — fourteen classify as planted. The fifteenth, "this session
added the reader", reads legitimate by a `LEGITIMATE` phrase older than this change (a desktop
session that added a device), which is that list's choice and is left. The tree now reads 0 history,
20 unread, 109 legitimate, 113 in code; `raster/` 0, 0, 4, 0.
