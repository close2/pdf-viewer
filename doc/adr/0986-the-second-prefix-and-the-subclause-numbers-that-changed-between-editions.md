# 0986 — The second prefix, the processor obligation two parts state and no row carried, and eight subclause numbers that mean something else in the edition PDF/A-2 adheres to

Session 975. Status: **accepted**. It takes the sentence-level audit of `crates/pdf-archive` from
the structural prefix into the whole of clause 6.2 of both parts — graphics, colour, images,
XObjects, transparency and every font subclause — finds **two normative sentences that both parts
state and no row carried**, splits one row whose two halves had different blockers, and continues
session 966's base-standard sweep into this crate's own citations, where it finds eight clause
numbers that resolve in ISO 32000-1:2008 to a *different subclause*.

Context: `crates/pdf-archive/src/coverage.rs`, `src/survey.rs`, `src/table/{fonts,graphics,
interaction,file_structure}.rs`; ISO 19005-2 6.2 and Annex B.1, ISO 19005-4 6.2; ISO 32000-1:2008
7.2.2, 9.6.4 to 9.6.6.4, 10.5.5.6, 12.6.4.16, 12.7.4.2.3, 14.7.4.4; ADRs 0972, 0973, 0981.

## 1. The region, and why clause 6.2 was the right next one

ADR 0981 gave the audit a frontier and a shape: the region read sentence by sentence is a
**prefix of each part in that part's own order**, plus the normative annexes, so that a reviewer
with their copy open reads straight down the page and can see that nothing was skipped. A
heuristic-chosen set of subclauses cannot be checked that way.

Clause 6.2 is what comes next in both parts, and it is also where most of the rows are:
`table/graphics.rs` is the requirement table's largest tranche by some way, and `table/fonts.rs`
is beside it. It is fifty-nine subclauses across the two parts. The frontier fell from 124 unread subclauses to 65, and the
figures are `cargo run -q -p pdf-archive --example frontier`'s rather than this file's —
`CLAUDE.md`'s rule, and with force here, because the frontier is the one number a later round is
*supposed* to move.

## 2. Two sentences both parts state that no row carried

Both were found the way ADR 0981's five were: by counting a subclause's sentences and asking
which row carries each, rather than by asking whether the subclause has rows at all. Both
subclauses were `Binding::Bound` and had been since the audit was written.

### 2.1 The output intent is the default blending colour space

ISO 19005-2 section 6.2.10 and ISO 19005-4 section 6.2.9 open their transparency rules with a
sentence addressed to the *processor*: the PDF/A output intent in force is the default blending
colour space. The sentence after it is the one the table had — where the document states no such
output intent, a page containing transparency must supply a `CS` of its own — and the two are
easy to read as one rule because they are one paragraph and one subject. They are not one rule.
The second is a condition on a **file** and is checked; the first is what a **processor** blends
with and no document can fail it.

`graphics/output-intent-is-the-default-blending-space`, `Check::Processor`, both parts. The two
parts differ in it, which is the reason the row's own sentence says "in force" rather than naming
a place: part 2 says the *document's* output intent, and part 4 the *current* one, which its
page-level `OutputIntents` array can change from page to page.

### 2.2 A processor renders with the embedded font programs

ISO 19005-2 section 6.2.11.4.1 and ISO 19005-4 section 6.2.10.4.1 close the embedding subclause
with the sentence that says what the four before it are *for*: a conforming reader renders with
the embedded fonts rather than with a locally resident, substituted or simulated face. Three rows
carried the file's half — the program is embedded, it is lawfully embeddable, it defines every
glyph shown — and nothing carried this one.

`fonts/embedded-programs-are-what-a-processor-renders`, `Check::Processor`, both parts. It is the
sharpest of the processor obligations this crate has recorded so far *for this project*, because
it is a requirement on the thing this repository is: a file that carries every program it uses
proves nothing if the program reading it reaches for a system face instead. `doc/PLAN.md` section
5a's conformance ledger is where a claim about our own font loading belongs.

