# 1228 — §10.8.3's simulation is a reader's request, so it is a preference and not a restriction level

Session 1195. Status: **accepted**.
Context: `crates/pdf-model/src/view.rs` (`ViewState::separation_simulation`,
`ViewState::set_separation_simulation`), `crates/viewer-core/src/command.rs`
(`Command::Separations`), `crates/viewer-core/src/viewer.rs`,
`crates/viewer-host/src/policy.rs` (`SEPARATIONS`, `separations`, `separations_note`),
`crates/viewer-host/src/keys.rs` (`WindowAct::Separations`),
`crates/viewer-confined/src/protocol.rs`, `crates/viewer-ffi/src/abi.rs` (`quorra_separations`),
the four windows, `doc/conformance/ledger.toml` §10.8.3.
Builds: ADR 1189 section 2 (a producer's graphics-state parameter is not one of the four levels),
ADR 1173 (`Purpose`, stated by an operation and never inferred), ADR 1106 (`Audience`, the same
shape), ADR 1076 (a host supplies the policy), ADR 1144 and ADR 1145 (the four levels and the menu
that sets them), `CLAUDE.md` principle 3.
Clauses: ISO 32000-2 §10.8.3, §10.8.1, §10.8.2, §12.11 (Table 275), §8.6.6.4, §8.6.6.5.

## 1. The clause states its own condition, and no file can meet it

§10.8.3 opens by saying who it is for:

> If it is important for the colours of the display for a PDF, on a device that normally would not
> be used to produce separations, to more closely match those produced when using separations,
> then a simulation of the separation process can be performed for the output to the
> non-separation device.

*Can be performed*, on a condition about what is **important** to somebody. §10.8.1 says whose
choice that is in as many words — "[w]hether separations are produced is up to the processing
software" — and §10.8.2 describes what the other answer looks like, which is what this tree has
always drawn: the alternate colour space and its tint transform evaluated, the overprint controls
ignored.

So the clause requires nothing of a processor that does not perform the simulation, and it makes
performing one conditional on an input no PDF holds. The ledger row had already found that the
*debt* is Table 275's `SeparationSimulation` requirement rather than this clause's, and that what
was missing was "the control: … a user's request rather than a document's, and this viewer has
none". This is that control.

## 2. Why it is a preference and not one of `CLAUDE.md`'s four levels

ADR 1189 section 2 drew this line for `/SA` and the same three sentences decide it here.
Principle 3's levels are for "the permissions a *document* asserts over the person reading it —
Table 22's `/P` flags, §12.8.2.2's `/DocMDP`, §12.8.6's usage rights". Nothing in any table asks
for a separation simulation; a reader asks for it, about their own document, and there is nobody to
ask on their behalf and nothing to warn them of afterwards. Four levels over two states would be a
vocabulary with two dead words.

**Table 275's `SeparationSimulation` is not a counter-example.** That entry says a document
*requires support for* this subclause, which is a statement about what a processor must be able to
do — and `requirements::unmet` names it when a document states one. It is not a document asking for
the simulation to be on now, which is why the requirement stays where it was and this value is the
reader's.

The value therefore joins `Trust`, `References`, `Audience` and `Clock` as the tenth host-supplied
policy value, with their rule and for their reason: it applies to every open document and to every
one opened afterwards, because it is a fact about the *reader* rather than about any one file. It
is `false` until a host says otherwise, under which every page draws exactly what it drew before
this value existed.

## 3. The channel, and why it is `ViewState`

`CLAUDE.md` rule 1 makes `pdf_syntax::Document` immutable and interpretation a pure function of the
bytes, the viewer state and what the user did. §10.8.3 decides what colour every mark on a page is,
so it decides a mark — and the only channel by which anything outside a file may is
`pdf_model::view::ViewState`, which is where `magnification`, `audience`, `purpose` and `paper`
already arrive. `ViewState::set_separation_simulation` is that channel and
`ViewState::separation_simulation` is where the colour route reads it; the setter answers whether
the answer moved, because a change supersedes the ink already produced and an unchanged answer is
not a reason to draw anything again.

**A `bool` and not a richer type**, deliberately: what a host knows is which of the two pictures a
person asked for, and what the simulation *is* — which process colourants the simulated device has,
what an output intent's `DestOutputProfile` or `ColorantTable` says about them — is §10.8.3 steps
a) to d) and belongs to the colour route that performs them. A host that had to describe a
simulated press would be answering a question about the document.

## 4. The surface: one word, one key, four windows and a C caller

`--separations=on|off` in all three windows, `WindowAct::Separations` on shifted `S` for a person
who is already looking at the page, `Command::Separations` across the confined wire as one bit, and
`quorra_separations` for a C caller. The state — which of the two a window is showing — is the
*host's*, because `viewer-core` holds no preference a person toggles and a window is what says
which one is on; what crosses the boundary is the answer.

`--separations=` is not `--restrictions=`'s or `--links=`'s spelling and takes `on` and `off`,
which is the `RESTRICTIONS` vocabulary rather than the `LINKS` one. That is not an inconsistency:
those two are scales with a permissive end and this is a switch, so the words that would carry a
direction are exactly the words it must not borrow.

## 5. What this does not close

The algorithm is not this ADR's. §10.8.3's four steps — separations, flat XYZ against a white
matte, a multiply blend, then the device's space — are `pdf-colour`'s and arrive with their own
number; this decides that they have an input, what kind of input it is, and where it comes from.
`content/xobject.rs`'s `enter_imported` carries `magnification` and `purpose` across into a
reference `XObject`'s own document and does not yet carry this one, which is a two-line change in a
file this round did not own: a page imported from another document should be drawn under the same
answer as the page importing it, for the reason the comment there already gives about the other
two.
