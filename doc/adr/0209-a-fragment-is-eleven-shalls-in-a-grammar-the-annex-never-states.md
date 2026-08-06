# ADR 0209 — A fragment is eleven `shall`s in a grammar the annex never states

Status: accepted, 2026-08-06 (session 369).

## Context

ADR 0206 gave the ledger the standard's eight normative annexes and found one of them entirely
unbuilt: **Annex O, fragment identifiers.** Eleven parameters, every one a `shall` addressed to
"the PDF processor", saying what a document shows when it is opened through a URI —
`report.pdf#page=12`. Its five rows were the ledger's only `silent` ones, which `CLAUDE.md` calls
the status worth hunting: not implemented, and nothing says so.

**And no gate in this project could ever have found it.** A document cannot contain a fragment
identifier; the fragment arrives with the *request*. The corpus and the oracle are blind to Annex O
by construction, which is `CLAUDE.md`'s two-denominators argument in its purest form — coverage
answering a question robustness cannot see.

Nine of the eleven parameters name a mechanism this tree already had. What was missing was the
sentence that joins a URI's fragment to them.

## Decision

### Read in `pdf-model`, applied in `viewer-core`, split in that order

`pdf_model::fragment::Fragment::parse` takes the text after `#` and answers with the parameters it
read, **in the fragment's own order**, plus the ones it could not read, **named**. It needs no
document and no window, because the grammar and the eleven argument forms do not.

`viewer_core::Open::apply_fragment` then carries them out against a document and a viewport, and it
is called from exactly one place — immediately after Table 29's `/OpenAction` — because §O.2.2 says
these "should be processed immediately after any other document-specified open parameters have been
processed". The document states where it opens; the URI overrules it.

`Command::Open` gains `fragment: Option<String>`, undecoded. `pdf-viewer doc.pdf#page=5` is the
first caller, which is what makes this a capability of the *program* rather than of a crate — the
failure ADR 0177 named, where §12.5.6.19's `/H` was `implemented` and no host could reach it.

### Seven of the eleven are carried out, and four are refused by name

| | | |
|---|---|---|
| `page` | §12.4.2's numbering, from one | ✔ |
| `nameddest` | §12.3.2.4's two tables, through `Destination::read` | ✔ |
| `structelem` | §14.7.2's `/IDTree`, then §12.3.2.3's algorithm for the element's page | ✔ |
| `comment` | Table 166's `/NM` on the page chosen so far → §12.5.1's focus | ✔ |
| `zoom` | Table 149's `/XYZ`, percentage → factor, corner flipped | ✔ |
| `view` | Table 149 itself, read where §12.3.2.2 reads it | ✔ |
| `viewrect` | Table 149's `/FitR`, corner flipped | ✔ |
| `ef` | opening an embedded file is a host's decision, and what follows it is about another document | reported |
| `highlight` | a rectangle, and what this program highlights is a *range of text* | reported |
| `search` | no document-wide search exists | reported |
| `fdf` | fetching a URI is a host's, and principle 3 keeps it out of the renderer | reported |

`Parameter::unhonoured` is where the four say so, in the shape `requirements::Kind::unmet` already
uses: one sentence per arm, a claim about *this tree* rather than about the standard, and it decays
the same way a ledger row does.

**`highlight` is the only one of the four that is a missing concept rather than a missing
capability**, and that distinction is the reason this round did not implement it. This program's
highlight is a range of the readback with the geometry the text layer gives it; a rectangle
measured from the corner of the page is not a range of anything. Drawing something rectangular near
it would have been inventing a feature and attributing it to the annex.

**`ef` had to be more than a refusal.** Table Annex O.3: "[a]ny remaining parameters after this
parameter apply to the selected embedded file." So a reader that skipped `ef` and went on would
apply `page=3` to the *wrong document* — a URI followed to somewhere it does not point.
`apply_fragment` stops there and says how many parameters it did not apply.

### Three findings in the annex's own text

**It prints the wrong code for its own separator.** §O.2 says parameters are "separated by the
AMPERSAND (28h) character", and 28h is `(`. The standard settles it against itself: Table D.2 gives
`AMPERSAND` the code 0x26. Checked in the PDF as well as in `doc/md/`, so it is the standard's
erratum rather than the conversion's. The reader takes the character the sentence *names*.

**It never states the `=`.** Two separators, two tables of parameter names and their arguments, and
not one example of a fragment — so nothing in ISO 32000-2 says how `page` is joined to `12`. `=` is
therefore a **documented choice**, not a reading: §O.1 defines these as a URI's fragment
identifiers, where a list of `&`-separated `name=value` pairs is the shape the surrounding syntax
already has, and no other spelling gives the two separators the annex *does* state anything to do.