Both rows are `Check::Processor`, so neither can move `over` and neither enters
`pdf_transform::archive::unconsidered()` — ADR 0972 section 4's argument, unchanged.

## 3. The two signature rows were **not** one predicate away, and the reason is a reading

ADRs 0972 and 0981 both recorded `signatures/digest-covers-the-whole-file` and
`signatures/signature-is-a-single-signer-cms-object` as closable, held back only by the
converter's census row. Read against the annex rather than against the tree, that is wrong for
both, and differently for each.

- **The digest row's blocker is a reading, and nobody had priced it.**
  `pdf_model::signature::Signature::coverage` does answer the question, and reports
  `Coverage::WholeFile` for exactly the `/ByteRange` ISO 19005-2 Annex B.1 describes. But the
  annex's sentence is about *the moment of signing*, and in a file that carries an incremental
  update after a signature every signature but the newest gives `Coverage::Unsigned` instead. So
  either the annex forbids a conforming PDF/A-2 file from carrying an update after a signature —
  which is what its own NOTE 2 says the rule ensures — or it does not; and a predicate written
  before that is settled fails documents on *this crate's reading* rather than on the file, which
  is precisely the way ADR 0972 section 4 said a new predicate can move `over`. The base standard
  answers the opposite way and `pdf_model::signature::Signature::must_cover_whole_file` already
  records it: §12.8.1's "should", made a `shall` for two sub-filters only, with a short range read
  as a later revision rather than as a defective signature. The row's reason now says all of this.

- **The single-signer row was two requirements in one, with different blockers.** ISO 19005-2
  Annex B.1 states, as separate sentences, that the signature value is a DER-encoded PKCS#7 object
  placed in `/Contents`, that the object conforms to **RFC 2315**, and that it carries at least the
  signer's certificate and exactly one signer. The first and third are countable from
  `pdf_model::cms::SignedData`; the second is not, because the annex names a narrower object than
  the RFC 5652 `SignedData` that reader accepts and nothing in this tree holds RFC 2315. Folding
  them into one row made the countable half hostage to the other. So the RFC 2315 sentence is now
  `signatures/signature-object-conforms-to-pkcs7`, `Check::Unchecked`, and the original row's
  reason is what is actually left owed: a predicate, plus the converter's census row.

**The general shape, and it is the third variant of a habit this stream keeps meeting.** ADR 0981
said a reason that names a *route* is a claim about this tree and decays. This round adds: a
reason that says a row is *one predicate away* is a claim about the **clause** as well, and the
clause half is the one nobody re-reads — because the route is in the tree, where a grep finds it,
and the reading is in a document licensed to one reader. Both of these rows had a correct route and
an unexamined sentence.

## 4. The base-standard sweep, continued into this crate's own citations

ADR 0973 section 3 walked the *converter's* decision table for justifications citing ISO 32000-2
at a target whose base standard is ISO 32000-1:2008, and closed by naming "the same base-standard
walk over `pdf-archive`'s own table" as owed. This is that walk, made mechanical: every `§` in
`crates/pdf-archive` was resolved in both editions' heading lists and flagged where the *titles*
disagree.

**Eight numbers on rows that bind a PDF/A-2 target name a different subclause in the edition that
target adheres to**, and the failure mode is worse than a dangling pointer: the wrong number
*resolves*, to a real subclause about something else, so a reader checking the claim is sent
somewhere plausible.

