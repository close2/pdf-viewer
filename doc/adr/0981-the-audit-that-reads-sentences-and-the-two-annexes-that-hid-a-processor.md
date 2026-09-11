# 0981 — The audit that reads sentences, the two annexes that hid a processor obligation, and a blocker priced at the wrong thing

Session 970. Status: **accepted**. It takes `crates/pdf-archive/src/coverage.rs` from a
subclause-level instrument to a sentence-level one over a stated region, finds eight normative
sentences with no row — in four subclauses, two of them recorded as stating only scope — and
re-prices the one `Unchecked` reason that named a route this tree does not have.

## 1. The claim that was one step short

ADR 0972 built the structural audit and stated its own limit in its module comment:

> Sentence-level coverage cannot be committed to this tree at all: it would have to enumerate
> the standard's sentences, and `doc/questions/A16` licenses these two documents to a single
> reader.

**That reasoning conflates two different things, and the distinction is the whole of this
round.** What the licence forbids is reproducing the standard's *words*. A sentence-level audit
does not need them: it needs a count and a mapping — how many rules a subclause states, and which
row of the table carries each — and the text beside each entry can be this crate's own paraphrase,
written exactly the way a `Requirement::asks` already is and for the identical reason. The
requirement table has been paraphrasing these two standards, rule by rule, since it was written;
a second list of paraphrases is not a new licence question.

So the limit was real about *quotation* and wrong about *coverage*, and it had the shape ADR 0964
section 2 named one clause over: a conclusion that was right for a reason that did not apply.
`coverage.rs` now carries `Sentence`, `Carried` and `Reading` beside `Subclause` and `Binding`,
and `frontier()` computes what has not been read rather than a document claiming it.

**Why the frontier is computed and not written down.** `CLAUDE.md`'s rule — a fact that can be
counted is not written down, the command that counts it is — applies with force here, because the
frontier is the one number in this file that a later round is *supposed* to move. A sentence in a
module comment saying "clause 6.1 and the annexes have been read" goes stale the round after it is
written; `cargo run -p pdf-archive --example frontier` prints the subclauses with no reading, from
the data, every time.

## 2. The region read, and why that region

Clause 5, clause 6.1 and the normative annexes of **both** parts — the structural prefix, plus the
annexes. Two reasons for that shape rather than "the subclauses where the gap looked largest":

- **A prefix is reviewable.** A person with their copy open reads straight down the page and can
  see that nothing was skipped. A scattered set of subclauses chosen by a heuristic cannot be
  checked that way, and the heuristic that would have chosen them (sentences outnumbering rows) is
  not even sound — one row legitimately carries four sentences of ISO 19005-2 section 6.1.9, and
  ten rows carry eleven sentences of section 6.1.13.
- **The annexes are where the subclause-level pass had found its hole**, so they are where a
  second pass is most likely to find the rest of it. It did; section 3 is what it found.

`coverage.rs`'s `the_region_this_audit_has_read_sentence_by_sentence_stays_read` names six
anchors inside that region and fails if any loses its reading. It deliberately does **not** assert
that anything outside the region is unread: a test that had to be edited to let a later round read
further would make progress cost a failure that looks like a regression.

## 3. What the sentence-level pass found

### 3.1 Two annexes audited as scoping, each hiding a processor obligation

ISO 19005-4 Annex A.1 and Annex B.1 each state, in four sentences, what a PDF/A-4f (respectively
PDF/A-4e) conforming *file, writer or processor* is. The subclause-level audit recorded both as
`Binding::Scoping` — "PDF/A-4f is clauses 5 and 6 as this annex modifies them, which is what
`Target::Four(Flavour::F)` and the `Applies::Flavours` column are" — which is true of the first
sentence and of nothing else in the subclause. The third and fourth sentences require a conforming
processor of that flavour to read and appropriately process every file of that flavour **and
everything a plain PDF/A-4 conforming processor is required to read**. No row reached either.

**This is the general lesson and it is larger than the instance.** `Binding::StatesNoRequirement`,
`Binding::Scoping` and `Binding::Restated` are claims about *every* sentence of a subclause, made
at a granularity that cannot see a sentence. They are exactly as capable of hiding a requirement as
`Binding::Bound` is — more so, because a `Bound` subclause at least has a row somebody wrote while
reading it. So `frontier()` puts every non-container subclause in the frontier, whatever its
subclause-level verdict, rather than only the `Bound` ones. A round that restricted the frontier to
`Bound` would have declared these two annexes finished.

### 3.2 Three sentences of clause 5 that no row carried

- **ISO 19005-2 section 5.5 and ISO 19005-4 section 5.2 each require a conforming processor to read
  and appropriately process every conforming file of that part.** This is an obligation about a
  processor's *coverage* rather than about its behaviour on the files it happens to open — a
  program that declines a conforming file is not a conforming processor, whatever it does with the
  rest — and it is a different sentence from the two that `conformance/processor-behaviour` already
  carried. `conformance/processor-reads-every-conforming-file`.
- **ISO 19005-2 section 5.5 adds that a conforming PDF/A-2 reader shall also read and appropriately
  process every PDF/A-1 file.** It is the one sentence of either owned part that reaches outside
  them both, and it was on no list. `doc/questions/A17` settles that part 1 is not a *target* of
  this crate; that is a different question and does not discharge this one, because a claim to be a
  conforming PDF/A-2 reader is a claim about ISO 19005-1 as well.
  `conformance/a-part-two-reader-also-reads-part-one`.
