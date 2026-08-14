# ADR 0348 — The witness needs glyphs before it needs a shaper, and the binary has none

Status: accepted, 2026-08-14. Session 513. Amends §12.7.4.3's ledger row, `doc/todo/21`,
`doc/todo/22` and `doc/stack.md`'s `rustybuzz` entry. Changes no pixel: the one code change is a
test pinning the refusal `freetext_no_appearance.pdf` already had. ADR 0112's asymmetry and ADR
0270's split are untouched.

## The question

`freetext_no_appearance.pdf` is the last item `doc/todo/22` owns and the one corpus document
§12.7.4.3's construction refuses whole: a free text annotation, no `/AP`, `/DA (/Helv 10 Tf 0
g)`, no `/DR`, and a `/Contents` that is a paragraph of Arabic. §12.5.6.6 sends that value
through §12.7.4.3 — "the PDF processor shall construct an appearance stream dynamically at
rendering time" — so the blank is this program's own answer, standing since ADR 0112 chose it
over a partial drawing. This round was asked to take the complex-script question honestly: what
would drawing that value actually require, which part of it is derivable from data this tree
could hold, and what would genuinely require the shaper `doc/stack.md` excludes by decision.

## The witness, measured

539 characters, decoded from the UTF-16BE `/Contents`: **36 distinct Arabic characters**
(U+0621–U+064A plus the tatweel U+0640 and the Arabic comma U+060C), **one combining mark** —
U+064B fathatan, once, in one word — **nine lam-alef pairs**, and otherwise only spaces, full
stops and line feeds. No digits, no Latin letters, no bracket pairs.

So the value needs four capabilities, in dependency order:

1. **Glyphs** an available face actually has, for the letters *and their positional forms*.
2. **Right-to-left order** — UAX #9. For this text the needed subset is small: the first strong
   character is Arabic Letter, so the paragraph direction is RTL (P2/P3), and every line is one
   RTL run with trailing neutrals. But it must exist, or the line draws mirrored.
3. **Joining forms** — the Unicode Standard's contextual shaping (ch. 9.2, ArabicShaping.txt's
   joining types): initial, medial, final and isolated forms selected by what joins on each
   side, plus the lam-alef ligature, which is mandatory and occurs nine times here.
4. **Mark placement** — GPOS-quality attachment for the fathatan, degradable to an overstrike
   for one mark in 539 characters.

Any construction with (1) but not (2) and (3) draws isolated forms left-to-right: a page of
plausible, confidently wrong text that reports nothing — trap 1's archetype, and the failure
mode ADR 0112 refused when the partial drawing was a scatter of dots. (2), (3) and (4) without
(1) draw nothing at all. **They land together or not at all.**

## The measurement that decides

The premise this round was handed — that the compiled-in Helvetica, Liberation Sans, carries
Arabic glyphs and GSUB — is **false for the bytes in this tree**, measured twice over:

- The `(3,1)` format-4 `cmap` of `data/standard-fonts/LiberationSans-Regular.ttf` maps every
  code point tried in U+0600–U+06FF and in both Arabic presentation-form blocks to glyph 0;
  the Bold face reads the same.
- Its `GSUB` script list is `DFLT`, `cyrl`, `grek`, `latn` — no `arab` — and `fc-scan` agrees
  from the other side: the face's language list has no `ar` and its charset's only F-range
  entries are the two Latin ligatures `fi` and `fl`.

The Foxit ten carry the standard Latin character set and nothing else (ADR 0270), and Symbol
and ZapfDingbats are what their names say. **No face this binary carries has one Arabic
glyph.** That settles the shaping question before it is asked: machinery without glyphs shapes
nothing, and principle 1 says what cannot be done properly now is not started now.

Two more measurements close the side doors:

- **The invented-`/Differences` route is shut machine-independently.** `with_differences` has
  31 free codes against 36 distinct characters, and the Adobe Glyph List `read-fonts` carries
  answers `char_to_name` with **no name for any Arabic character** (its generated table holds
  zero `afii` entries). So `named_glyphs_reach_more` cannot reach an installed Arabic face on
  any machine, and the refusal this tree ships is reproducible everywhere — the property ADR
  0133 paid 804 KB for, holding here by measurement rather than by design. The new test pins
  it: blank page, one report, both halves named.
- **The references draw nothing better.** `pdftoppm` prints "found character that the font
  can't represent" once per character and lays out the remainder: the value's full stops
  scattered over an otherwise empty page — looked at, and it is exactly the construction ADR
  0112 rejected. No reference draws this text, so there is no agreeing picture to be evidence
  of anything; the refusal is not in the minority against anyone who succeeded.

## The routes, costed

What a future round would need, in the order the dependencies run:

1. **A glyph source.** Either a fifteenth compiled-in face — an OFL Arabic face, which is a
   *typeface choice* of the kind the owner made for ADR 0133, a licence to read against
   `doc/third-party-data.md`, and several hundred kilobytes of binary — or the machine's own
   faces, which reopens the machine dependence ADR 0133 closed, in the one construction the
   oracle compares across machines; `doc/todo/21` §1 built and reverted exactly that for want
   of a machine-independent gate.
2. **Form selection**, two constructions:
   - *Presentation-form code points* (U+FB50–U+FDFF, U+FE70–U+FEFF) selected by
     ArabicShaping.txt's joining types — compiled-in statics under the Unicode licence, which
     `doc/third-party-data.md` would have to accept. Works only against a face whose `cmap`
     maps those blocks, which modern faces increasingly do not; the choice in (1) constrains
     this or is constrained by it.
   - *The face's own `GSUB`* `init`/`medi`/`fina`/`rlig` lookups. `read-fonts`, already in
     this tree under `skrifa`, parses `GSUB`; executing lookup types 1 and 4 against one known
     compiled-in face is bounded work with **no new dependency**. It is the shaper's core
     loop, and it is scoped to text this program generates — the exact scope `doc/stack.md`'s
     own `rustybuzz` entry reserved for a return.
3. **Order** — UAX #9's paragraph and run level, from Unicode's Bidi_Class data, the same
   statics decision as (2)'s first construction.
4. **Marks** — GPOS mark attachment, or a documented overstrike degradation.

And one sharpening of the stack argument, which is what this round can decide: **if shaping
returns for §12.7.4.3, `rustybuzz` is not its shape.** It would bring `ttf-parser` — a second
sfnt stack beside `skrifa`/`read-fonts`, the same shape ADR 0229 declined a second hash stack
for and ADR 0331 declined again — while `read-fonts` already reads the tables the work needs.
The exclusion's original argument (PDF content carries already-positioned glyphs; re-shaping
them moves them) is untouched and still decisive for everything a document positions itself.

## The decision

The refusal stands, now with its reading and its price written down, and the page keeps the
honest blank plus the report naming both halves. What was done:

- `the_arabic_free_text_declines_whole_and_names_both_halves` pins the witness: no ink, one
  report, `/Helv` and "not drawn at all" both present. It is the guard against the failure
  mode this round was warned about — a later change that draws this value partially or in
  logical order fails a gate instead of shipping a plausible wrong page.
- §12.7.4.3's ledger note no longer says the witness "is `doc/todo/21`'s per-character
  fallback": that fallback alone — a chain of faces asked per character — would draw isolated
  forms left-to-right even where it found every glyph. The witness needs (1)–(3) together, and
  the row and both todo files now say so.

## What was deliberately not done

- **No shaping machinery on spec.** Bidi tables, joining data or GSUB execution with no glyph
  source behind them would be dead code with a licence bill.
- **No fifteenth face this round.** A typeface this program will draw Arabic in is an owner's
  choice with a licence to read, not a side effect of a round about a reading.
- **No machine-face fallback.** ADR 0133's argument is not weakened by one witness: a page
  §12.7.4.3 constructs must reproduce across machines or the oracle cannot judge it.
- **No report-condition change.** The existing report already names both halves accurately;
  trap 11's cost arithmetic is untouched.
