# ADR 0065 — A third input

Status: accepted, 2026-07-31.

## Context

Every render this program has ever produced is a function of two things: a document and a page.
`interpret(document, page)` is that signature, and both gates rest on it — a page that renders
differently on two runs of the same file would make the oracle's comparison meaningless.

§12.6.4 breaks it. A set-OCG-state action (§12.6.4.13) turns a layer off; a hide action
(§12.6.4.11) sets an annotation's Hidden flag. Both decide what the *next* render of the same
page draws, and neither is written back to the file. The handover named these two as ready —
"three rows need only the *gesture*, their machinery being built already" — and it was right
about the machinery: §8.11's optional content answers "is this layer on" for every mark, and
§12.5.3's Hidden flag is honoured by `annotation.rs`. What was missing was somewhere to put
the change.

(The handover called the hide action §12.6.4.9. That is Sound actions; Hide is §12.6.4.11.)

## Decision

**A third input, named `ViewState`, and a second entry point that takes it.**

```rust
pub fn interpret(document: &Document, page: &Page) -> Interpretation {
    interpret_with(document, page, &ViewState::of(document))
}
pub fn interpret_with(document, page, state: &ViewState) -> Interpretation
```

`ViewState::of` is the state a document opens in: §8.11.4.5's initial configuration and nothing
hidden beyond what each annotation's own `/F` says. So `interpret` is `interpret_with` at the
opening state, the two cannot drift, and every existing caller — both gates, every other test,
the viewer's first frame — means exactly what it meant before. A test pins that equivalence.

### Why the state is not in the `Document`

`pdf_syntax::Document` is what the file says, and a layer somebody switched off is not what the
file says. Putting it there would make two renders of one page differ for reasons the file
cannot explain, which is precisely the property the oracle depends on. §8.11.4.5 draws the
same line itself: the initial state is "the state used by all PDF processors", and everything
after it belongs to one processor.

### `action.rs` reads twenty types and performs three

Table 201 lists twenty. Three change what a page displays and are performed; the other
seventeen become `Action::Refused` **carrying their own name and reason** — a file system the
sandbox withholds, a network this program does not have, clause 13, principle 5's exclusion of
scripting, or viewer behaviour not yet built. A name Table 201 does *not* hold produces no
action at all, because §12.6.4.1 says a processor "shall ignore" an unrecognised type and
calling it a refused action would claim to know what was refused.

`/Next` is where the reading is. NOTE 1 makes it a tree rather than a list, and states the one
robustness rule the clause gives — "self-referential actions ought not be executed more than
once" — which is a rule about broken files, so each action object is visited once and the total
is bounded. Flattening depth-first also fixes something that was quietly wrong: `link.rs` read
the outermost `/S` for its go-to, so a link that plays a sound and *then* jumps had no
destination. It reads the flattened list now.

### `/PreserveRB` made `/RBGroups` load-bearing

Table 217's `/PreserveRB` defaults to **true**, and what it preserves is Table 99's `/RBGroups`
— collections in which at most one group may be on. `/RBGroups` had been recorded as unread for
the project's whole life, correctly, because §8.11.4.5 gives it no part in the *initial* state.
It has a part in every change, and the default meant implementing the action without it would
have been wrong on every document that writes one. This is the shape the handover's own habit
names: a parameter whose default is the unimplemented behaviour is a gap wherever the feature
is used.

The collections live in `optional_content.rs` with the states they govern, and `apply` takes the
change list and the flag. §12.6.4.13's parsing stays in `action.rs`.

### §12.7.4.2 was `inapplicable` and is not

Table 214's `/T` may be "a text string giving the fully qualified field name of an interactive
form field whose associated widget annotation or annotations are to be affected". So a hide
action needs §12.7.4.2's names, and the ledger row for those said `inapplicable` — names
identify a field for export, scripting and the user interface, none of which decides a mark.

The reasoning was sound and the conclusion is now false: a field name decides whether an
annotation is drawn. `view.rs` builds the table from `/AcroForm /Fields`, appending each partial
name to its parent's, with the clause's own rule that a kid with no `/T` "shall not be considered
a field but simply a Widget annotation" — so one name reaches every widget of its field, which is
what Table 214's "annotation or annotations" means.

**An `inapplicable` row is a claim that can decay exactly as a `silent` one can**, and this is the
first time one has. The ledger's vocabulary note says `inapplicable` means "[n]othing is owed";
what it actually means is nothing is owed *by the clauses that reach it today*.

## Consequences

**Measured demand, and it is thin.** Walking every object of all 964 openable corpus documents:
`GoTo` 269 actions in 138 documents, `JavaScript` 234 in 55, `URI` 217 in 8, `GoToE` 31 in 2,
`Named` 9 in 3, `ResetForm` 3 in 2, `Rendition` 2 in 1, **`SetOCGState` 1 in 1, `Launch` 1 in 1,
and no `Hide` at all**; 32 documents carry an `/AA`. So this is spec-driven work with almost no
demand behind it, and saying so is what the two tracks are for. The largest unwritten action is
`URI` at 8 documents, and it wants a browser.

**Neither gate moves**, which is the expected result and was checked: 858 documents draw with
nothing reported, 832 pages agree, 65 contradicted. Nothing performs an action during either
gate, so every page is interpreted at its opening state.

**The ledger's `silent` count falls 188 → 181** and six clause-12 rows become `implemented`.

**The viewer performs them.** A click on a link runs its whole action list against the `App`'s
`ViewState` and redraws if the state moved, whether or not the page changed — so a link whose
only effect is turning a layer off now does something. That path cannot be run by the agent (no
X authority; see the handover's environment notes) and is the one part of this session verified
by construction rather than by execution.

**What is still owed for these two**: a layer panel, so that a person can do what the action does
without a document asking; §12.6.3's `/AA` trigger events, which would let an annotation hide
itself on rollover as §12.6.4.11's NOTE describes; and the fifteen other refused types.
