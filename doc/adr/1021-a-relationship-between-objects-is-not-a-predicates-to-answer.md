# 1021 — A relationship between objects is not a predicate's to answer

Session 1001. Status: **accepted**. The direction review's fifth next step
(`doc/reviews/984-direction-and-boundaries.md`, Finding 5 and Q2; ADR 1005 §5 is the proposal),
part one taken and part two put to the owner as `doc/questions/Q61`. Unblocks and closes
`doc/todo/62`, which ADR 0935 deferred and ADR 0941 measured. Adds
`crates/pdf-archive/src/reach.rs` and `crates/pdf-archive/tests/reach.rs`; changes
`examination.rs`, `finding.rs`, `lib.rs`, `clarification.rs`, `coverage.rs`, `editions.rs` and
`examples/unreferenced.rs` in the same crate. Prices the converter's identity case in §6 without
building it.

## 1. The shape the review found, in one sentence

`Examination` held five shared answers — `survey`, `objects`, `annotations`, `pages`, `spans` —
and every one is a **flat enumeration a predicate filters**. `objects()` is "every object a
cross-reference section names". Nothing answered *what reaches this object*, so every requirement
about a relationship between objects had to build its own graph, and `doc/todo/62` had been
blocked for three sessions on exactly that: ISO 19005-2 section 6.2.2 and ISO 19005-4 section
6.2.2 both exempt a named resource the associated content stream never references, and deciding
that an object is exempt means proving that *every* route to it passes through such an entry.

`Examination::reaches()` is the missing notion. One walk from the roots, computed once behind a
`OnceCell` like the other five, recording per object the entries it was reached through; and
`Examination::exempt()` beside it, which is that walk run twice — once following every edge and
once with the exemption's edges cut — so that the exempt population is a difference of two
reachable sets rather than an enumeration of paths.

## 2. What the walk's contract is, and what its bounds do

- **The roots are more than `/Root`.** §7.5.5 makes the trailer the entry point, so every
  reference the trailer states is a root — `/Root`, `/Info`, `/Encrypt` where §7.6 applies. Two
  further populations are named by the file's *structure* and by no reference: §7.5.7's object
  streams, whose containers the cross-reference table names as a location, and §7.5.8's
  cross-reference streams, found by `startxref` and the `/Prev` chain. A walk without them reports
  the file's own plumbing as unreferenced.
- **Annex F's linearisation parameter dictionary and hint stream are deliberately not roots**, and
  the measurement says that is most of what the unreferenced set holds: 764 of ISO 19005-2's 986
  corpus documents hold an object the trailer reaches not at all, and the ones inspected are a
  `/Linearized` dictionary and the hint stream its `/H` names by *offset*. `CLAUDE.md` leaves
  linearisation out until it is separately ratified, and reporting those two as unreferenced is
  the true answer to the question this module asks. The rest of that set is worth having on its
  own account: a stray `/Info` no trailer names, an orphaned integer, a `/Metadata` stream nothing
  references.
- **A path is the *entries*, plural.** An arrival records the object the edge left and the run of
  entries inside it that led there, so a page reached through a `/Pages` node's `/Kids` array
  arrives through `[Key("Kids"), Index(0)]` and not through "position 0 of something". The run is
  built in a per-object arena of `(entry, parent)` pairs rather than a `Vec` per traversal step,
  so an edge costs two slots and no allocation until an arrival is recorded.
- **One route rather than every route.** The breadth-first order decides which — the fewest
  objects, and among equals the one the walk met first. A question about *all* the routes is a
  different walk, and cutting edges and walking again is how this crate asks it.
- **The bounds refuse rather than truncate**, and that is the load-bearing decision of the module.
  `EDGES` is 8 000 000 references examined; `DEPTH` is 256, which is `pdf_syntax::Limits::max_depth`
  and therefore a bound the parser reaches first for anything this reader parsed — stated rather
  than assumed, so that a round raising the parser's limit gets a named refusal instead of a
  silent one. When either stops the walk, `unreferenced` and the exempt set are **empty** and
  `stopped_at()` names the limit: an exemption read off a walk that did not finish is a failure
  withdrawn in silence, which `doc/todo/62` §3 names as the one direction a validator may not move
  by accident.
- **Identity is the object number, with the generation dropped.** Not a reading of §7.3.10 — it is
  agreement with the program, because `pdf_syntax::Document::get` caches and resolves on the number
  alone and `Examination::objects` names every object at generation zero for that reason. It
  changes no count over `doc/veraPDF-corpus`; it is there for the file that spells a generation.

## 3. The example was the specification, and it now has none of the code

