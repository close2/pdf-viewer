# The residues left from reading hayro's tracker

Status: **closed** — all three residues, the last in the nine-hundred-and-eighty-third (ADR 1004);
kept as the record of three cases a round deliberately did not fix and what changed each answer.
Opened in the five-hundred-and-fifty-seventh session.
Priority: 53 — neither is witnessed by a corpus document, which is exactly why they are written
down rather than left to be rediscovered.
Clauses: §7.2.3 and §7.8.2 (item 1), §9.6.5.2 (item 2)
Code: `crates/pdf-syntax/src/lexer.rs`, `crates/pdf-font/src/type1.rs`
Reading: `doc/HAYRO_ISSUES.md`, ADR 0392

Each of these was found by asking one of hayro's issues of this tree, and each is a case the round
that found it deliberately did not fix. The reason was the same in all three: the fix is larger
than the finding, and nothing in the corpus draws wrong because of it. That is a reason to record,
not a reason to forget.

**All three are closed** — the third first (ADR 0434), then the second (ADR 0932), then the first
(ADR 1004). The third was the one whose shape was a correctness
hazard rather than a missing diagnostic — a CCITT decode bound and an image height sharing one
`u32` on the sandbox pipe, so a decode Table 11 legitimately stopped short of the image was
refused for being the size the clause asked for. The pipe carries both numbers, the lines between
them are blank and named, and the witness is a hand-built pair of pages in
`crates/pdf-model/tests/ccitt_bound.rs` because the corpus has no such document. The other two are
below, each with what closed it above the entry that said why it had been left.

## 1. A digit run that swallows an operator is silent — **closed** (ADR 1004, session 983)

The rule this entry said the standard does not state turned out to be one clause along from
where it was looked for. §7.2.3 and §7.3.3 cannot tell `5f` from `12pt` — both are one token and
neither is a number — but §7.8.2 can, by what the dropped tail *is*: an operator is "a PDF keyword
specifying some action that shall be performed", so a tail naming one of §8.2 Table 50's operators
is an action the salvage threw away, and a tail naming none is a spelling. `Lexer::salvaged` now
records what a salvage dropped and `content::reader::next_content_token` hands the interpreter the
whole run as a keyword where the tail is an operator, so `5f` is reported through the existing
`Unsupported::Operator` and `12pt` is still 12. The corpus witness that this entry said would
change the answer never appeared — `issue6342.pdf` gains three reports and no ink moves — and the
answer changed anyway, because the clause that separates the two was there to be read.

The original entry is kept below because it is the argument.

### The original entry

`5f` is one token (§7.2.3: `f` is a regular character, and a token ends only at a delimiter or
white space). It spells no number and no operator, so §7.8.2's rule applies — "when a PDF reader
encounters an operator in a content stream that it does not recognise, an error shall occur" — and
this tree reports nothing at all. It salvages the run to the number 5, drops the letters, and
paints nothing.

The ink is right, which is why this is a residue and not a defect: `a_digit_run_that_swallows_an_operator_paints_nothing`
pins it. What is missing is the report, and the report is what trap 5 exists for — a page that
silently drew less than its producer wrote.

**Why it was not fixed.** ADR 0303 corrected the digit-*less* case (`.`, `-`, `--`) and scoped
itself deliberately: `Lexer::salvage_number` reads `12pt` as 12 because content streams in the wild
need it, and `crates/pdf-model/src/fragment.rs` documents that as intended behaviour. Making `5f`
a `Keyword` makes `12pt` one too. The two would have to be told apart by something — a unit suffix
list, or a rule about where in the operand run the token sits — and inventing a rule the standard
does not state, to improve a *report*, is the wrong trade to make without a witness.

**What would change the answer**: a corpus page that draws wrong (rather than draws right and says
nothing) because of the salvage. `doc/todo/00`'s step 7 is the instrument that would find it — our
ink minus the lightest reference's — since the failure would show as a *missing* mark.

## 2. A Type 1 program's unassigned codes claim glyph 0 — **closed** (ADR 0932, session 941)

The diagnosis below was right and its estimate of the size was not. A census over every corpus on
this disk found **471 bare Type 1 programs with a custom encoding array**, of which the ones in 51
documents claim **109 789 codes their own array never assigns**; that is the ordinary shape of a
subsetted Type 1 font rather than an edge case. What kept it off the page is that the built-in
table is consulted only where the PDF `/Encoding` names nothing, and a producer that subsets a
font usually names the codes it goes on to show — so the wrong answer was computed and discarded.
`crates/pdf-font/examples/type1_encoding_census.rs` is the instrument.

The fix is not the `read-fonts` API question this file predicted. The `/Encoding` array is in the
**cleartext** part of the program, before `eexec`, so which codes it assigns can be read without
decrypting anything and without a second font reader: `assigned_codes` reads `dup <code> /<name>
put` and nothing else, and every other property stays `read-fonts`' answer. A code assigned to a
name the program lacks stays assigned, which is ISO 32000-2 §9.6.5.2's own instruction.

The reading that follows is kept because it is the argument, and because its last paragraph names
what a later round should still want: an upstream change so no consumer has to do this.

### The original reading

`crates/pdf-font/src/type1.rs:129-139` walks all 256 codes of a Type 1 program's built-in encoding
through `skrifa`'s `Type1Font::encoding`. `read-fonts` pre-fills a custom encoding table with
`GlyphId::NOTDEF` and `Encoding::map` returns that slot verbatim, so **every code the array's
length covers but does not assign comes back as `Some(0)`** — a code that is *encoded*, to
`.notdef`. The CFF path does not do this: `read-fonts`' CFF encoding returns `None` for an
unassigned code, and `crates/pdf-font/src/cff.rs:117-126` produces genuine `None`s.

So the two producers of `NameKeyed` disagree about what "unencoded" means, which contradicts
`name_keyed.rs:10-13`'s claim that they produce one shape. Three things downstream read the
difference:

- `name_keyed.rs:129` — an unassigned code selects glyph 0 and draws the designer's `.notdef`,
  often a box, where nothing should be drawn.
- `loading.rs`'s whitespace departure (`substitutes_notdef`, around line 1270) sees the code as
  encoded, so a code that means a space may deposit that box.
- `name_keyed.rs:147-154`'s "no character code maps to a glyph" refusal can never fire for a
  Type 1 program carrying a custom encoding array.

The regression test that would catch it, `loading.rs::an_uncovered_code_has_no_glyph_rather_than_a_guessed_one`,
iterates `corpus_bare_cff_fonts()` only and additionally skips a font whose table is fully
covered — which is what such a Type 1 always now is.

**What it needs.** Deciding what `Some(0)` from the built-in encoding *means*, which is not
obvious: a Type 1 program may legitimately encode a code to `.notdef`, and §9.6.5.2's own answer —
"[i]f an encoding maps to a character name that does not exist in the Type 1 font program, the
.notdef glyph shall be substituted" — makes the two indistinguishable at the glyph level. The
honest fix is probably to read the program's encoding *array* rather than its resolved map, so
that "no entry" and "an entry naming `.notdef`" stay apart; that is a `read-fonts` API question
before it is a question here.

**What would change the answer**: a page where a space or an unencoded code draws a box. The
`hollow_glyph_census` and `font_metric_census` examples are the places to look for one.
