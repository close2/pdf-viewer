# ADR 0858 — A dictionary body that walked through `endobj`, and the object it assembled out of two

Status: accepted. Session 908.
Clauses: ISO 32000-2 §7.3.7, §7.3.10, §7.3.8.1, §7.2.3, §9.6.4.
Code: `crates/pdf-syntax/src/parser.rs` (`read_dictionary_body`'s new arm).
Tests: `crates/pdf-syntax/tests/robustness.rs::a_dictionary_body_stops_at_the_keyword_that_ends_its_object`.
Continues ADRs 0784, 0787. Beside ADR 0859, which is the walk that found it.

## What was found

`corpus-cache/tika-issue-tracker/batch5/cairo/cairo-85141-0.zip-3.pdf` page 1 heads round 908's
ink ranking of that directory at **4.630 against `poppler`'s 1.757 and `mutool`'s 1.662**, and
reports nothing at all. It does not converge: the four-render ladder `doc/oracle-and-corpus.md`
§3d asks for gives 4.630 / 4.588 / 4.586 / 4.645 at 72, 144, 288 and 576 dpi against a reference
pair flat at 1.7, so the disagreement is content rather than scan conversion. The page is a
Finnish university problem sheet, and what this tree draws and the references do not is most of
its text.

The text is a Type 3 font's. Object 76 is `/F16`'s `/CharProcs`, and the file's producer's bytes
for it stop in mid-entry — `/a111 56 0 R /a112 57` and then another stream's compressed data,
written over the rest of the dictionary. `qpdf --check` calls the file damaged and gives up on
the object; `poppler` and `mutool` draw the page without that font.

**This tree did not give up, and what it produced instead was not the object.** `Document::get(76)`
answered a **stream**, whose dictionary held object 76's forty surviving `/aNN` entries, a
manufactured key made of the binary damage, **and object 78's `/Length 149` and `/Filter
/FlateDecode`** — with **object 78's stream data** attached to it. Forty glyph procedures then
drew, from an object no producer wrote, and `interpret` reported `unsupported []`.

## Why it happened

`read_dictionary_body`'s last arm:

> A non-name where a key belongs. Skipped rather than fatal: files with a stray value between
> entries are recoverable, and the alternative loses the whole dictionary.

The arm is `_ => {}`, so it skips *every* token that is not a name and not `>>`. On this object
that included the damage, then `endstream`, then `endobj`, then `78`, `0`, `obj` and `<<` — after
which the reading was inside the next object's dictionary, took its two entries, met **its** `>>`,
and returned `Ok`. `parse_dictionary_or_stream` then found the `stream` keyword that followed and
attached the data.

**This is trap 28 exactly: a recovery's guard is a claim, and the comment above it is a different
claim.** The comment describes a stray *value between two entries*. The arm describes any token
whatsoever. Walking through `endobj` and a following object header is not the first thing and is
not recoverable: it does not lose entries, it **takes another object's**.

It is also the one outcome `parse_dictionary_body`'s own doc comment forbids in writing — "an
error is never a truncated object, because a shortened dictionary handed back where a whole one
was asked for would render a wrong page and report success". The rule was stated at the top of the
function and broken twelve lines below it, and neither ADR 0784 nor ADR 0787 — both of which
argued about exactly where a damaged dictionary's reading stops — asked what stops it.

## The clause

Three of the four keywords are given their meaning by clauses this row already cites, and each of
them *ends* the object a dictionary body is inside:

- §7.3.10: an indirect object's definition is "followed by the value of the object bracketed
  between the keywords obj and endobj".
- §7.3.8.1: a stream is a dictionary followed by zero or more bytes "bracketed between the keywords
  stream (followed by newline) and endstream".

So none of `obj`, `endobj`, `stream` or `endstream` can stand where a key belongs, and meeting one
is not an ambiguity to skip past: it is proof that the `>>` this body is looking for **is not in
this object**. ADR 0787's argument is what settles the rest, and it settles this the same way —
the prefix is the producer's own only because the reading tokenised *continuously from the `<<`*,
and a reading that has crossed `endobj` is no longer continuous with anything the object states.

**Four keywords and not more, which is a decision.** §7.5.4's `xref` and §7.5.5's `trailer` and
`startxref` are structural too, and a body that runs far enough will meet them — but every object
this reader parses is entered at its own header and ends at `endobj`, so those three are only ever
reached *after* one of the four, and adding them would widen the arm without adding a case. Where
`endobj` itself is missing the reading meets the next object's `obj` instead, which is why `obj` is
in the list beside it.

The arm added therefore stops the body at those four keywords, and nothing else changes: a stray
number or string between two entries is still skipped, `max_dict_len`, the null rule and the
duplicate-key choice are still one function, and `parse_damaged_dictionary` still offers the
prefix to a caller that asks for it by name.

## What it changes on the page

On the witness, `Document::get(76)` is now §7.3.10's null, so §9.6.4's own sentence applies for the
right reason — `Type3Font::read` reports `font /F16 is a Type 3 font with no /CharProcs
dictionary` — and the page draws **1.70076**, inside the references' 1.662 … 1.757 rather than
2.7 levels above both. `Document::damaged_dictionary(76)` answers the forty-one-entry prefix,
stopping at `endstream` at byte 20678, which is the door ADR 0784 built for a consumer that wants
a subset *and says so*. Nothing consumes it for a font today; that is `doc/todo/03` section 47's
open item and not this one's.

**Forty real glyph procedures stop being drawn, and that is the point rather than a cost.** They
were reachable only through an object assembled out of two, under a reader that said nothing; a
prefix drawn deliberately, reported, and taken through the door that names it is a different
change and a later one.

## Population

**Measured, not assumed, and over two corpora with their names in the sentence.** `tools/safedocs
survey --dir` was run twice on each, once with the binary built before the change and once after,
with each run's own `pdf-sandbox-worker` beside it, and the per-document verdict lines diffed:

| corpus | documents | documents whose page-one verdict changes |
|---|---|---|
| `CC-MAIN-2021-31`, the whole SafeDocs crawl | 65 944 | **0** |
| `corpus-cache/tika-issue-tracker` + `doc/pdf.js` + the four `doc/corpora/` submodules | 24 324 | **8** |

**Not one of the 65 944 crawled documents changes**, which is the shape `doc/todo/03` section 1's
argument predicts: a crawled page is a file a producer shipped and a web server served, and this
defect needs an object physically overwritten by another object's bytes. The eight are all in the
issue trackers — files somebody filed *because* a program choked on them — and every one of them
changes a **report**, never a mark another reader draws:

- `batch5/cairo/cairo-85141-0.zip-3.pdf` — the witness. Complete → `font /F16 is a Type 3 font with
  no /CharProcs dictionary`, ink 4.6304 → 1.70076 against `poppler` 1.7573 and `mutool` 1.6622.
- `batch5/cairo/cairo-85141-3.pdf` — `/F1 is a Type 3 font with no /FontMatrix` becomes `the /Font
  entry F1 is stated and is not a font dictionary`, §7.3.10's own sentence. The `/FontMatrix` it
  used to be missing was a claim about an object assembled out of two. Both draw 0 ink; `poppler`
  draws 0 too.
- `batch1/PDFBOX/PDFBOX-4351-0.pdf` — **the sharpest of the eight**, because the splice had
  manufactured a statement about the *standard*. Its trailer's `/Encrypt` states `/Filte^` where
  `/Filter` belongs, and the body then walked through `endobj` into `10 0 obj << /Filter
  /FlateDecode … >>` — so this tree refused the document with `unsupported encryption: /Filter
  /FlateDecode is not the standard security handler (§7.6.4)`, naming a security handler no file
  anywhere has ever stated. It is now `no first page`, which is less specific and *true*; neither
  `pdftoppm` nor `mutool draw` produces a raster for it either.
- `batch2/GHOSTSCRIPT/GHOSTSCRIPT-697885-1.pdf` — an `annotation with no /Subtype` stops being
  reported, because the annotation was an object assembled out of two and is now §7.3.10's null.
- `batch2/GHOSTSCRIPT/GHOSTSCRIPT-702586-0.pdf` — `Operator { "Do on /CBN" }` becomes
  `MissingResource { XObject }`, which is the same fact named by the clause that decides it. Ours
  and `poppler`'s ink agree to the digit either way, 72.648.
- `batch2/GHOSTSCRIPT/GHOSTSCRIPT-696131-0.zip-2.pdf` — `no first page` becomes `no usable
  cross-reference table … no object headers were found`, which is what a file whose every object
  header is followed by an unterminated dictionary actually is.
- `batch5/poppler/poppler-91353-0.pdf` — gains §7.3.7's own damage report, `this page's object
  states 3 entr(ies) and then stops being readable`, which it had earned all along and which the
  splice had hidden by completing the page dictionary out of the next object.
- `batch5/qpdf/qpdf-243-0.pdf` — complete → `no /MediaBox anywhere in the page's ancestry`, for the
  same reason: the media box it used to have was another object's.

**Six of the eight gain or sharpen a report and two lose one they should never have had.** No page
of `doc/pdf.js` is among them, which is why every raster gate's figures are unchanged.

## What would reopen it

A reading of §7.3.7 under which one of those four keywords may appear between two entries of a
dictionary. There is none: each is defined by a clause that gives it a structural position, and a
producer that meant a key wrote a name.
