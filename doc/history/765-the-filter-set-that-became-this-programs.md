# 765 — The filter set that became this program's, and the head a mention took off the list

The errata selection rule's seventh use, and its third run with the fourth step in place. The full
ranking's head is §7.4.1, `implemented`, and the erratum that put a round on it turns Table 6 from a
description of what documents contain into an obligation this program owes. The live ranking's head,
meanwhile, moved for a reason that is nobody's reading: a sentence written to *disown* two issue
numbers named them.

Date: 2026-08-25.
ADR: [0691](../adr/0691-the-filter-set-that-became-this-programs.md).

Touched: `crates/pdf-syntax/tests/filters.rs` (one new test), `doc/conformance/ledger.toml` (§7.4,
§7.4.1), `doc/errata-read.md`, `doc/todo/01`, the ADR and this file. **No pixel moves and no
behaviour moves**: what the round adds is evidence for a requirement already met, and what it removes
is a number that never added up.

## What the rule gave

307 issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret; **118 are named
nowhere in this tree** at this round's base, and that figure is two short — see below. Over live rows
the head is **§7.6.4.1 and §7.6.6 with six annotations apiece**. Over **every** row the head is
**§7.4.1 with eight under two issues**, `implemented` — the figure 750 measured from outside before
the fourth step existed and 760 named as second, reproduced before it was trusted, which is 755's
calibration practice for the third round running. Both issues fall under one `emit` heading, so
nothing had to be reassembled; the check was still run.

The population is 116 after this round.

## What the issues said

`doc/errata-read.md` has both with the rectangle that places each, and two of the placements are
exact to the hundredth of a point — the strike's `/Rect` and `pdftotext -bbox`'s word box are the
same four numbers, which picks the fourth `are` out of four on the page with nothing left to argue
about.

- **#216**, three annotations. Two are a producer's: *are* becomes *shall be* in "which decoding
  filter or filters to use are specified in the stream dictionary", and *All stream data shall follow
  the appropriate format(s) as described below.* is inserted after it. The third strikes *files* from
  "PDF files support a standard set of filters that fall into two main categories" and writes
  *processors shall*.
- **#527**, two annotations, both on EXAMPLE 3. The base-85 stream the standard prints has no `~>`
  end-of-data marker; one caret adds it and the other turns `/Length 447` into 449. Two bytes of
  marker, two of length — the halves check each other. This tree quotes the example's *arrangement*
  and never its bytes, so nothing here transcribed the omission.

## What reading them made this round look at

**Table 6 became a closed set this program owes, and nothing asserted it.** Every filter in the table
has a test of its own *output*; not one of them asks whether the table is covered. So a name dropped
from `decode_reported`'s match arms or from `is_image_codec` becomes `FilterRefusal::Unsupported` —
which is exactly what a name from no table gets — with the rest of the crate green.
`every_filter_table_6_names_is_supported_under_both_of_its_spellings` walks the ten names and Table
92's seven abbreviations, seventeen spellings, asking of each whether it decodes here or is an image
codec, and whether two spellings of one filter answer alike on the same bytes. **Calibrated per trap
13 against two plants the rest of the crate cannot see**: `JPXDecode` out of `is_image_codec`, and
`A85` off `ASCII85Decode`'s arm. Under each, this test fails alone — 104 unit tests and seven other
integration tests stay green.

That is 755's shape for a third row and a third mechanism: a round trip that could not fail, then a
sentence about a sibling row's status, now a set with no closure check. The three share nothing but
the status, which is the fourth step's whole argument.

**And §7.4's own row could not add up.** It called Table 6's ten "[f]our … stream filters implemented
here, one … a pass-through … and four … image codecs" — nine — while the same note has said since
ADR 0587 that all *five* of Table 6's byte-to-byte filters can be windowed. `filter.rs` decodes five.
`--bin counts` is right not to print it: a cardinal is a claim about a family there only where it
governs one of the ledger's own words for a row, and *stream filters* is not one.

## The finding about the instrument, and it is the eighth blindness

