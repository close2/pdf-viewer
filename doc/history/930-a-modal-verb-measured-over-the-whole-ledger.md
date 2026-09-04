# 930 — A modal verb measured over the whole ledger, and a screen annotation that reported a clause it has

Date: 2026-09-04.
ADR: 0900 (the twenty-fourth sweep — a modal verb as a discriminator, and what 214 rows said
back); 0901 (a `shall` in the prose after the table, and the five rows read).
Files: `tools/conformance/src/permitted.rs`, `tools/conformance/src/bin/permitted.rs`,
`tools/conformance/src/prose.rs`, `tools/conformance/src/entries.rs`,
`tools/conformance/src/lib.rs`, `crates/pdf-model/src/annotation.rs`,
`crates/pdf-model/tests/annotations.rs`, `doc/conformance/ledger.toml`,
`doc/todo/01-ledger-partial-rows.md`, `doc/adr/0900-*.md`, `doc/adr/0901-*.md`.

A coverage round in two halves, both set by session 928: measure whether its finding generalises,
then take the next band of `doc/todo/01`'s blame list.

## First half — the instrument, and the number

ADR 0896 said a sweep cannot find the rest of the population it named and offered a grep over the
note. `--bin permitted` is that question asked of the **standard** instead, because a row's note is
not evidence: the note's quotations are located in `doc/md/` and the verb is read off the
standard's own sentence holding each, and every `(table, key)` the note attributes is looked up in
the table's own row, where a row all of whose named entries the standard states as optional is
ADR 0896's shape. Beside every hit it prints ADR 0897's suggested column — the clause's `shall`
sentences outside its tables and NOTEs — which is also the tie-break within a rank.

**Of the 214 `partial` rows this round found, 105 quote a requirement of the standard and 109 do
not**: 49 naming only
optional entries, 3 whose strongest quoted verb is a permission, 12 quoting the standard with no
modal verb at all, 7 resting on a recommendation, 37 quoting nothing the conversion holds. Half
the population is a reading list, not a defect count, and the sweep exits zero.

**The calibration set the design rather than confirming it** (trap 13). Session 928's four rows
were restored to the ledger and run against. Under the rank as first written — *stated optional
**and** described with no `shall`* — three of the four are silent, because Table 122's `/Lang`
carries "The value shall be a Language-Tag as defined in BCP 47", a `shall` on the **value** that
a reader declining an optional entry never reaches. Loosened to the table's own word, §14.9.2 and
§14.9.2.2 land on rank 1, §12.11.5 on rank 3, and §14.7.4.2 is silent — which is right, since it
stayed `partial` for a real `shall`. Restored, all four are gone.

## Second half — five rows read, two moved

| row | what it rested on | verdict |
|---|---|---|
| **§12.5.6.18** | Table 190's `/MK`, "(Optional) An appearance characteristics dictionary" | **`partial` → `implemented`**, and a false report deleted |
| **§12.7.7** | "A script executed by an ECMAScript action **can** add the named page" | **`partial` → `implemented`**, on the exclusion |
| §8.11.1 | Table 98's `/Configs`, flagged rank 1 | kept — the note also names §8.11.4.5's "shall be reapplied", three words, under the sweep's minimum |
| §11.4.8 | its own NOTE's "can be significantly simplified" | kept — the clause states no `shall`, is a restatement by its own first sentence, and follows §11.4.4 and §11.4.6 |
| §11.7.4.4 | a portion whose shape cannot be stated, or an element that blends | kept, and its evidence corrected |

**§12.5.6.18 is the one that reached code.** Both of the clause's reader-facing `shall`s are in the
prose after Table 190, which a row enumerating the table never reached — ADR 0897's shape a second
time. One was not met: "If AP is not present, the screen annotation shall not have a default
visual appearance and shall not be printed", where `appearance::construct`'s catch-all answered
`its clause states no geometry` about a clause that states the absence of one. `Screen` now
answers `Decision::Nothing` beside `Projection`. That is the **third** correction of the same arm,
after §12.5.6.11's caret and §12.5.6.23's redaction, each time because the clause did speak and
the arm had not been asked. The clause's other `shall` binds a processor performing §12.6.4.14's
rendition action, which is `out-of-scope`, so it is vacated the way §12.11.1's is.

**Its cited test asserted a different clause** — the third such citation in two sessions, after the
two session 928 found:
`an_unknown_subtype_still_draws_its_normal_appearance` builds a `/Subtype /SomethingFromThePDF3Era`
and asserts Table 167's `Invisible` row. The new test states a `Screen` in both directions and was
calibrated by planting the arm it replaces.

**§12.7.7 the sweep could not have found**, and that is written down as the instrument's limit: the
row quotes the four `shall`s it implements — all of them about the *file*, run as invariants — while
its debt is a *can* in prose. It was found by reading the band `doc/todo/01` named.

**§11.7.4.4's read-and-kept sentence was wrong about its own evidence**: it named
`knockout_is_drawable`, a function this tree has not had for hundreds of sessions, where the gate
is `content::transparency::knockout_group_elements`. `--bin pointers` cannot see a bare identifier.

## What the next round takes

Rank 668's other six rows, unread. §11.7.4.4 is read and its departure stays: moving it is
§11.4.6's work, because §11.4.4's NOTE 3 is what lets a non-isolated knockout group be composited
onto transparency, and a blending element is where that cancellation stops holding.