**Its coordinate rule has two halves that pull apart, and only one reading keeps both.** §O.2.2 says
"[a]ll coordinate values … shall be expressed in the default user space coordinate system", while
`zoom`, `viewrect` and `highlight` each measure "from the top left corner of the page". Default user
space's origin is the *bottom* left (§8.3.2.3). Both sentences are true at once exactly when the
*units* are default user space's and the *origin* is the page's top-left corner, which is what
`Open::in_default_user_space` computes — against `display_box`, because §12.2's `/ViewArea` decides
what "the page" is on a screen. `view` is measured the other way, and the annex says so: its
arguments "shall correspond to those found in 12.3.2.2".

The same annex states one magnification twice: `zoom` is a percentage, Table 149's `/XYZ` is a
factor, three rows apart.

### Two things this deliberately does not do

**It is not a URI parser.** RFC 3986 splitting is the host's, and `viewer-core` never sees it.
`pdf-viewer` splits the argument by a rule of its own — **the filesystem decides, not the
punctuation**: an argument naming an existing file is taken whole, and only one that does not is
split at its first `#`. One `stat` on the launch path, and a file called `a#b.pdf` still opens.

**It does not percent-decode a URI.** It decodes each *argument*, after splitting, because a `%26`
inside a destination's name is data and not a separator — which is what `structelem`'s "byte string
with URI encoding" needs and where RFC 3986 section 2.4 puts the boundary.

### An argument count is part of what the table states

A parameter given more arguments than its row names is not read: nothing in the annex defines a
fourth argument to `zoom` or a second to `nameddest`, and taking the ones that fit would mean
looking up a destination whose name is not the one that was written. `view` is the exception and not
by choice — how many numbers a Table 149 keyword takes is that table's business, and
`View::from_keyword` reads them exactly as it reads a destination array's.

### Numbers are §7.3.3's, and refused rather than salvaged

The annex asks for "an integer or floating point value" and states no syntax, so the reader takes
the standard's own — which rejects an exponent (§7.3.3 bans it outright), `inf` and `NaN`, the three
spellings `f32::from_str` accepts and PDF does not have. `pdf_syntax::Lexer` reads numbers
*leniently* on purpose, because a content stream with `1.2.3` in it still has to draw; a fragment
has nothing to keep drawing, and a mistyped page number silently read as zero would open the
document somewhere nobody asked for.

## Consequences

- **`silent` is 0, from 5.** The ledger prints 875 rows: `implemented` 399, `partial` 240,
  `reported` 30, `inapplicable` 85, `writer-side` 8, `out-of-scope` 113. §O.1 and §O.2 are
  `implemented`; §O.2.1 and §O.2.2 are `partial` with the four refusals named; §O is `partial`.
- **25 tests**: thirteen in `pdf_model::fragment` for the grammar, eleven in
  `viewer-core/tests/fragments.rs` against real corpus documents, one in `pdf_model::destination`
  for `structelem`'s page. The workspace's total is 1187 and the ten crates' is 1095.
- **The fragments in the grammar tests are written by hand and that is unavoidable**: a fragment
  identifier cannot come from a corpus document. What they are *applied to* is a real file, and
  every expected value in the viewer-core tests is derived from that file's own objects — the
  comment above each test quotes them.
- **One fuzz target added, the tenth**, at 1 000 000 runs with no artifact. A fragment is untrusted
  input that no document carries, which makes it the first parser in this tree whose bytes come from
  the person who sent the link.
- **`Open::pending_view` became `pending_views`**, a list applied in order, because §O.2 makes
  left-to-right normative and a fragment may state two views. `/OpenAction` puts at most one there.
- **No pixel moved**: corpus 73 incomplete, oracle 856/68/750, text 99.2%, quorra 914/42/1, dates,
  XMP and JPEG 2000 all unchanged.
- **Seen working**: `pdf-viewer 'doc/ISO_32000-2_sponsored_EC3.pdf#page=100&zoom=150'` under `Xvfb`
  opens at *page 100 of 1023* and asks for an 893×1263 raster, which is 150% of a 595×842 page; and
  `#highlight=1,2,3,4&ef=data&page=3&pagemode=none` prints four notes — the highlight refused, the
  embedded file refused, the one parameter after it not applied, and `pagemode` named as no
  parameter of this annex.

## What is left, and it is written down rather than left to be rediscovered

`doc/todo/39` keeps the four. `search` wants a document-wide search, which `viewer-core`'s crate map
already owes; `fdf` wants a fetch, which is `Command::Supply`'s shape and needs a host that has a
URI; `ef` wants `doc/todo/38`'s levels, because the annex asks for a policy — "a PDF processor may
choose to prompt the user or even prevent opening of the file" — rather than for a refusal; and
`highlight` wants a way to say "this rectangle" that this vocabulary does not have. Each is a
sentence in the ledger and a row in that file.
