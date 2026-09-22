# 1171 — The version is what the last revision reached, not what the last catalog says

Status: accepted. Session 1167.
Context: `crates/pdf-syntax/src/version.rs` (`Document::version`, `Document::stated_versions`,
`as_version`), `crates/pdf-syntax/src/xref.rs` (`walk_revisions`, `XrefTable::lay_over`),
`crates/pdf-syntax/src/document.rs` (`Document::as_of`),
`crates/pdf-syntax/tests/incremental_update.rs`.
Builds: ADRs 0206, 0180, 1043.
Clauses: ISO 32000-2 §7.5.6, §7.5.2, §7.7.2 (Table 29), §7.5.8.4, Annex I.
Errata: Errata Collection 3, Issue #399 (`/State` `Review` `Accepted`).

## 1. The clause has two sentences and the tree carried out one of them

Table 29 ranks the header against the catalog's `/Version`, and this reader has done that since
ADR 0206: the later of the two wins. §7.5.6 adds the other half, and Issue #399 rewrote it. The
amended sentence is not quoted here or in the code, because the text this project checks its
quotations against is the published clause and a collection of errata is not a corrected standard
(`doc/errata-read.md`). Paraphrased: an update's catalog `/Version` *upgrades* the version the
document conforms to, considering the header and any catalog entry already present together, and
the catalog of an incremental update shall not reduce that version — by the entry's value or by
its absence.

The consequence for a reader is one line of arithmetic and it is not the line that was here. A
document's version is the **maximum** over the header and every revision's catalog, because the
clause makes the sequence non-decreasing; taking the newest catalog alone answers whatever the
last update happened to say. Two files get the wrong answer that way and both in the same
direction, downwards:

- one whose second update states a lower `/Version` than its first — the case the ledger row had
  named since the four-hundred-and-eighteenth session and left unbuilt;
- one whose update rewrote the catalog and left the entry out altogether, which the old reading
  answered with the bare header. **This is the likelier of the two** and the row had not named it:
  it needs no producer to contradict itself, only one that rebuilt a dictionary from the entries
  it cared about. It is exactly what the amended sentence's *or absence* is for.

## 2. Why the objection that stopped this in the four-hundred-and-eighteenth session no longer bites

The row's recorded reason for not building it was cost: resolving a catalog per `/Prev` section is
"work on the open path for a number Annex I uses to *warn*". The premise was checked rather than
inherited, and it is false. Nothing in `Document::open` asks for the version. The one consumer
that produces Annex I's warning is `viewer_core::notes::about`, which `open.rs` holds behind a
`OnceLock` and documents as off the launch path by measurement; the others are `pdf-transform`
verbs, each asking once per file it is about to write. So the walk is paid by the caller that
wants the number, which is the shape `CLAUDE.md` section 2 asks for — not deferred work but work
that was never on that path.

What remains of the objection is that the walk must not be quadratic, and that is a real
constraint: `MAX_XREF_SECTIONS` admits a chain of 1024, and a table rebuilt per link would read
the whole chain 1024 times. So the chain is walked **once, forwards**. `xref::walk_revisions`
reads each section exactly as `xref::read` does and then lays them over an accumulating table
oldest first, which is §7.5.6's "most recent copy" rule stated as an overwrite instead of as the
first-writer-wins sort `XrefTable::fill` uses backwards. After the *n*th section the table is the
one a reader opening the file at the *n*th `%%EOF` would have built. A revision whose catalog
stands at the same place as the one before it is not parsed again, so the ordinary file — written
once, never updated — pays one section read and one catalog parse.

## 3. An earlier revision is read by substituting its table, and nothing else

`Document::as_of` builds a second `Document` over the same `FileBytes`, the same limits and the
same file encryption key, with an earlier table and empty caches. That is the whole of what a
revision is: §7.5.6 appends rather than rewrites, so every revision's bytes are still in the file
and only the table decides which copy of an object number a reader sees. The caches are empty
because they are keyed by object number and the point of the exercise is that the number now names
other bytes.

The key is carried rather than derived again, and that is the clause's own reasoning rather than a
shortcut: §7.6 makes the file encryption key the *file's*, from the `/Encrypt` dictionary and the
first half of `/ID`, and §7.5.6 requires the added trailer to restate both. A file that
re-encrypted itself in an update would defeat this — and would defeat every other reader too,
since the objects under the update would be ciphertext under a key no trailer still names.

This is a narrower instrument than ADR 1043's `FileBytes::prefix`, and deliberately so. A prefix
is a whole earlier *file*, which §12.8.2.2.2 needs because it is comparing what a signature
covered; the version question needs one dictionary, and finding each revision's `%%EOF` to cut a
prefix at would be work in service of a stronger object than the question asks for.

## 4. The writer's half is met by construction, and is worth stating because it could stop being

The same sentence binds a writer, and an incremental update is the only place this program could
break it: §7.5.2's header is inside the bytes "leaving its original contents intact" forbids an
append to touch, so an update's catalog is the only thing that can state a version at all. Every
catalog this tree writes into an update is the document's own catalog with one key changed —
`view.rs`'s usage-rights withdrawal, the name-tree holder, `pdf-transform`'s attach and detach —
so `/Version` travels with it and no revision reduces anything. A future writer that assembled a
catalog from named entries instead would break the rule silently, which is why
`an_update_states_a_version_and_a_later_one_does_not_take_it_away` writes a third revision that
drops the entry and holds the reader's answer at 2.0: the test states the rule from the outside,
where a change of construction cannot walk around it.

Nothing here obliges this writer to *raise* the version. §7.5.2 would — a processor writing a file
that conforms to this document identifies it as 2.0 — if an update carried a PDF 2.0 construct
into a 1.x file. None does: what these updates write is §7.11.4's embedded file streams, name
trees, annotations, field values and page-tree edits, and the newest of those is PDF 1.7. The
obligation arrives with the first 2.0 construct an update writes, and this is where the next round
to write one should look.