**A mention is not a use, and step 2's grep cannot tell them apart.** 760 recorded — correctly — that
an early draft of its ADR had written two issue numbers in full and taken §14.8.5.3 off the ranking
without a verdict. The sentence recording that fix writes both numbers with the `Issue #` prefix,
inside backticks, in a history file, in order to say that they should not be written that way. Both
therefore count as named, and the live head this rule had carried for four consecutive uses is not in
the live ranking at all.

Measured rather than argued: restoring the two reproduces 760's own figures exactly — 120 named
nowhere, §14.8.5.3 at the live head with seven annotations — and without them the live head is six.
Nothing else in the tree names either number.

The repair is a rule about writing, because no third grep can see the difference: a bare-number
search collides with `doc/HAYRO_ISSUES.md`, and excluding `doc/history/` would silence true records.
**A sentence about the form of an issue number must not contain one.** Neither number is written in
`doc/errata-read.md`, in the ADR, in `doc/todo/01` or here — this file included, which is the point —
so both are back in the population with no verdict, which is where they belong.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
`tools/round.sh` says this **is** a fifth round, so §2's sequence ran whole and §5 rebuilt and
installed the binaries — which `round.sh` had flagged as absent from `target/`. Both workers were
built before any gate that decodes an image (trap 10).

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green. The only clippy output was `viewer-qt`'s
cold-build gcc `-Wmaybe-uninitialized` lines, which §2 documents as not lints. **The oracle says no
pixel moved.**

**One gate failed once on a loaded machine and passed on a quiet one, and it is worth naming rather
than passing over.** `viewer-host::a_launch_waits_for_page_one_instead_of_polling_for_it` failed in a
`nextest --workspace` run taken at a one-minute load average of 17, with three neighbouring rounds
building, and passed both alone and in a full re-run. Its own doc comment predicts exactly that — "on
a machine that gave the drawing thread no core the wait would run out and the page would arrive
through the poll instead" — and `Drawing::settle` takes a budget, so the assertion is on a clock
after all. It is §2's "run the sequence on a quiet machine" arriving from the side nobody expected:
not a reference renderer losing its budget, but this tree's own thread losing a core to a neighbour.
Left as an observation for a `viewer-host` round rather than changed by an errata round.

Sixteen sweeps run before the edits and after them. `quoted` and `unpriced` were not run: this round
touches no page-list note and both take the oracle's log as their right-hand side.

`entries`, `unread`, `blockers`, `capabilities`, `callers` and `spec-errata check` and `moved` are
**byte-identical** in their report bodies. **Not one defect bucket moved:**

- `counts` 8319 ← 8294 sentences over 430 attributed counts, **149 the family agrees with, 58 "no
  such way" and 4 places counting one family twice, all unchanged**, with 223 attributed to a clause
  with no rows below it.
- `quotations` 6425 ← 6404 document quotations over 999 ← 997 documents with **diverging unchanged
  at 38**, and 1953 ← 1951 ledger quotations with **diverging unchanged at 2**.
- `tables` 6729 ← 6712 sentences with **key citations unchanged at 2477 — agreeing 2313, absent 100,
  contradicted denials 6, keyless 58**.
- `pointers` 8541 ← 8524 with **absent unchanged at 131** and **13 undefined unchanged**.
- `overstated` 136 ← 135 terms asserted with **contradicted unchanged at 8** in every rung.
- `overtaken` 587 ← 586 decision records with **47 overtaken unchanged**.
- `spec-errata applied` 784 ← 772 places naming an erratum over 57 420 ← 57 361 places read, with
  **the read-first list unchanged at 10, the corrections quoting retired wording at 90 and the
  places inside `errata-read.md` at 72**.
- `inapplicable` and `owed` moved only in the ordering of a term list — `JPXDecode` and `DCTDecode`
  each gained a naming file, this round's test — and in one place that is the sweep watching the
  round work: the cousin list under the vocabulary word *Four* loses §7.4, because the sentence that
  put it there is the wrong number this round corrected.