- **Part 2 requires a conforming reader to ignore features the base standard does not describe;
  part 4 recommends it.** `conformance/processor-behaviour` folded that into its own `asks` for
  both parts, which overstated part 4 — ISO 19005-4 section 5.2 writes that sentence with *should*
  where ISO 19005-2 section 5.5 writes *shall*. It is now
  `conformance/undescribed-features-are-ignored`, binding part 2 alone, and `processor-behaviour`'s
  `asks` no longer claims it. Same shape as ADR 0964 section 2 again, and again invisible to every
  gate: no verdict moved, because both rows are `Check::Processor` and no document can fail either.

All five new rows are `Check::Processor`, which is a decision and not an economy — the argument is
ADR 0972 section 4's and it still holds: a new predicate is a new row of the converter's census
(`crates/pdf-transform/tests/archive_unconsidered.txt`, held to equality in both directions), so a
round that adds one owes a second crate a decision in the same commit. `Processor` rows never enter
that list.

### 3.3 What the pass found to be correctly carried, which is also a result

Of the 128 normative sentences read across 52 subclauses, 100 are carried by a row, 19 restate a
rule the table carries at another clause, 5 are scoping and 4 state no requirement at all. The
restatements are concentrated where one would expect them and where nobody had checked: the whole
of ISO 19005-2 Annex A.2 to A.5 is the transparency method whose single row sits at A.1, and
ISO 19005-4's Annex A.2, A.3, B.4 and B.5 send the reader back to section 6.9 and section 6.7.3.

Three sentence-level differences between the two parts were confirmed rather than assumed, and each
is a row the table had already got right: part 4 states no `stream`/`endstream` line-ending rule
(part 2 section 6.1.7.1 does), no `DocMDP` digest-key rule in its permissions subclause (part 2
section 6.1.12 does), and no sentence telling a processor to render an annotation from its
appearance alone (part 2 section 6.3.3 does).

## 4. A blocker priced at a field that is really a resolver

`conformance/no-deprecated-features` — ISO 19005-4 section 5.1, a conforming file shall not use a
feature ISO 32000-2 describes as deprecated — arrived in session 965 with a reason saying "the
route to it is in this tree: `pdf_spec`'s Arlington-derived model states `deprecated_in` for a key
and for a type". Read against the code, that is wrong in the way ADR 0964 section 4 warned about:
**a blocker priced at one field is a different decision from a blocker priced at a resolver.**

- The field is per *key of a named Arlington object*. `pdf_spec::object` takes Arlington's own name
  (`Catalog`, `AnnotSound`, `XObjectFormTrapNet`), and **nothing in this tree maps a document's
  dictionary to that name**. The only consumer that walks the model — `pdf-model`'s
  `integer_entry_census` — iterates the whole table rather than resolving a document's objects
  against it. Closing this row needs a type inference over the object model, which is a piece of
  infrastructure rather than a predicate. (`pdf-syntax` declares `pdf-spec` in its manifest and
  uses it nowhere, which is how the route looked closer than it is.)
- **And the population would be wrong even with one**, on the part's own evidence. ISO 32000-2
  marks `UR3` deprecated in PDF 2.0; ISO 19005-4 section 6.1.11 names `UR3` as one of the only two
  keys a permissions dictionary may hold. A mechanical per-key reading of section 5.1 makes that
  sentence forbid two keys and permit none. Whatever the clause's population is, it is not "every
  key Arlington marks deprecated".
- **The size of the per-key route, now measured** rather than left as "an amount nobody has
  measured": 419 of Arlington's 3983 key rows carry `DeprecatedIn`, 372 of them at 2.0. Most are
  whole objects PDF/A-4 forbids for other reasons already — `AnnotSound`, `AnnotMovie`,
  `AnnotTrapNetwork`, the OPI and Web Capture trees, `URTransformParameters`. What the per-key
  route would *miss* is what the row always said: ISO 32000-2 deprecates the standard security
  handler's revisions 2 to 4, the `adbe.x509.rsa_sha1` and `adbe.pkcs7.sha1` subfilters, SHA-1 as a
  digest algorithm, an array of blend mode names, and two character collections — none of which is
  a key.

The row stays `Unchecked` and its reason now says all of that. **A reason that names a route is a
claim about this tree, and it decays exactly the way a ledger row's does** — this is the third one
in four validator rounds (ADR 0956's three, ADR 0964's two, this).

## 5. The habit this leaves

**An instrument's stated limit is a claim, and a claim about what *cannot* be done is the most
expensive kind to leave unexamined.** ADR 0972's "sentence-level coverage cannot be committed to
this tree at all" was written by the round that had just built the best coverage instrument this
crate has, in the same comment that recorded its own frontier honestly — and it survived exactly
one round because somebody read it against the licence it cited rather than against the sentence
it had become. The generalisation, and it is the same one `CLAUDE.md` principle 5 makes about a
clause the standard is silent on:

> A sentence saying *this cannot be done* is evidence about the round that wrote it, not about the
> tree. Before inheriting one, ask what exactly it says is impossible, and check that the reason is
> about that thing and not about a neighbouring one.

The two failure modes are visible together here. `coverage.rs` said sentence-level coverage was
impossible, citing a licence that forbids something else; and `no-deprecated-features` said the
route was in the tree, citing a field whose use needs a resolver that is not. **A round that reads
both of a claim's halves — what it says, and what it cites for it — finds these; a round that reads
only the first inherits them.**

## 6. What did not change

- **`over` is 0 on all six targets**, before and after, with the one standing miss
  (`6-6-2-3-3-t03-fail-b`, errata A029) unchanged. Every new row is `Check::Processor`, so no
  document's verdict can move.
- **`crates/pdf-transform/tests/archive_unconsidered.txt` is untouched and still empty**, and
  `cargo nextest run -p pdf-transform` is 358 passed including
  `the_unconsidered_requirements_are_the_ones_this_file_names`.
- **No requirement identifier was removed or re-keyed.** Five were added, all `Processor`.
