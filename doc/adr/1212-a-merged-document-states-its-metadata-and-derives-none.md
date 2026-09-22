# 1212 — A merged document states its metadata and derives none

Status: accepted. Session 1187.
Context: `crates/pdf-transform/src/merge.rs` (`MergePlan::information`, `write_information`),
`crates/pdf-transform/src/update.rs` (`information_dictionary`, `validate`),
`crates/pdf-transform/src/bin/quorra-transform.rs` (`--info`), `crates/pdf-model/src/xmp.rs`
(`packet`), §14.3.3, §14.3.4, `doc/questions/A55`. Amends ADR 0821 section 9 and its second ground
only; sections 1–8 and 10 stand.

## 1. What expired, and what did not

ADR 0821 section 9 gave two grounds for the merged catalog carrying no `/Info`. The first holds and
is the reason this round did not change the default: §14.3.3's entries are claims about *the
document*, and the merged document was made by no source's producer at no source's creation time, so
carrying one source's would write a false claim into the file.

The second — *synthesising one would be authoring metadata this program does not author* — is gone.
`update::Edit::SetInformation` writes Table 349's nine keys through `pdf-vfs`'s verbs, and a merged
document's entries are no more authored than an edited document's: in both the operator states them
and the program writes what was stated.

## 2. The stated-entries path, and why the default did not move

`MergePlan::information` is a list of `update::InfoEntry`, validated by `update::validate` and built
by `update::information_dictionary` — the same two functions the in-place update uses, so Table
349's types are read once rather than twice. `quorra-transform merge --info Key=value` is the
program's half, repeatable, with a key outside the table or a `/Trapped` outside its three names
refused with both named.

**An empty list stays the default and writes no `/Info` at all**, which is what a merge did before
this existed. That is `doc/questions/A55`'s rule — a derivation is never a default — applied one
verb over from the converter it was answered about: the moment a merge derived a `/Producer` or a
`/CreationDate` from its inputs it would be asserting something no input asserts. The operator is
the only source, and a merge that is told nothing says nothing.

## 3. §14.3.4's first rule is in force here, and the packet is why

The clause states four rules for a writer. Two of them are `shall`s conditioned on *both* sources
being written, and the in-place update satisfies them vacuously by writing one and warning
(ADR 0855). A merge is a different position, and the clause names it:

> When writing the time and date of creation for the first time, typically when a new document is
> created, a PDF processor shall ensure that the data in the document information dictionary and the
> document level metadata stream -if both are written -are fully equivalent.

A merge *is* "a new document is created", and the fourth rule says the same of the modification
date. So where the stated entries carry a date, both sources are written and both name the same
instant: a `/Metadata` stream with Table 347's `/Type /Metadata` and `/Subtype /XML`, holding a
packet whose `xmp:CreateDate` and `xmp:ModifyDate` are §7.9.4's date spelled the way ISO 16684-1
spells one.

**"Fully equivalent" is about the instant, and the two texts spell an instant differently.** Every
field §7.9.4 leaves out has a default the clause itself states — the month and the day are 01 and
the rest are zero — so the instant is complete either way. The zone is the one field with no
default: an absent `O HH'mm` is a producer saying nothing rather than saying UT, so a date stating
no zone becomes an ISO 8601 date stating no zone.

**The packet carries three keys and not nine, and that is the clause's scope rather than a
shortcut.** §14.3.4's whole subject is the two dates; §14.3.3's deprecation sentence ("[i]n PDF 2.0
such use is deprecated except for two entries, CreationDate and ModDate") and Table 349's NOTEs name
an XMP counterpart for every key, and three of those counterparts are simple values in the XMP basic
schema — `xmp:CreatorTool`, `xmp:CreateDate`, `xmp:ModifyDate`. The other six are `dc:` and `pdf:`
properties whose `rdf:Alt` and `rdf:Seq` shapes `xmp::packet` does not write, and a NOTE is not a
`shall`. A round that teaches that writer the two container shapes can add the rest; nothing here
depends on their absence.

**ADR 0821 section 9's list is otherwise unchanged**: `/Metadata` is still not *carried* from any
source, and the warning still names every source that states one. What the merged document may now
have is a packet of its own, holding what the operator stated and nothing else.

## 4. §12.5.6.2's `/ExData`, checked rather than restated

The same round re-read Table 173 against §12.10, because "in scope and states nothing" is a claim
about the specification and those decay. Table 173's `/ExData` row gives the dictionary a required
`/Type` and a required `/Subtype`, and of the three subtype values `MarkupGeo` is the one clause 13
does not take: *This Subtype does not define any additional entries.* §12.10 names `MarkupGeo`
nowhere at all — its subclauses are the geospatial measure dictionary, the two coordinate system
dictionaries and the point data dictionary — so no clause outside Table 173 attaches an entry or a
reading to it, and a reader that has seen the `/Type` and the `/Subtype` has seen the whole of it.

**No reader was built, deliberately.** A `pub(crate)` reader with no consumer is dead code, and what
it would answer is a subtype nothing in this program acts on. What the row's note gained instead is
the check against §12.10 and the list of what actually keeps §12.5.6.2 `partial` — `/Subj`,
`/CreationDate` and `/DS` have no reader — so the debt is a list rather than an impression.
