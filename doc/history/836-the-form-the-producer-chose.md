# 836 — The form the producer chose, and two gates a clause supplies

Date: 2026-08-31. On `main` directly, from `ad8c7afa`.

ADR: 0763 — the form the producer chose, and the substitute that lost it.

Touched: `crates/pdf-font/src/vertical.rs` (new — `Downward`, `VerticalForms` and the reading),
`crates/pdf-font/src/predefined.rs` (`is_vertical_form`, `has_vertical_forms`, the collection pair
table and two tests), `crates/pdf-font/src/composite.rs` (`collection_names`, one reader for the
three questions that had three), `crates/pdf-font/src/substituted.rs` (`script_sample` reads it),
`crates/pdf-font/src/loading.rs` (the `Substituted` arm and `face_states_vertical_forms`),
`crates/pdf-font/src/lib.rs` (the module, and a crate doc comment that had claimed Table 116's data
was not in the tree for hundreds of sessions after it arrived),
`crates/pdf-model/tests/vertical_forms.rs` (new), `crates/pdf-model/tests/indexed_out_of_range.rs`
(new), `crates/pdf-model/tests/inline_image_abbreviations.rs` (a second witness),
`doc/conformance/ledger.toml` (§9.2.4, §9.7.3, §9.7.4.2, §9.7.5.1, §9.7.5.2, §8.6.6.3, §8.9.7),
`doc/todo/21-font-substitution.md` (§7, with its remainder priced), `doc/todo/03-more-corpora.md`
(§28's two open items closed), `doc/adr/0763-*`, this file.

## The primary item

`doc/corpora/pdf-differences`'s `VerticalText.pdf`: `/Identity-V` over a non-embedded
`Adobe-Japan1` `CIDFontType0`, whose producer wrote the collection's **vertical-form** CIDs — 7887,
7888, 7891, 7911–7916 — and which this tree drew with horizontal brackets and centred full stops on
columns `/DW2 [880 −1600]` had placed correctly.

The loss is one step and the clause names it. §9.7.4.2 leaves a substitute reachable only by
character; §9.10.2's `Adobe-Japan1-UCS2` gives a character; and that table sends CID 7911 (vertical
「) and CID 686 (horizontal 「) both to U+300C, because Unicode has one code point for the
character. §9.7.5.1's NOTE is what makes that a loss rather than a nicety — "in some cases,
different shapes are used when writing horizontally and vertically. In such cases, the horizontal
and vertical variants of a CMap specify different CIDs for a given character code" — and this
project had been reading that NOTE as a sentence about *metrics* for its whole life.

§9.5's NOTE 5 puts a substitute's shapes outside the standard, so neither half of the route is a
derivation and both are published tables read for what they say. **Which CIDs are vertical forms**
is the collection's own statement, from the Table 116 pair this binary has carried since ADR 0140:
`predefined::is_vertical_form` asks `UniJIS-UCS2-V` and `UniJIS-UCS2-H` and takes the CID only where
the two disagree about the character. **Which glyph is that form** is the face's own statement, from
OpenType's `vert`/`vrt2` features read straight out of `GSUB`. No new data, no shaping, and nothing
hard-coded about which characters rotate.

Both halves of the CID comparison earn their place, and ADR 0763 says why: without `V(ch) == cid` a
producer who wrote the *horizontal* CID under a vertical `CMap` would be rotated against its will,
and without `H(ch) != cid` every kanji on the page would qualify. Both were calibrated by removal.

## What moved, and it is one document

`examples/display_list_digest` over all 974 pdf.js documents and all 37 of `pdf-differences`, run
either side of the change in one sitting: **one line of 1012 differs, and it is `VerticalText.pdf`**
(71 commands both ways, 193 349 bytes to 192 773). The page now matches the corpus's own
`CorrectVertical.png` in structure, which was looked at rather than inferred.

