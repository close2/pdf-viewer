# 1160 — What time it is is an input a host supplies, and Table 166's `/M` is written from it

Session 1161. Status: **accepted**. Revisits [ADR 0196](0196-an-annotation-a-person-added.md).

ADR 0196 wrote no `/M` on an annotation a person added, "because `CLAUDE.md`'s rule 3 gives
`viewer-core` no clock. A host with one may add it; inventing a timestamp from nothing would be
worse than omitting an optional entry." The reasoning was right and its premise expired: since ADR
1039 this tree has a settled shape for a fact a renderer may not read and a host can supply, and
four values already use it. This ADR makes the instant the fifth.

## 1. What the entry asks for, and what a `should` means here

ISO 32000-2 Table 166:

> The date and time when the annotation was most recently modified. The format should be a date
> string as described in 7.9.4, "Dates" but interactive PDF processors shall accept and display a
> string in any format.

Two things follow and both shape what is built.

**The entry is Optional**, so writing nothing is conforming and ADR 0196 was never in breach. What
it cost is not conformance but the file: every markup tool a person compares this program against
writes the entry, and a note whose date is missing sorts nowhere.

**The `should` is met rather than taken advantage of.** The second half of the sentence binds a
*reader* — "shall accept and display a string in any format" — and says nothing about what a writer
may invent. §7.9.4's string is the one form every reader can order, and `pdf_syntax::Date::instant`
is this tree's own reason to care: a date outside the grammar is a string nothing can compare.

## 2. The shape: the ninth host-supplied policy value

`viewer_core::Command::Clock(Option<pdf_syntax::Date>)` carries it, and it is the same shape as
`Trust` (ADR 1039, ADR 1076), `References` (ADR 1101) and `Audience` (ADR 1106): a fact about the
*reader's machine*, asked once in a place a host can answer, defaulting to nothing, applying to
every open document and to every one opened afterwards. `pdf_model::view::ViewState` holds it
beside the audience and the magnification, which is rule 1's only channel from outside a file into
what this program does with one.

**Two things this is not.**

It is not `Command::Tick`. That one says how much time has *passed*, in milliseconds, because
§12.4.4.1's `/Dur` needs a duration; this one says what time it *is*, because §7.9.4's date needs
an instant. Neither can be derived from the other, and ADR 0135's tick has never carried a wall
clock.

It is not a clock the core reads. **The instant last stated is the instant written**, and nothing
in the core makes it expire: a state machine with no clock cannot tell how stale a date it was
given is, and a value that decayed on its own would be exactly the invention rule 3 forbids. So the
host reads its clock at the moment the entry is about — `App::state_the_time` in the winit host
does it immediately before a save — and what it read is what the file says.

## 3. Where the entry lands

On every annotation dictionary a save writes, and the rule is Table 166's own: it is the
*annotation's* table.

- An annotation a person added (`write_additions`).
- A free text annotation of the file's own whose note a person retyped (`write_retypings`).
- The **widget** of a field a person typed into (`Update::stamp_annotation`) — and not the field,
  which §12.7.4.1 lets be an ancestor and which Table 166 says nothing about. Where the value goes
  on the widget itself the entry costs nothing; where it goes on an ancestor, or where only the
  appearance stream's bytes were replaced, the widget is written for the entry alone. That extra
  object is the price of putting the date where the table puts it, and it is paid only by a host
  that supplies an instant.

## 4. §14.3.4 is not engaged, and that is worth stating

`saving.rs`'s `saving_leaves_both_of_a_documents_metadata_sources_exactly_as_they_were` records a
deliberate decision not to write `/ModDate`, because §14.3.4's fourth rule — a `shall` — would then
require `xmp:ModifyDate` in the document's metadata stream to match it.

That rule is about "the document information dictionary and the document level metadata stream".
Table 166's `/M` is an annotation's entry and is neither, so writing it puts none of §14.3.4's four
rules into force and the decision that test records stands untouched. The document's own two
metadata sources are still never written.

## 5. The zone is UT, and that is a choice

`viewer_host::modification::now` states `Z00'00'`. The standard library offers no local offset, so
a host here can state UT, which it knows, or guess an offset, which would be a wrong claim in
somebody's file rather than a missing one. §7.9.4's grammar spells UT outright and the instant a
reader computes from it is the same instant either way. A host that knows its own zone can build
the `Date` itself and send it; nothing here prevents that.

## 6. What a host that says nothing gets

The bytes this program wrote before the command existed, byte for byte: no `/M` anywhere, on any
path. That is what `pdf-model`'s own tests, every corpus gate, the oracle and the save round-trip
walk see, because none of them supplies an instant — so this decision moves no measurement and no
page.
