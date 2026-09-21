# 1159 — Table 224's flag names nobody, so the save says which field it is for

Session 1161. Status: **accepted**. Revisits [ADR 0121](0121-the-one-kind-of-writing-this-project-does.md).

The owner's revisit note on ADR 0121 asks one question: does a field's save write §12.7.4.3's
constructed appearance stream, or does it write Table 224's `/NeedAppearances` and leave the work
to the next reader? It argues for the first on the grounds that the exclusion sentence 0121 rested
on has been amended away and that free text and user-added annotations already save constructed
streams.

**The decision is already made and the tree already does it.** `Update::write_appearance` has
written §12.7.4.3's stream into the file since ADR 0121's second cost was closed; §12.7.4.3's
ledger row records it; the flag has gone in only "for the widgets whose stream this program could
not produce or could produce only part of" ever since. What the note read was ADR 0121 itself,
which is a record and is never edited — and a `save` doc comment that had gone on stating the
retired reason as the current one. So this ADR says which of the note's claims held, deletes the
retired sentence, and closes the one thing that was genuinely missing.

## 1. What the note claimed, against the tree

| the note says | and |
|---|---|
| a filled field is saved with `/NeedAppearances` rather than the constructed appearance | **does not hold.** `view.rs`'s `save` calls `Update::write_appearance` for every edited widget, and `appearance::for_saving` builds the stream by the same layout that draws it |
| the exclusion sentence 0121 rested on is amended away | **holds**, and `CLAUDE.md` now sanctions variable-text field appearances by name |
| free text retyping and user-added annotations save constructed streams | **holds** (ADRs 0196, 0304) |
| `view.rs` puts `/NeedAppearances true` into the update whenever it is needed | **holds, and the word is load-bearing** — *needed* is the residue below |

## 2. What the clause owes a writer that changes `/V`, read again

§12.7.4.3 states the operation and where its result goes:

> The new appearance stream becomes the normal appearance ( N ) in the appearance dictionary
> associated with the field's widget annotation

and the update form, for a stream that already exists:

> To update an existing appearance stream to reflect a new field value, the interactive PDF
> processor shall first copy any needed resources from the document's DR dictionary … The
> interactive PDF processor shall then replace the existing contents of the appearance stream from
> / Tx BMC to the matching EMC with the corresponding new contents

Table 224's own row then says what the flag is for, and it is a residue rather than an alternative:

> A PDF writer shall include this key, with a value of true , if it has not provided appearance
> streams for all visible widget annotations present in the document.

with a NOTE that settles the direction — "Appearance streams are required in PDF 2.0 and later" —
and the entry itself marked deprecated in PDF 2.0. So a writer that *has* provided the streams owes
no flag, and a writer that has not owes one. That is exactly the rule the tree implements, and the
clause is the reason rather than the exclusion sentence either ADR argued from.

## 3. What was actually missing: the flag names nobody

A file this program writes with `/NeedAppearances true` has something outstanding in it — a widget
whose value is in the file and whose appearance is not, or is only partly. The entry is a boolean:
it cannot say *which*, and nothing else said it either. `Update` held the condition as a `bool`.

That is trap 5's shape on the write path, beside two reports that already exist on it:
`Written::withheld` names every password field whose value was not stored, and
`Written::unappeared` every free text annotation written without a stream.

So `Update` holds the widgets rather than a flag, and `Written::unconstructed` names their fields
by §12.7.4.2's qualified name. `viewer_core`'s save reports each one the way it reports the other
two. The condition the report fires on is the entry's own (trap 11): *this writer did not provide
the appearance stream for this widget* — not a guess about whether a reader will mind.

**Named by field rather than by widget**, because that is what a person typed into: §12.7.4.1
spreads one value over a field's widgets, and two sentences about one name would be one fact said
twice.

## 4. The cost, and where it lands

The population is the one `doc/todo/22` names: a value whose characters no font in reach can spell
declines whole rather than drawing part of a line, and a `/DA` whose `Tm` turns the line off both of
the box's axes has no box to measure. Those saves now produce a sentence each. A save where every
changed widget got its whole stream produces none and writes no flag, which is what it already did.

## 5. What is not decided here

Whether §12.7.4.3's refusals can be *narrowed* — the Arabic layout and the off-axis matrix — is
`doc/todo/22`'s and §12.7.4.3's row's, unchanged. This ADR is about what the file and the person
are told when they are reached, not about reaching fewer of them.