`crates/pdf-archive/examples/unreferenced.rs` carried the whole computation — a hand walk, a
fixpoint over indirect resources dictionaries, two reachability passes — because the crate had no
answer to lend it. It is now a printer: 212 lines instead of 481, with the machinery in `reach.rs`
where every row can reach it. **The check on the move was that the census reproduced exactly**,
and it did — 487/80/69/5/5 on PDF/A-4 and 986/81/74/7/7 on PDF/A-2b before the move and after it,
figure for figure, which is what made the next paragraph's *deliberate* change legible as one.

Two lines were added to what it prints, and both are the new method's rather than the exemption's:
how many documents hold an object the trailer reaches not at all, and how many stopped a walk at a
bound (zero, on all six targets).

## 4. What taking the exemption changed, and the one thing the move found

`doc/todo/62` §4 asked for four things and this round did all four. The carve-out is read **by
clause** in `reach::exemption_narrows` rather than by row identifier, because that is what the two
parts state: part 4 keeps its four object-syntax subclauses, sections 6.1.6 to 6.1.9, in its
published text, and part 2 keeps sections 6.1.2 to 6.1.13 under `TechNote 0010` A010. Section 5.1
is kept for both — A010's second sentence for part 2, and for part 4 the direction that withdraws
nothing, which costs nothing today because the row at 5.1 is `Unchecked`.

`crate::check` applies it by running a failing requirement's predicate a second time against a
`Findings` that drops what the exemption reaches. **A second run rather than a filter**, because a
report keeps the first 32 places and counts the rest, so a filter over a prefix cannot say what
the whole of it was.

**The move found an over-exemption that had been in the measurement since session 944, and it had
two halves.** The old `owners` map recorded *every object holding a `/Resources` key*, with an
empty set of referenced names where the object had no content:

- A `/Type /Pages` node holding the resources its descendants inherit was an owner, so it exempted
  every resource in a dictionary a hundred pages draw with. A003 says inheritance is not
  association, and the three owners it names are a stream, a page's `/Contents` and a Type 3
  font's glyph procedures; the test is now what the object *is*. **Measured**: PDF/A-4's entries
  80 → 75 and its exempt objects 69 → 68, PDF/A-2b's 81 → 80 and 74 → 73, PDF/A-2u's 4 → 0, with
  the candidate rows unmoved.
- A page or a Type 3 font with a `/Resources` entry and **no content at all** was an owner whose
  content references nothing. That is an *inference* — it draws nothing, so nothing is used — and
  not the sentence the standard states, which is about what *the associated content stream*
  references. With no stream there is no such fact, exactly as for an `/AcroForm` `/DR`, so the
  premise fails and nothing is exempt. **Measured**: no corpus document moves, because every
  veraPDF page has a content stream. Twelve synthetic fixtures in `crates/pdf-transform/tests`
  do — see §7.

Both corrections run the same way, which is the one that withdraws fewer failures.

**The corpus is unchanged, measured both ways.** With the narrowing switched off and on, all six
targets print the same six columns: `over` 0 everywhere, `missed` 1 on PDF/A-2b at section
6.6.2.3.3, which is pre-existing and about an XMP extension schema. That is ADR 0941's prediction
holding from the other side, and it is also why the corpus could not have ranked this work:
`CLAUDE.md`'s two denominators, the coverage question the robustness instrument is blind to.

## 5. The cost, and the two measurements that shaped the code

`CLAUDE.md` principle 2 asks for a number, and the first one was bad. A full PDF/A-4 report over
ISO 32000-2's own specification — 110 000 objects, 1023 pages — with the exemption wired in
naively took **23–26 s** against a 5–6 s baseline, producing a byte-identical report. Two things
were wrong and both are recorded at the code:

1. **The fixpoint.** Discovering which indirect objects are resources dictionaries and which are
   categories was a loop over every object of the file, repeated until the maps stopped growing.
   It is now two passes over the owners and over what the first pass found, and two is the whole
   answer rather than an approximation: `inner_spot` admits exactly two steps — a `/Resources` key
   under an owner, and a category key under one of those — and everything below a category's
   entries is an ordinary object again. 23–26 s → 8.8–11 s.
2. **The second predicate run.** The dearest failing requirement on that file reports a place per
   page over 1023 pages and names **no object at any of them**, so the exemption could not have
   withdrawn one of them and the re-run was pure waste. `Findings::named_an_object` counts, past
   the bound as well as inside it, whether any place named an object; `check` asks it before
   re-running. 8.8–11 s → 6.3–6.4 s.

Three runs each of the final code, on a machine with five rounds building: **6.06, 8.30, 11.46 s
without the narrowing and 5.43, 5.93, 9.31 s with it** — indistinguishable, which is the honest
report. The figures above are far outside that noise, which is what made them findings.

