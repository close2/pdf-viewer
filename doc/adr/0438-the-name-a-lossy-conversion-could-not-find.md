# ADR 0438 — The name a lossy conversion could not find

Status: accepted, 2026-08-19. Session 603. Takes `doc/todo/03` §1's chunk over a population no
reference had ever been pointed at — the SafeDocs crawl — and fixes the defect its first two
thousand documents produced: a resource name that is not valid UTF-8 could not be found, because
the interpreter carried it as a `String` built with `from_utf8_lossy`. Amends §7.3.5's ledger row.

## The chunk, and why this one

`doc/todo/03` §14 ends with "[e]very population on this disk is ranked", and that sentence is about
the *curated* corpora: `pdfCabinetOfHorrors` and `govdocs1-error-pdfs` (§8), the rest of
`format-corpus` (§12), `pdfbox`'s 64 (§13) and `pdf-differences`' 37 (§14). It is not true of the
largest population this project holds. **The 65 944 crawled documents have only ever been
*surveyed*** — 145 archives, one process each, five questions this tree asks of itself — and a
survey answers "does this reader report anything", never "is the page right". `CLAUDE.md`'s
robustness question has the world for its denominator, and 64 507 documents this tree calls
*complete* had never been put beside another renderer at all.

So: **two whole archives, 2000 documents**, `0100` and `7680`. Which archives is immaterial and
that is ADR 0261's finding — the crawl is sorted by SHA-256 and cut into 7933 pieces, so an
archive is a hash bucket and any 2000 of them are an unbiased sample of all 7 932 878.