`issue11555.pdf` is the corpus's only other substituted vertical Japanese page and did not move,
correctly: it shows `abc` and あいう through `90ms-RKSJ-V`, and that file states no mapping for ASCII
and none for those kana, so the collection says they have no vertical form.

## The warm-up, and two things found in writing it

Both of `doc/todo/03` §28's open per-case gates are in, and neither is quite the one-liner the item
predicted.

- **`IndexedCS_negative_and_high.pdf`.** Its README says the two rows of patches "should match
  exactly" and they do not: the reference row writes `0.95 0.5 1 rg` for a palette entry of
  `F380FF`, and 0.95 × 255 is 242.25 against 0xF3's 243, so three patches are a level apart in any
  renderer that rounds. The gate therefore reads the **upper** row against the file's own lookup
  string and §8.6.6.3, which is what "derived rather than voted" has to mean here. Calibrated by
  replacing `round()` with a truncation: it fails on the `6.5 sc` patch.
- **`InlineAbbreviations.pdf` is not the copy `doc/pdf.js` already carries.** Same 15 125 bytes,
  seventeen of them different, and every one inside a `/L` or a `/Length`: 1276 and 201 against
  `issue14256.pdf`'s 1240 and 197, and 1276 is where the `EI` actually is. So the two take different
  routes through `inline_image::data_extent` — one answered by §8.9.7's stated length, the other
  falling through to the first filter's own end-of-data — and the second test is a second witness
  rather than a duplicate.

## The second track

Seven ledger rows. §9.7.5.1 gains the NOTE it had read as metrics-only and the route that follows
from it; §9.7.5.2 gains the second question Table 116's data now answers, and the reason two of the
six collections have no answer (Adobe-KR is published with no vertical `CMap` and Adobe-Japan2 is
deprecated); §9.7.4.2 gains what the substituted route lost and no longer does; §9.7.3 stays
`inapplicable` and now names the one reader of `/Registry` and `/Ordering` and why `/Supplement` is
deliberately not read — the clause's own "This value shall not be used in determining compatibility
between character collections"; §9.2.4 gains a sentence separating its half of vertical writing from
§9.7.5.1's, which is the confusion this round was about. §8.6.6.3 and §8.9.7 gain the two new tests
and the two findings above.

## Gates

The full §2 sequence on a quiet machine: formatting (both workspaces), `clippy --workspace
--all-targets` and the `fuzz/` manifest under `RUSTFLAGS="-D warnings"`, the workspace tests, the
doctests, the sandbox worker and `pdfref-hayro`, the corpus gate, the oracle, the three extraction
gates, both censuses, `dates`, `xmp`, `jpeg2000`, `render-quorra`'s corpus gate, `fixed_documents`
and `cargo test -p conformance`. All green.

`doc/todo/00`'s step 7 was re-run whole over every page the oracle prints as `ambiguous`, from the
artefacts on disk: **19 at or past −1, 16 of them documents this tree calls incomplete**, and the
three complete ones are `issue16038.pdf`, `issue12295.pdf` and `issue14297.pdf` — the same three
names, to the thousandth, as the eight-hundred-and-sixth session's run. The alarm holds.

Three sweeps ran beside the sequence rather than during it — `pointers`, `quotations` and `parts` —
and none names anything this round added.

## What the next round might take

`doc/todo/21` §7 leaves four priced remainders and the sharpest of them is not the vertical route at
all: **a substituted face with no vertical form, and a substituted face with no glyph, are the same
silence**, and §3 has kept that as a number rather than a report since ADR 0152 on an arithmetic
that is now nine hundred sessions old. What would settle it is not a decision but an instrument —
the third thing ADR 0422 built for §5's band, a value on `Interpretation::shortfall` that crosses
the crate boundary without costing the oracle a judged page. The other three are a half-width
vertical pair with no witness, a `GSUB` lookup type nothing on this machine states, and two
collections Adobe publishes no vertical `CMap` for at all, which is closed rather than owed.