## 6. The converter's identity case, priced and not built

ADR 1006 put `if input.verdict() == Verdict::Conforms { copy_the_source }` in front of the
converter's three stages, and the review's Q2 says why it had to be a special case: `Rewrite::WholeFileRewritten`
is "the file", so the converter has no representation of *which objects change* and cannot check
its own rule — "nothing is changed that no failed requirement asked for" — except by re-validating
the output.

With `reaches()` the validator can now say, for a requirement's finding, **which object it is
about and how that object is reached**. That is the half of the seam this crate owns, and it is
done. What the converter's side would need, in the shape that fits what now exists:

- **A change set beside the output**, not a verdict in front of the input: `convert` returns the
  objects it replaced, added and deleted, keyed the way `Findings`' places are keyed. The identity
  case is then `changed.is_empty()` — a *fact about the run* rather than a bypass in front of it —
  and ADR 1006's special case comes out.
- **The rule becomes checkable rather than re-validated**: every changed object is one some kept
  finding's `Where::object` named, or is reachable only through one (which is `Reach`'s question,
  asked of the output). Re-validation stays as the net; what it stops being is the only instrument.
- **Price.** The change set is a field on the writer's existing walk — `rewrite.rs`'s `convert`
  already visits every object and calls `replace`, so recording what it replaced is bookkeeping
  rather than a second pass. The reachability check over the output is one `Reach` of a document
  already in memory, which §5's measurement puts well under a second on the largest file this
  project has. The work is a day in `pdf-transform`, whose crate is another round's this batch, and
  it is **not** started here: this ADR states the seam so that the round that takes it does not
  have to rediscover the shape.

## 7. Twelve fixtures in another crate, and why they are the finding rather than the casualty

`cargo nextest run --workspace` ends with twelve failures, all in `crates/pdf-transform/tests/archive.rs`,
all of the same shape and none of them a defect in this crate. That file's `Conforming` builder
writes a page whose `/Contents` is always object 4 and whose data is **empty** unless a test
states some, and twelve fixtures declare a resource — `/XObject << /Im0 6 0 R >>`,
`/ExtGState << /Gs0 6 0 R >>`, a `/Font`, a `/ColorSpace` — that the empty stream therefore never
names. Under section 6.2.2's last sentence those resources are not used for rendering, so the
requirement each fixture is about no longer fails on them, and the converter has nothing to decide.

**The validator is right and the fixtures are documents that do not exercise what they claim to.**
A test asserting that a converter fixes an image's `/Interpolate` needs a document that *draws the
image*; the one it has draws nothing, and until this round nothing could tell it so. The fix is
one line per fixture or one in the builder — a content stream that names the resource, `/Im0 Do`
inside a `q`/`Q` for an XObject, `/Gs0 gs` for a graphics state — and it belongs to
`crates/pdf-transform`, which is another round's this batch. Session 1001 did not touch it.

The twelve: `a_colour_specification_the_part_ignores_is_removed_only_with_authorisation`,
`a_file_marking_no_specification_best_keeps_the_one_a_jp2_reader_uses`,
`a_form_xobjects_opi_and_postscript_passthrough_are_removed`,
`a_postscript_xobject_is_dropped_with_the_resource_entry_that_named_it`,
`a_requirement_this_converter_cannot_meet_refuses_by_name_and_writes_nothing`,
`a_spot_colourant_the_file_already_defines_gains_the_producers_own_entry`,
`a_spot_colourant_the_file_never_defines_stays_refused`,
`a_symbolic_truetype_fonts_encoding_is_removed_where_no_shown_code_moves`,
`an_images_alternates_and_opi_are_removed`, `image_interpolation_is_a_loss_and_needs_authorising`,
`one_specification_with_a_method_the_part_forbids_stays_refused`,
`two_specifications_marked_best_stay_refused`.

**The converter over the real corpus is unmoved**: `tools/state.sh archive`'s second section
prints the same six lines it did before, conforming in and conforming out on all six targets. The
exposure is to synthetic fixtures, which is where a document with a resource nothing draws is
easy to write by accident.

## 8. What this ADR does not decide

The observer question. `crates/pdf-archive/src/survey.rs` is 3 715 lines re-deriving the
interpreter's resource-in-force and `q`/`Q` beside `pdf_model`'s `run_reader`, against
`pdf-render`'s own principle that a state machine exists once or the two disagree. ADR 1005 §5
prices it and calls it a design question for the owner rather than a task; it is
`doc/questions/Q61`, with what the tree does meanwhile written down there.