The instrument is `doc/todo/00` step 7's number, the one sessions 505, 544, 554 and 558 applied to
the curated corpora: page one at 72 dpi from this tree and from `pdftoppm`, `mutool` and `gs`,
every invocation explicit about the page box (trap 3), ranked by **our ink minus the lightest live
reference's**, with each panel's raster size beside it because a size disagreement is invisible to
a ranking by ink (§13's lesson). 0.33 s a document at sixteen workers; 2000 documents is eleven
minutes of wall clock, which is what makes this population affordable at all — the full oracle,
with its structural similarity, its consensus vote and its seven verdicts, is a different
proposition and §"What this leaves" below prices it.

## What the ranking said

**The head separates from the body by three orders of magnitude, which is what this instrument is
for.** `0100223.pdf` at **−225.633 of 255** against a next-largest of −5.040: our ink is
**0.000** — a blank sheet — where `poppler`, `mupdf` and `ghostscript` each deposit about 225.6 and
agree with one another to 0.93. A full-page scan, drawn by everybody except us, at the same raster
size in all four panels.

The rest of the tail is what the curated corpora have been producing since session 544: −5.040,
−3.043, −2.896, −2.371 and shallower over 1000 documents in `0100`, and −6.256 downwards in `7680`
— glyph weight and the odd image. Nothing else in 2000 documents is a whole page.

## The defect

`0100223.pdf` is a Hewlett-Packard scanning driver's output. It names its image `XObject` after the
Windows path the operator scanned from:

```text
/XObject << /C:\Documents#20and#20Settings\user\…\Plaquette#20ANC#20-#20Contr#F4le#20de#20fonctionnement.PDF 4 0 R >>
```

and its content stream ends `… Do` on the same name, byte for byte. `#F4` is the file's own
spelling of 0xF4, which is what *ô* is in Windows-1252 — and 0xF4 is a byte no UTF-8 sequence
begins.

This tree reported `MissingResource { category: "XObject", detail: "/C:\Documents and
Settings\… is not in /XObject" }` and drew nothing. **The file is not malformed and the report was
wrong**: `Interpreter::resource_entry` takes the name as a `&str`, and every caller reached it
through `run::name_at`, which did

```rust
String::from_utf8_lossy(name.as_bytes()).into_owned()
```

So the probe was `Contr\u{FFFD}le` and the key was `Contr\xF4le`, and no dictionary lookup could
ever match. §7.3.5 states both halves of why:

> Beginning with PDF 1.2 a name object is an atomic symbol uniquely defined by a sequence of any
> characters (8-bit values) except null (character code 0). Uniquely defined means that any two
> name objects that, after all escaping is expanded (see below), and the resulting sequences of
> bytes are not an exact binary match denote different objects.

and, in the same clause:

> Ordinarily, the bytes making up the name are never treated as text to be presented to a human
> user or to an application external to a PDF processor.

The parser had this right all along — `pdf_syntax::Name` is `Arc<[u8]>`, its documentation says
"[n]ames are compared as bytes rather than as text because the specification defines them that
way", `Dictionary` is keyed by `Name` and `Dictionary::get_by_name` probes it with one. What was
wrong was the *interpreter's* half of the journey: the operand became text on the way from the
content stream to the dictionary, and text is exactly what §7.3.5 says a name is not.

**The second direction is worse than the first and had no witness.** `from_utf8_lossy` maps every
invalid byte to one replacement character, so `/A#F4` and `/A#F5` both became `A\u{FFFD}`: a `Do`
on one of them would have found the other, drawn an object the content stream did not name, and
said nothing. That is trap 1's archetype — a mark on the page from the wrong object — and it is
pinned by a test now rather than by luck.

## What was changed

`resource`, `resource_entry`, `unresolved_resource` and `note_missing_resource` take a `&Name` and
probe with `Dictionary::get_by_name`; `run::name_at` returns the operand's `Name` unchanged. The
lossy conversion survives at each *report* and each *trace label*, which is where §7.3.5's
"occasionally the need arises to treat a name object as text" actually applies — a human reading a
diagnostic. Seven operator families share the corrected lookup: `Do`, `gs`, `cs`/`CS`, `scn`, `sh`,
`Tf` and `BDC`'s property list.

**Two neighbouring conversions were deliberately left as text**, and the rule is worth stating
because it is the one that decides where the boundary goes. A marked-content *tag* (`/OC`,
`/Artifact`, `/ReversedChars`, `/AF`) and a rendering intent (`/AbsoluteColorimetric`) are names
**the standard itself defines**, so they are compared against ASCII literals — as bytes now, which
is the same answer by a shorter route. And §9.6.2.2's fourteen standard fonts are ASCII names, so
`standard_font_named` still asks for a `&str` and a name that is not one cannot be one of the
fourteen: `Name::as_str` returning `None` *is* that answer rather than a lookup lost on the way.

## What it moved

- **The witness**: 0 commands and one report → **1 command and no report**, ink 0.000 → 225.476
  against `poppler` 226.567, `mupdf` 225.640 and `ghostscript` 225.633. Its gap goes −225.633 →
  **−0.158**, which is inside the spread of the three references themselves.
- **The other 999 documents of archive `0100` did not move at all.** The ranking was re-run whole
  after the change and diffed panel by panel: exactly one row differs, and every reference panel
  is byte-identical, which is the instrument saying the change is the change and nothing else.
- **No gate moved**, which is the honest form of this population's rate: the pdf.js corpus's 974
  documents contain no name a text conversion mangles, so a defect that blanks a whole page on the
  web is invisible to every ratchet this project has. `doc/todo/03` §1 has said since session 470
  that a construct's absence from a corpus is a *measurement* rather than an excuse, and this is
  the same sentence from the other side.

The rate this chunk can honestly claim is **1 document in 2000**, measured; a whole-crawl census
would need a walk that classifies a name by the entry that names it, which is
`damaged_stream_census`'s `who_names_what` shape and is a round of its own.

## The test, and why it is synthetic

`crates/pdf-model/tests/missing_resources.rs` gains a pair: a name whose bytes are not UTF-8 is
found, and two names differing only in such a byte are two names. Both are hand-built, for two
reasons that happen to agree — trap 8's (a corpus finds what documents contain, not what the
clause says, and the *collision* direction has no witness anywhere on this disk) and the promotion
rule's (`doc/todo/03` §3: a SafeDocs document is recorded by archive, member and SHA-256 and never
committed). The witness is `cc-main-2021-31`, archive `0100`, member `0100223.pdf`, 320 487 bytes,
SHA-256 `033f9b02ed7f447482775169e9ef4e874e55c9edeff6816e15b8f5810920995b`, as `manifest.tsv` records it.

## What this leaves

**The crawl is ranked over 2000 of 65 944 documents.** The remaining 63 944 are eleven minutes per
2000 at this round's settings, so the whole population is about six hours of wall clock at sixteen
workers — affordable by a round that wants it, and cheaper than it sounds because the reference
renders are the cost and `PDFREF_CACHE` does not help an instrument that renders each document
once. What is *not* affordable is the oracle proper: its per-page work is four renders plus a
structural-similarity comparison plus a vote, and it exists to ratchet a fixed corpus rather than
to walk a new one. Pointing it at the crawl would need a decision this round did not take — what a
verdict *means* on a population with no expected values, which is the question ADR 0393 answered
for `pdf-differences` and would have to answer again here.
