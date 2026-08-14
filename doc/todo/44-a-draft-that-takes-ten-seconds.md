# A draft that takes ten seconds to appear, and a third of a second per frame after that

Status: **evaluation owed** — the owner asked whether displaying this document can be improved,
and supplied a trace. A first reading of that trace is below; nothing is diagnosed to the point
of a fix, and the trace itself has one hole that must be closed first.
Priority: 44
Corpus: none — `tmp/Entwurf.pdf` is the owner's own document (49.7 MB, one page, 58 009 display
commands), outside the tree like `doc/todo/28`'s, with its trace beside it as
`tmp/trace.entwurf.txt` (also untracked; the numbers below are copied from it so this file
survives the trace's deletion, taken 2026-08-14 on the owner's machine, AMD 890M/RADV).
Clauses: none — this is a performance item; §2's launch rules in `CLAUDE.md` are the standard it
is judged against
Code: `crates/viewer-ui/src/bin/pdf-viewer/timing.rs` (the launch table and frame lines),
`crates/render-quorra` (`encode`), `crates/pdf-model/src/content/` (whatever the missing
launch line turns out to name)

## What the trace says, first reading

One page of 58 009 commands — a vector drawing (a plan; "Entwurf"), untagged, no reports, no
fallback frames, nothing refused. Two separate costs:

1. **Ten seconds to the first present, and the launch table cannot say where.** Its last two
   lines are `document joined 505.704 ms (+15.120)` and `first present 10220.077 ms
   (+9714.373)`. The first frame itself accounts for 2 698 ms of that (`scene 982.1`,
   `device 1706.6`, of which `encode 978`), and `needs render` is first logged at 7.5 s — so
   roughly **seven seconds sit between joining the document and having a display list, with no
   line naming them.** Interpretation of the 58 009 commands is the suspect, and a suspect is
   exactly what a trace exists to replace.

2. **A static scene re-encoded every frame.** 28 frames: median 393.1 ms, p90 1 196.9, max
   2 698.4. `device` is the bulk (median 320.4) and inside it `encode` is median 233.8 ms
   against `execute` 0.5 ms — the GPU draws in half a millisecond what quorra spends a quarter
   of a second re-encoding, **per frame, for a display list that did not change** (the culled
   frames — `40 up, 58029 culled` — still pay 112–190 ms in `device`). `scene` adds a median
   50.2 ms of translation on top, same story. This is `doc/todo/45`'s "quorra's `encode`" row
   wearing a witness: nothing in the loop caches the encoded scene between frames whose display
   list is identical, and this document makes that the whole user experience — every zoom step
   costs 160–310 ms.

## What the evaluation owes, in order

1. **Close the trace's hole first** (the owner's own instruction: if the trace does not say
   enough, make `--trace` say it). The launch table in `timing.rs` gets stages between
   `document joined` and `first present` — at minimum *interpreted* (the display list exists,
   with its command count) and *first scene built*; the frame line already splits the rest.
   Re-run on this document; the seven seconds get a name with no guessing.
2. **Attribute the interpretation cost** (callgrind, not the wall clock, if the machine is
   shared): 58 009 commands from a 49.7 MB stream — is it the lexer, resource lookups, path
   arithmetic, or something a memo already priced (`doc/todo/41`'s population argument applies:
   one page opened once has no repeats, so the decoded-stream memo is not the lever here)?
3. **Price the encode cache** — an encoded-scene reuse for frames whose display list and scale
   are unchanged, which is `doc/todo/45`'s item made concrete. The trace's own arithmetic says
   what it would buy on this document: median frame 393 ms → the ~60 ms the non-device half
   costs, and a zoom step under 100 ms instead of a third of a second. Whether it belongs in
   `render-quorra` or in the window's backend is the design half, beside ADR 0297's precedent
   (the reduced raster kept in the window's backend).
4. Only then decide what is worth building, with the numbers beside each option — this file is
   an evaluation item, and `CLAUDE.md` forbids optimizing what nobody measured.

## Cross-references

`doc/todo/45` (where a frame goes — quorra's `encode` was already its open row),
`doc/todo/42` (the launch path; its five items are about the program's own startup, where this
document's ten seconds are one page's interpretation — different lever, same gate),
ADR 0297 (a per-frame recomputation kept out of the loop once before).
