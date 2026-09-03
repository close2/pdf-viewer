# A document's restrictions are the reader's to set, and they have levels

Status: **the reading, the four levels and the verdict are one module (ADR 0212, session 373;
ADR 0803, session 872); what is left is the event a window sends for *ask* and *warn* and the
command that answers it.** No user interface is to be built until the project owner asks for one.
Priority: 38 — capability, and low priority by the owner's own words
Clauses: §7.6.4.2 (Table 22's `/P`), §12.8.2.2 (`/DocMDP`), §12.8.6 and Table 258 (usage rights),
§12.7.5.5 (Table 236's signature field lock — the one restriction addressed to a *named field*
rather than to the document, ADR 0284), §12.7.6.2
Code: `crates/pdf-model/src/restriction.rs`, `crates/viewer-core/src/viewer.rs`,
`crates/viewer-core/src/notes.rs`, `crates/pdf-syntax/src/crypt.rs`,
`crates/pdf-transform/src/lib.rs`

## The policy, in the owner's words

> DRM restrictions are low priority and we should always have the possibility to turn them off. We
> should not implement a UI for them right now, but our finishing product will have a few different
> DRM levels: off, on, ask before operations, warn before operation. I tell this, so that, when we
> encounter them for any reason, they are now implemented in a way, which allows such levels later.

**This is not about the sandbox.** Principle 3's confinement runs the other way — it protects the
reader from the document — and nothing here is negotiable in that direction.

## What the three-hundred-and-seventy-third session built

| clause | what it does now | where |
|---|---|---|
| §12.8.2.2's `/DocMDP` | states which of Table 257's levels withholds which operation | `restriction::asserted` |
| §7.6.4.2's Table 22 | **consulted at last**, for bit 6 and bit 9, with `/R` deciding bit 9 | `restriction::withheld` |

**The population that reaches `withheld` grew in the eight-hundred-and-eighty-seventh session**, which is worth a line here because this file is about what a *document* asserts over a reader: `/R` 5 was refused by `pdf-syntax` and its Table 22 flags therefore reached nothing at all. 33 of the 41 `/R` 5 documents among the 90 535 in `doc/pdf.js`, `doc/corpora/` and `corpus-cache/` now open, 19 of them withholding at least one of the two operations this program has — and their flags arrive through §7.6.4.4.9's encrypted `/Perms` block, which outranks the plaintext `/P`, so what is obeyed for them is the copy a producer could not silently edit. ADR 0820.
| §12.8.6's `/Perms` | composes the two, because the clause says a permission needs *each* handler | `restriction::asserted` |
| §12.8.2.3's `/UR3` | unchanged, and deliberately: a grant is not a restriction | `view::withdrawn_usage_rights` |

The reading is in `pdf-model` and decides nothing; the policy is one value a host supplies
(`Command::Restrict(RestrictionLevel)`), asked **once per `Edit`**; the refusal leaves as
`Event::Refused { document, operation, notes }`, which carries the operation precisely so that it
can become a question. The argument, the corpus measurement and the Table 22 revision finding are
in ADR 0212.

**All three windows supply the value since the seven-hundred-and-twenty-first session, and this
paragraph said `viewer-ui` did and stopped there** (ADR 0604). It was true and it was half the
sentence: `pdf-viewer-gtk` and `pdf-viewer-qt` sent `Command::Restrict` nowhere and could not be in
any level but `On` — while both answered every refusal with a sentence naming
`--ignore-restrictions`, and both argument parsers answered that word with *"is not an option this
program has"* and exit 1. `CLAUDE.md`'s "**it shall always be possible to turn them off**" therefore
held in one program of three, and the sentence promising otherwise was the thing that made it look
closed. `viewer_host::IGNORE_RESTRICTIONS` is the word now and `viewer_host::refused` the sentence
that names it, in one module so that they cannot drift apart again; three tests hold the sentence,
each parser and the whole chain, because the defect lived exactly between two links that each had
one.

## What the eight-hundred-and-seventy-second session built

`pdf_model::restriction::Level` is the four levels; `Level::verdict` is the policy applied, a
pure function to an exhaustive `Verdict` (`Proceed`, `Warn`, `Ask`, `Refuse`, each carrying
every reason); `decide` is the whole question in one call. Every Table 22 position is named in
`Bit`, two of them as consumed by nothing, and `Operation` has the transform's three arms beside
the viewer's two, so §12.8.2.2's certification is read against a page render, an extraction and
a file written in, not only against the viewer's edits. `pdf-transform` consumes all four levels
— a pipe answers *ask* with `Refusal::Unanswered`, and its command line refuses the word before
opening the file — and `viewer-core` supplies `Off` and `On` through `RestrictionLevel::level`
and matches every verdict, the two it cannot produce in one arm that refuses visibly. ADR 0803.

## What is left

- **The event and the command, in `viewer-core` and its windows.** *Ask* is `Verdict::Ask`
  reaching a host as an event carrying the operation and the reasons, the edit held, and a
  `Command` that answers it — the `Event::PasswordRequired` shape; *warn* is the edit applied and
  an event saying what the document asserted. `RestrictionLevel` grows its two variants the same
  day, and `Viewer::refusal`'s one arm becomes three. They are not shipped because a variant
  nothing produces and nothing answers is a level that silently behaves like another one.
  Nothing here is `#[non_exhaustive]`, so adding them fails every consumer's compile until it says
  what it does, which is what makes waiting safe rather than lazy.
- **A user interface**, when the owner asks for one. A menu with four entries and, probably, the
  per-document override the viewer-wide value does not express today. **A command line is not one**,
  which is worth stating because it is what kept two hosts without any way out for the whole of
  their lives: nothing in the owner's instruction was blocking the flag, and nobody checked.
- **Table 22's bit 5, and the copy operation nothing here can name.** The bit is "[c]opy or
  otherwise extract text and graphics from the document", and this crate hands a host a *readback*
  — the same `Query::Selection` that a drag asks sixty times a second in order to draw a
  highlight. Refusing that would refuse the highlight. The bit also carves itself: "for the limited
  purpose of providing this content to assistive technology, a PDF reader should behave as if this
  bit was set to 1", so §14.9's tree must never be gated by it. What is needed is a host saying
  *this is a copy* — plausibly `Query::LogicalSelection`, whose own doc comment already says "[a]
  host asks this when a person presses copy", made to answer differently under the policy. It
  wants an `Answer` variant rather than `Answer::None`, because a copy that came back empty would
  be a lie about the selection.
- **Annex O's `ef`, which is the same four levels arriving from `doc/todo/39`.** "[S]ecurity should
  be strongly considered when opening an embedded file … a PDF processor may choose to prompt the
  user or even prevent opening of the file" — a *prompt*, which is exactly the ask level, over an
  operation (`Command::Extract`) that no document restricts today. It is the second consumer the
  levels are waiting for.
- **Assembling and faithful printing** (Table 22 bits 11 and 12) are named in `restriction::Bit`
  and consumed by nothing, each saying why; bit 3 is consumed since session 872, by
  `pdf-transform`'s page render. `Operation` gets an arm for 11 the day `split`, `merge` or
  `pages` exist (`doc/todo/57`), and for 12 only if this tree chooses the "implementation-
  dependent algorithm" the row leaves to the processor.

## What not to do

- **No user interface**, by the owner's instruction, until it is asked for.
- **No level enum shipped with one caller**, which is why two of four are absent rather than
  stubbed. ADR 0178's lesson.
- **No weakening of what is written.** §7.5.6's incremental update, `Document`'s immutability and
  the signature-withdrawal rule are correctness, not policy: a save that exceeds a usage-rights
  grant must still remove the `/UR3`, because §12.8.6 makes the signature a claim about the file
  and leaving it would make the file lie. Turning the *restriction* off is the reader's; making
  the file assert something untrue is not. The types keep the two apart —
  `RestrictionLevel` reaches `Viewer::refusal` and nothing else, and `withdrawn_usage_rights` is
  reached from `ViewState::save` with no policy in scope at all — and it must stay that way.
- **§12.7.6.2's submit is still not one of these**, re-checked in the three-hundred-and-seventy-third
  session: it is refused because it needs a network this program does not have (principle 3), which
  is a capability rather than a permission, and no level would turn it on.
