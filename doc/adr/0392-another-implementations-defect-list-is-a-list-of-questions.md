# 0392 — Another implementation's defect list is a list of questions, and the answers are tests

Status: accepted
Date: 2026-08-16
Session: 557

## Context

The project owner asked for all 167 issues on `LaurenzV/hayro` — open and closed — to be read and
triaged, and for the ones that would interest the quorra developers to be written up.

hayro is not an arbitrary choice of tracker. It is the only other feature-complete pure-Rust PDF
rasteriser, it is the oracle's fourth reading (`Reference::Hayro`), and this tree links three of
its crates — `hayro-jbig2`, `hayro-jpeg2000` and `hayro-ccitt` — into shipped binaries. So its
issue list is three different things at once: a set of questions about clauses, a set of possible
defects in our own dependencies, and a set of observations about rasterising PDF that a renderer
we do not own would want.

`CLAUDE.md` principle 5 governs the whole exercise and is easy to violate here without noticing.
"hayro draws X" is a fact about hayro. It is never a statement about what is correct, and the
temptation — with 167 concrete, well-written bug reports in front of you — is to treat a defect
list as a specification.

## Decision

**1. Every issue is sorted into exactly one of four buckets, and the buckets are the product.**
A defect this tree may share; a defect in a crate this tree links; something quorra should read;
not relevant. 17, 36, 37 and 77 respectively. The sort itself is the durable part: it is what
lets the next round read `doc/HAYRO_ISSUES.md` instead of the tracker.

**2. A bucket-1 issue is answered against the standard, never against hayro.** For each, the
clause is named, the standard's own sentence is read, and *this tree's* code is checked. Sixteen
of the seventeen turned out already right. One was not, and is fixed here.

**3. Where this tree is right, the answer becomes a test.** This is the decision that changes what
the round is worth, and it came from the project owner mid-round. A round that reads code and
writes "we checked, we are fine" has produced a sentence with a half-life: the code moves, nobody
re-checks, and the sentence becomes one of the false claims `doc/todo/01`'s sweeps exist to find.
A round that writes a test has produced something that fails when it stops being true. Six tests
were added, each naming in its doc comment the issue it guards against and the clause it rests on:

