# 1403 — History in comments is counted by shape, and a record's dead pointer is its date

Session 1283. Status: **accepted**. Two instruments `CLAUDE.md` and `doc/todo/01` already name, each
given the one distinction it could not make, and the populations each then read and worked.

Context: `tools/comment-history.py`, `tools/state.sh` (`comments`), `tools/conformance/src/pointers.rs`
(`Reach::Historical`, `RECORDS`, `Tree::project_of`), `doc/todo/01`'s eighth-sweep bullet. ADR 1023
states the comment rule; ADRs 0372 and 1391 are the pointer sweep and its pattern rung.

## 1. The comment sweep is counted by shape

`CLAUDE.md` names `grep -rE "hundred-and-|session" --include=*.rs crates tools` as how much of the
comment rule is still owed. Run as written it mixes four things: code (`let mut session`), a session
this program or the desktop has (a viewer's `Session`, "a file this session attached", the session
bus, a Wayland session), a round's session by ordinal, number or pointer, and prose neither decides.
Only the third is debt. `tools/comment-history.py` prints the grep's own count first, unchanged, then
sorts each hit, first match wins: outside a `//` comment is **code**; code spans are removed (paired
per line, so the prose between two spans is never read as one) and the legitimate phrases after them;
what still carries an ordinal, a number or a deictic session is **history**; what still says
`session` is **unread**, printed by name, because whether "a session that reads it" is advice or
history is a question about English. A comment wraps at a hundred columns, so the tail of the line
before is read with the hit. `raster/` is counted apart: the grep does not reach it and the rule does.

It is a count and a reading list, never a gate — ADR 1023's own reason: a rule about English applied
by a pattern teaches rounds to write around the pattern. `tools/state.sh comments` prints it.

## 2. A record's dead pointer is historical

`--bin pointers` printed some 250 absent pointers, and two thirds of them were written in records:
ADRs and reviews, and the dated correspondence with the render library and with `hayro`'s tracker
(`doc/QUORRA_*`, `doc/HAYRO_*`, `doc/quorra-*`), which name that tree's layout at the revision they
cite — `crates/quorra-gpu/…`, `crates/render-quorra/…` — before it was folded in under `raster/`. A
record is never rewritten (`CLAUDE.md`, *Where knowledge lives*), so a finding there can only be
fixed by breaking the rule the finding serves. `Reach::Historical` is its rung, and `RECORDS` names
the populations with the reason for each; `doc/history/` is not swept at all (ADR 0372).

Two further readings run before *absent*, because they are how a reader resolves the pointer:
from the root of the **sub-project** the file is in — `raster/` has a `CLAUDE.md` of its own, so its
`crates/raster-gpu/tests/archetypes.rs` is `raster/crates/…` — and from **beside the file**, so
`raster/fill.rs` in `src/compute.rs` is that crate's `src/raster/fill.rs`. Both are unit-tested.

What stays absent outside a record is breakage and was fixed where it is written: a moved file named
by its new path, a retired one by the ADR that retired it.

## What this does not decide

Which comment is history and which is the lesson is still read, not matched: a trap and a habit keep
their incidents (ADR 1023), and nothing under `doc/adr/`, `doc/history/` or `doc/reviews/` was
edited to shorten this count.