| ISO 32000-2 | ISO 32000-1:2008 says | what ISO 32000-1:2008's own number is |
|---|---|---|
| §7.2.3 character set | 7.2.2 | *Comments* |
| §9.6.4 Type 3 fonts | 9.6.5 | *Font subsets* |
| §9.6.5 character encoding | 9.6.6 | *Type 3 fonts* |
| §9.6.5.1, §9.6.5.2, §9.6.5.4 | 9.6.6.1, 9.6.6.2, 9.6.6.4 | — (no such subclause) |
| §10.6.5 halftone dictionaries | 10.5.5 | *Automatic stroke adjustment* |
| §10.6.5.6 type 5 halftones | 10.5.5.6 | — (its table is 134 there, 132 here) |
| §12.6.4.17 ECMAScript actions | 12.6.4.16 | *JavaScript actions* |
| §12.7.5.2.3 check boxes | 12.7.4.2.3 | — |
| §14.7.5.4 finding structure elements | 14.7.4.4 | *User properties* |

Clause 9.6's whole shift has one cause worth knowing rather than memorising: **ISO 32000-2 removed
*Font subsets* as a subclause of its own**, and everything after it moved up one. That is why
§9.6.4 is the trap in the table — it is a real subclause in both editions and a different one in
each.

**Three findings were errors rather than edition slips, and they are the ones that would have
misled a reader of either edition:**

- `crate::survey`'s Type 3 walk cited **§9.6.5** for "a Type 3 font's glyphs are content streams of
  their own", which is ISO 32000-1's number for that subclause carrying an ISO 32000-2 label, and
  **§9.6.5.4** for a Type 3 font's own `/Resources`, which is *Encodings for TrueType fonts* in the
  edition the `§` names and nothing at all in the other. Both are §9.6.4.
- `fonts/to-unicode-present` binds a **PDF/A-2 target alone** and its reasoning cited ISO 32000-2
  §9.6.4 for what a Type 3 font's `/Encoding` must state — the only edition that row never sees.
- A test comment cited **§6.3.2** for ISO 19005's annotation-flags exemption. `§` means ISO
  32000-2 (`doc/habits.md`), whose 6.3.2 is *Conformance of PDF processors*.

**Where the correction went is a decision, not an oversight.** Each `§` in this crate is *correct
as an ISO 32000-2 number*, so nothing is renamed. What was missing is the second number, and
repeating it at thirty-odd sites would be thirty copies of one fact. So the mapping is a table in
the module comment of each file it bears on — `table/fonts.rs`, `table/graphics.rs`, `survey.rs`,
with `table/file_structure.rs` and `table/interaction.rs` corrected in place because each has one
or two sites — and each table says explicitly that **everything else that file cites was checked
and agrees in both editions**. That last sentence is the part with value: it is what stops the next
round redoing the sweep.

## 5. One limit of the instrument, found by using it

`every_row_at_a_read_subclause_is_named_by_a_sentence` assumes every row citing a subclause comes
from a sentence *of that subclause*. One row does not: `graphics/named-resources-are-defined` binds
part 2 on `TechNote 0010` A002, which **widens** part 4's rule to a part whose own text does not
state it (`crate::clarification` records the shape). The reading of ISO 19005-2 section 6.2.2
therefore carries a sixth entry whose `says` opens "not part 2's own sentence", which keeps the
mapping complete and keeps the distortion visible in the one place a reader would look. A cleaner
answer would be a `Carried::Clarified` variant; it is not worth one row, and it is written down
here so that the second such row is what buys it rather than the first.

## 6. What did not change

- **`over` is 0 on all six targets**, with the one standing corpus miss (`6-6-2-3-3-t03-fail-b`,
  errata A029) unchanged. Every row added is `Check::Processor` or `Check::Unchecked`, so no
  document's verdict can move.
- **`crates/pdf-transform/tests/archive_unconsidered.txt` is untouched and still empty.** No row
  was promoted to `Check::Implemented`; the two that would have been are section 3's, and section 3
  is why.
- **No requirement identifier was removed or re-keyed.** Three were added:
  `graphics/output-intent-is-the-default-blending-space` and
  `fonts/embedded-programs-are-what-a-processor-renders` (`Processor`), and
  `signatures/signature-object-conforms-to-pkcs7` (`Unchecked`).