| test | clause | guards against |
|---|---|---|
| `lexer::a_number_longer_than_the_fast_path_is_still_the_number_written` | §7.3.3 | a wrapping mantissa accumulator (#1341) |
| `lexer::a_digit_run_ending_in_letters_is_one_token` | §7.2.3 | splitting `5f` into `5` and the fill operator (#994) |
| `numbers_without_digits::a_digit_run_that_swallows_an_operator_paints_nothing` | §7.2.3 | the same, end to end, in ink |
| `contents_entry::a_contents_entry_that_is_a_bare_integer_says_so_too` | §7.3.10 | unwrapping a `/Contents 8` (#1189) |
| `composite::an_escaped_cmap_name_is_the_name_it_spells` | §7.3.5, §9.7.5.2 | `/Identity#2DH` read as a name of its own (#11) |
| `sub_pixel::a_bowtie_is_not_taken_for_a_rectangle` | §8.5.3.3 | a crossing quadrilateral on a rectangle fast path (#1336) |
| `crypt::a_key_length_outside_table_20s_range_is_refused` | §7.6.3.2 | slicing a 16-byte digest at `/Length ÷ 8` (#1273) |
| `image::end_of_block_overrides_the_rows_parameter` | §7.4.6 | the defect below |

**4. The one defect found is fixed: `/Rows` is not a row count.** `decode_ccitt` handed the
`DecodeParms` `/Rows` to the codec whenever it was non-zero. Table 11 does not permit that.
`/EndOfBlock` is "[a] flag indicating whether the filter shall expect the encoded data to be
terminated by an end-of-block pattern, overriding the Rows parameter", and only "[i]f false" does
the filter "stop when it has decoded the number of lines indicated by Rows". Its default is true.
So in the ordinary file `/Rows` does not bind, and the decode is bounded by §8.9.5.1's `/Height`.
`pdf_model::ccitt_rows` is that derivation, extracted into a named function precisely so it can be
tested.

The same reading corrected two quotations. `pad_to_height`'s doc comment and §7.4.6's ledger note
both cited "whichever occurs first" as the governing rule; that is the *conditional* half of the
`/EndOfBlock` row with its "If false" dropped. Both now quote the `/Rows` row, which states the
same thing unconditionally.

**5. Three residues are refused rather than fixed, each with what would change the answer.**
`doc/todo/53`: the `/EndOfBlock`-false short decode, which needs a second field on the sandbox
pipe; `5f`'s missing diagnostic, which cannot be separated from the deliberate `12pt` leniency
without inventing a rule the standard does not state; and a Type 1 program's unassigned codes
claiming glyph 0, which is a `read-fonts` API question before it is ours. None is witnessed by a
corpus document. Refusing a fix is a decision and is recorded as one.

**6. The quorra document is a reading list and says so at the top.** `doc/HAYRO_ISSUES_FOR_QUORRA.md`
is written to be handed over, in `doc/QUORRA_FUNCTION_PAINT.md`'s voice. It states three things it
is not: a defect list, a claim that quorra has any of these problems, and a claim that hayro is
right. Without that paragraph a document of this shape reads as an accusation.

**7. The record lives in a file of its own rather than in `doc/oracle-and-corpus.md`.** That
document is about the instrument — which references vote, what a tolerance admits. This is a dated
survey of somebody else's defect list, which decays as they fix things and whose main job is to
stop the next round re-reading it. `doc/JPEG2000_FEEDBACK.md` is the precedent.

## What the codec audit found, since it is the part with a number attached

The three linked crates were checked against the versions this tree actually pins, which is the
only question that matters and is not the one the tracker answers.

- **`hayro-ccitt` 0.3.0** is the newest published version and no bug fix has landed after it —
  only two refactors. Nothing owed.
- **`hayro-jbig2` 0.3.0** is the newest published version and *four* commits have landed after it,
  including the fix for their #1261. Measured rather than assumed: the regression file that fix
  added upstream was fetched and run through `pdf_sandbox::decode` on a debug build with overflow
  checks on, and it comes back `JBIG2: unexpected end of input` — a clean typed refusal. The
  reason is the interesting part. The overflow was reachable in their revision because of a June
  fast-path rewrite, and 0.3.0 predates the rewrite as well as its fix. **This tree's version is
  older than the defect**, so there is nothing to take: taking the fix means taking the rewrite,
  and no release carries either.
- **`hayro-jpeg2000`** is pinned to `close2/hayro` `1dc833f7`, and the pin's terms changed. Two of
  its three fixes are now on hayro's `main` — one of them this project's own pull request #1340,
  merged the day before this round. The third, the reduced-resolution allocation, is not, and no
  pull request exists for it. And a fourth condition was found: hayro's #1188 reports
  `let rb = lab.ra.unwrap_or(200)` where `lab.rb` is meant, which is present in **both** published
  versions, 0.3.5 and 0.4.0, and fixed on `main`. Going back to crates.io today would regain it.
  So `Cargo.toml`'s "the moment a release carries both" is now a condition with three parts, and
  the note says so.

  One consequence reaches the oracle: `hayro` 0.7.1 pulls `hayro-jpeg2000` **0.3.5** through
  `hayro-syntax`, so `pdfref-hayro` carries that typo today. A disagreement with the fourth
  reading on a JPEG 2000 plate with explicit CIE Lab parameters is explained rather than mysterious.

## Consequences

- The tracker does not need reading again. `doc/HAYRO_ISSUES.md` carries the date and the search
  that finds what is newer.
- Seven clauses gained a test and their ledger rows say what was checked and when: §7.2.3, §7.3.3,
  §7.3.5, §7.4.6, §7.6.3.2, §7.7.3.3, §8.5.3.3, §9.7.5.2.
- One clause family gained a correction it would not otherwise have had, and the *route* is worth
  noting: nothing in the corpus draws wrong because of the `/Rows` reading, so no instrument this
  project owns would have found it. It came from reading somebody else's bug report and then
  reading the clause. That is a third source of work beside the two `CLAUDE.md` names, and it is
  cheap.

## The finding that is about method rather than about code

hayro's #1331 reports that a Type 3 font without `/ToUnicode` yields no character, where a simple
outline font falls back to the encoding's glyph name. **This tree held exactly that position for
three hundred sessions** — `type3.rs` said a Type 3 glyph name "names a procedure, so … the name is
no evidence at all about the character" — and it was wrong for a reason the standard states plainly:
§9.6.4's step b) is "[g]et the glyph name from the Encoding entry", and §9.6.5.3 makes
`/Differences` "the complete character encoding for this font". Corrected in session 326; the
readback moved from 98.2% to 99.1% of `pdftotext`'s words.

Two independent implementations reached the same wrong conclusion from the same plausible argument,
and two sentences of the standard settle it. That is principle 5's case made better than any
statement of the principle: agreement between implementations is not evidence, because the thing
two readers of a clause are most likely to share is the misreading.
