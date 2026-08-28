# ADR 0719 — The stream a reference could name, and the appearance that dropped its name

Status: accepted, 2026-08-28. Session 782, a general-improvement round choosing its own subject.

An annotation's stored appearance stream is now a stream Table 357's `/Stm` can name, `/StmOwn` —
the last unread entry of that table — is read beside it, and the structural-parent walk asks an
annotation's appearance streams for Table 359's keys, which nothing did. §14.7.5.2's row keeps
its status and loses a limit; `doc/todo/31`'s "take the two together or not at all" is taken,
together.

## 1. Why this subject

The instruments named it twice, from two directions:

- `cargo run -p conformance --bin entries` printed §14.7.5.2 — an **implemented** row — with
  Table 357's `/StmOwn` "named nowhere at all, AND THE NOTE DOES NOT NAME IT", which is the
  sweep's own read-first shape: an entry unaccounted for under a row that claims completion.
- `doc/todo/31` carried the other half as a named remainder with its design already argued: an
  `/MCR` whose `/Stm` names an appearance stream "finds nothing where it used to find the wrong
  thing", what would close it is "the `/AP` reference carried through `Appearance`", and
  `/StmOwn` "is the same item from the structure side — take the two together or not at all".

No corpus or crawl document exercises it — `mcid_stream_census` over 65 944 crawled documents
found 545 stating a `/Stm` and none naming an appearance stream — so this is spec-driven
coverage, which is the side of `CLAUDE.md`'s two tracks a corpus can never reach. The batch's
siblings hold the errata ranking, the oracle's contradicted pages and the confined-boundary
host; this touches none of their lanes.

## 2. What the standard states

Table 357's `/Stm` row names the population in its own parenthesis:

> This entry should be present only if the marked-content sequence resides in a content stream
> other than the content stream for the page (see 8.10, "Form XObjects" and 12.5.5, "Appearance
> streams").

— so an appearance stream was always in this entry's population, beside the form `XObject`s that
ADR 0488 made nameable. `/StmOwn` is its companion:

> The indirect reference to the PDF object referencing the stream identified by the Stm key.

with the NOTE naming the use: "to identify the annotation dictionary owning the appearance
stream". And Table 359 puts the structural-parent key "in the page object of a page containing
marked-content sequences, in the stream dictionary of a form or image XObject, or in an
annotation dictionary" — an appearance stream **is** a form `XObject` (§12.5.5), so its
`/StructParents` was covered by words already quoted in §14.7.5.4's ledger row, and read by
nobody.

## 3. The decisions

**The reference is captured where it is resolved, and travels as `Appearance::source`.**
`annotation::stored_appearance` reads the `/AP` entry — and the state subdictionary's entry,
which is the other of §12.5.5's two forms — *before* resolving it, the same shape as
`Interpreter::draw_xobject`'s unresolved lookup and for the same reason: resolving first throws
away the one identity `/Stm` can match. `Interpreter::draw_appearance` then seats
`ContentStream::Object(id)` for the stream's run instead of `Unnameable`.

**`Unnameable` keeps exactly what no reference can reach**: a stream written directly into the
appearance dictionary, and a §12.7.4.3 construction, which is this program's bytes and has no
object at all. That is the variant's own doc comment honoured rather than amended.

**A §12.7.4.3 regeneration keeps the stream's identity.** The splice replaces one `/Tx` region
of the stored bytes and leaves every other sequence byte-identical (`appearance::spliced`), so
the sequences a marked-content reference can name are still the file's. A sequence inside the
rewritten region is gone either way — it was replaced by the field's current value, which is
what a screen reader should be given.

**`/StmOwn` is read into `Child::MarkedContent` and consumed in the channel §14.7.5.3 built.**
The entry is an ownership statement, so where it names one of the page's annotations it is the
same statement an object reference's `/Obj` makes, and `viewer_core::accessibility` pushes it
into the element's `objects` — the annotation's `/Rect` places the element, §12.7's control says
what it is, and §12.5.1's activation has an annotation to go to. It contributes nothing where it
names anything else (a resource dictionary, say), because the match is against the page's own
annotations; and it does not by itself put an element on the page — the sequence's own `/Pg` and
marks decide that, because `/StmOwn` states who owns the stream, not where the sequence is.

**The population walk asks appearance streams, which is the finding this round did not go
looking for.** The end-to-end test failed with the element pruned: `Tree::elements_on_page`
walks the page's `/StructParents`, its annotations' `/StructParent` and the `XObject`s the
page's *resources* name — and an `/AP` entry is not a resource, so an element whose only content
item is a sequence inside an appearance stream was reachable by no route. `Tree::appearance_owners`
walks Table 170's three entries, each "a single appearance stream or an appearance
subdictionary", asking each stream for both of Table 359's keys and following its resources.
This is the fifth route in a function whose fourth was itself a missed-route finding (ADR 0488),
which suggests the shape recurs: **a walk enumerated by where streams are usually found is
blind to a clause that puts them somewhere else.**

## 4. A narrowing that fell out

ADR 0488's recovery — a bare integer the page's own stream cannot answer is given the sequence
when **exactly one** other stream carries it — tested "one stream" by comparing `ContentStream`
values, and every appearance stream was the *same* value, `Unnameable`. Two widgets' appearances
each carrying `/MCID 0` therefore passed the one-stream condition together and were both
answered. Named apart, they answer nothing, which is what the condition always said. No corpus
document sits in that shape (the two known recovery witnesses put every sequence in one form);
the change is recorded because it is a behavioural edge a future diagnosis might meet.

## 5. What was measured, and the tests

- `entries` sweep before: §14.7.5.2 implemented with `/StmOwn` "named nowhere at all, AND THE
  NOTE DOES NOT NAME IT"; after: the hit is gone (the row's code names it, and the note says
  what is done with it).
- Three tests in `marked_content_scope.rs` (the fixture is a stamp whose appearance stream marks
  `/MCID 0` against the page's own `/MCID 0`): the span is recorded against
  `ContentStream::Object(appearance)`, `logical_text` answers each element from its own stream,
  and the `/MCR` arrives with `owner: Some(annotation)`. One in `headless.rs`: an element
  reaching its check box through `/Stm`+`/StmOwn` — no `/OBJR` anywhere — is placed by the
  widget's `/Rect`, says `CheckBox { on: true }`, names the annotation, and has the appearance
  sequence's own quads. Each was calibrated per trap 13 against the defect it exists for —
  the interpreter's seating forced back to `Unnameable`, the `/StmOwn` read and the consumer
  push each dropped, the population walk reverted — and failed under every plant, then passed
  restored.
- The fixture found one specification requirement on itself: Table 166 makes `/AS` *required*
  when `/AP` holds subdictionaries, and without it `stored_appearance` answers
  `StateNotDefined` and draws nothing — which is the reading, not a defect.

## 6. What this does not do

- No behaviour changes for any document that states no `/MCR` into an appearance stream and no
  colliding identifiers across appearance streams — the corpus gates are the evidence.
- `NoView`/hidden appearances: a sequence in an appearance the viewer does not draw still
  produces no marks, so such an element keeps its rectangle route (`/StmOwn`) and nothing else,
  which is what a picture nothing shows owes a screen reader's geometry.
- The `/StmOwn` of a stream that is *not* an appearance (a form named by `/StmOwn`'s "any"
  wording) is carried and matched the same way; nothing further is invented for it, because the
  standard states nothing further.
