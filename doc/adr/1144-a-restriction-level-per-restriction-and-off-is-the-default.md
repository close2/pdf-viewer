# 1144 — A restriction level per restriction, and `off` is the default

Status: accepted. Session 1147.
Context: `crates/viewer-core/src/command.rs` (`RestrictionPolicy`, `RestrictionLevel`,
`Command::Restrict`, `Command::Copy`), `crates/viewer-core/src/event.rs` (`Event::Copied`),
`crates/viewer-core/src/viewer.rs` (`Viewer::copy`, `Viewer::standing`, `Viewer::answer`),
`crates/viewer-core/src/open.rs` (`Held`), `crates/viewer-confined/src/protocol.rs`,
`crates/viewer-ffi/src/{abi,kinds,events,session}.rs`, `crates/viewer-ffi/include/quorra.h`,
`crates/viewer-host/src/policy.rs` (`RESTRICTIONS`, `restrictions`), the three windows,
`crates/viewer-core/tests/restriction_levels.rs`.
Builds: ADR 0212 (the reading), ADR 0803 (`Level`, `Verdict`, `decide`), ADR 0814 (the event, the
command and the four levels), ADR 0875 (two faces that ask), ADR 0604 (`IGNORE_RESTRICTIONS`),
ADR 1106 (the most recent host-supplied policy), `doc/todo/38`, `doc/ui-boundary.md`.
Clauses: ISO 32000-2 §7.6.4.1, §7.6.4.2 (Table 22), §12.8.2.2, §12.8.6, §14.8.2.5.

## 1. The census: 1 901 documents restrict copying or annotating and nothing consulted bit 5

Over the 90 763 files of `doc/pdf.js`, `doc/corpora/` and `corpus-cache/`: 90 340 open without a
password, 2 186 state an `/Encrypt` this reader reads, and 4 of those open as the owner, whom
§7.6.4.1 gives "full (owner) access". Of the rest, **1 539 withhold copying** (bit 5), 1 655
withhold annotating (bit 6), 1 523 withhold filling in (bits 6/9), 2 025 withhold modifying (bit 4)
and 170 withhold printing (bit 3); **1 901 withhold copying or annotating.** Bit 5 was read by
`pdf-transform` and by nothing a person could press, which is what this round changes.

## 2. One level per operation, because Table 22 is eight subjects and not one

`Command::Restrict` carried one `RestrictionLevel` for the whole window. Table 22 does not state
one permission — it states eight positions with eight different subjects — and a reader who wants
to be asked before text leaves the program has said nothing whatever about whether they want to be
asked before each keystroke into a form field. One level could not tell those apart, and the person
forced to choose the stricter of the two would have had a restriction imposed on them by the shape
of a type. So `Command::Restrict` now carries a `RestrictionPolicy`: one `RestrictionLevel` per
`pdf_model::restriction::Operation`, `RestrictionPolicy::uniform` being the value it used to carry.

**A variant's shape changed rather than a message added**, which `doc/ui-boundary.md` records as its
own preference and which made every consumer fail to compile. The policy is **total** over
`Operation`, including `Print` and `Assemble`, which this crate performs today in neither case: a
policy with a hole in it would have to grow a message to fill it, and the day a window gains a print
path the level is already the reader's to set.

## 3. `Off` is the default, in every face, because the owner's sentence says so

The default was `On`, on the argument that ignoring a document's restrictions unasked chooses for the
person in the other direction. `CLAUDE.md` principle 3 decides it the other way and says why:
restrictions "are low priority", "it shall always be possible to turn them off", and "[a] restriction
a reader cannot switch off is a restriction imposed on the reader by somebody else's file, and this
program is the reader's". Every other face already opened at `off` (ADR 0875's table): the viewer was
the one that did not. §7.6.4.1's `shall` is not thereby unread — it is one command away, and
`--restrictions=on` is how a reader keeps it.

## 4. A copy is an operation, so it is a command and not a query

`doc/todo/38` left bit 5 unconsulted because "this crate hands a host a *readback* — the same
`Query::Selection` that a drag asks sixty times a second in order to draw a highlight. Refusing that
would refuse the highlight." A query also produces no events, so nothing can wait on one, and the
*ask* level cannot exist over a readback at all. So the gesture is `Command::Copy`; the policy is
consulted in `Viewer::standing` exactly where every other operation's is; and the text comes back as
`Event::Copied`, carrying **both** of §14.8.2.5's orders because `viewer_host::copied` already makes
the choice between them once for three windows and a C caller. Nothing selected sends nothing at all.
§14.9's tree stays a query, gated by nothing, on Table 22's own carve-out: "for the limited purpose
of providing this content to assistive technology, a PDF reader should behave as if this bit was set
to 1".

`open::Held` is an enum now — an edit or a copy — because both are resolved before they are held, for
`Done`'s reason: what goes ahead on a `yes` is what was asked about at the moment it was asked.

## 5. The interface, which is a command line and says so

`doc/todo/38`'s deferral was lifted by the project owner on 2026-09-16, and this is its first piece.
`--restrictions=copy:ask,annotate:on` is `viewer_host::restrictions`, one parser for the three
windows, taking `RestrictionPolicy::word`'s six operation names and `Level::as_str`'s four levels so
that a person reads the same words everywhere; a bare `--restrictions=on` is all six, which is
`pdf-transform`'s existing spelling. **A command line is not the menu the entry still wants**, and
that sentence is in the code rather than only here, because a file that said otherwise is what left
two hosts without a way out for the whole of their lives (ADR 0604).

The C ABI gains `quorra_restrict_operation`, `quorra_copy` and `quorra_event_copied`, and
`QUORRA_EVENT_KIND_COUNT` moves 20 → 21. `QUORRA_ABI_VERSION` does not move: no struct crosses by
value and an entry point added is one an old caller never calls. The wire spells a policy as six
level bytes in `RestrictionPolicy::OPERATIONS`'s order — the one order the wire, the ABI and the
command line all enumerate a policy in, so that the three cannot drift.

## 6. What this does not touch

`pdf-model` still decides nothing: `asserted` reads the file, `Level::verdict` is a pure function,
and the policy and the consultation are `viewer-core`'s (`CLAUDE.md` principle 3). §12.8.2.3's
withdrawal of a `/UR3` whose grant a save exceeds is untouched and stays reached from
`ViewState::save` with no policy in scope — turning a restriction off is the reader's, making the
file assert something untrue is not. `launch_path` is unmoved because `/P` is read where the
encryption dictionary already is, and only when an operation asks; `raster_golden` is unmoved
because no pixel is decided here.
