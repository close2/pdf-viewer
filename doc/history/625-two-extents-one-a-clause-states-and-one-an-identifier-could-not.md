# 625 — Two extents: one a clause states, and one an identifier could not

`doc/todo/03`'s chunk again, over the SafeDocs crawl, for the fifth round running. Ten whole
archives this time, and two defects — one where a clause states where a thing ends and a filter
was answering instead, and one where "where does this thing end" could not be asked of the
quantity that was asking it.

Date: 2026-08-20.
ADR: [0459](../adr/0459-two-extents-one-a-clause-states-and-one-an-identifier-could-not.md).

Touched: `crates/pdf-font/src/program.rs`, `crates/pdf-render/src/repeat.rs`,
`crates/pdf-model/src/content/pattern.rs`, `doc/conformance/ledger.toml` (§9.9, §8.7.3.1),
`doc/todo/03-more-corpora.md` §20, the ADR and this file.

## The chunk

**`0669`, `0915`, `1530`, `2391`, `3129`, `4113`, `5220`, `6204`, `7311`, `7926` — 10 000
documents**, none of the twenty-two sessions 603, 613, 615 and 619 ranked. An archive is a hash
bucket (ADR 0261), so any set is unbiased. 603's instrument **reused rather than rewritten**: page
one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box,
ranked by our ink minus the lightest live reference's, panel sizes beside each number. About two
minutes an archive.

**Checked before it was trusted, and the sandbox worker built explicitly** — 619's lesson, taken
rather than re-learnt: `cargo build --release -p pdf-model --example render_at` **and**
`cargo build --release -p pdf-sandbox --bins`. All **26** documents named by ADRs 0438, 0448, 0451,
0454 and 0456 reproduce their recorded numbers to the thousandth.

## What the two defects were

**`0669424.pdf` at −7.223** — a text page drawing nothing but its rules, reporting three fonts
whose `/FontFile2` "decoded only as far as its damage (Truncated, 87764 bytes)". 87764 is the
stream's own `/Length1`, exactly, and so are the other two files' 62548 and 61016; every table of
every directory ends inside those bytes. §9.9's Table 125 states `/Length1` in **decoded** bytes —
"the entire TrueType font program, after it has been decoded using the filters specified by the
stream's Filter entry, if any" — so a decode that reaches it has produced every byte of the
program, and what stopped short is the filter's end-of-data marker, outside it. ADR 0356's rule one
clause along.

**And the length is not the whole condition, which the corpus said before any gate did.** The first
version of the fix asked only about the extent, and `issue13316_reduced.pdf` — ADR 0343's own
witness — decodes to **168 808 bytes with `/Length1` 168 808** and draws **A C E F** where six CJK
glyphs belong. Its damage is `Corrupt`, not `Truncated`: RFC 1951's grammar violated at a definite
point, past which nothing is the producer's. `Truncated` is the encoded data merely running out,
and every byte it produced is what the producer's compressor emitted. Two conditions, and the one a
length test cannot supply is the one that mattered.

**`4113230.pdf` at −112.626** — the deepest row of the ten thousand, silent, 162 commands: a title
page filling one path with two full-bleed photographs in turn, of which this tree drew the first
and the references the second. Diagnosed by bisecting the content stream at unchanged byte length
(621's method): each fill draws alone, the second contributes nothing when the two paths are
identical, nudging either path by one unit restores it, replacing the second cell's image with a
flat rectangle changes nothing, and the display list holds both fills' commands either way. So the
loss is in `pdf_render::Cell`, and it is `DisplayList::add_clip` **interning**: the second cell's
`/BBox` — same rectangle, same matrix, same parent — was handed the first cell's identifier, which
sits below the mark the copier used to decide "already in force", so every site of the second
tiling kept the first cell's *first-site* box. That site is off the top of the page.

`Cell` is given the clip the tiling was drawn inside now, and a clip is the cell's own exactly when
it is none of the clips in force — that base and its ancestors. **The narrower reading, "does it
descend from the base", was tried first and the oracle caught it**: `issue8565.pdf` went newly
contradicted, because a soft mask's group is interpreted in a clip context of its own and its
clips are the cell's without descending from anything the tiling was given.

## What moved

Thirty-two archives, 32 000 documents, ranked whole before and after with the same instrument and
diffed row by row: **39 rows move**. Four are the head documents the two fixes are about, and two
of those four are in archives an earlier chunk ranked. Twenty-three are tiling-pattern pages
moving by at most 1.34 — eighteen toward agreement, five away by at most 0.64 — every one silent
and every one carrying more than one `PatternType 1`. The other twelve are the instrument: nine
have our own panel identical with a *reference* panel differing between runs, and three had a
panel absent from the earlier run.

Six documents move on the font fix, all from a deficit to agreement — `6942406.pdf` −15.171 →
−0.033, `6696243.pdf` −8.218 → +0.058, `0669424.pdf` −7.223 → +0.181, `4100967.pdf` −6.410 →
+0.112, `7680832.pdf` −2.645 → +0.212, `3990014.pdf` −0.551 → −0.080 — and **two of the six are in
archives an earlier chunk ranked**, `6696` (615's) and `7680` (603's). `4113230.pdf` −112.626 →
−0.103 on the tiling fix.

**The population behind the font fix is measured over the whole crawl with an instrument that is
not this tree's** (trap 8): a Python walk over all 65 944 documents finds **140 embedded font
streams in 8 documents** whose Flate data ends before RFC 1951's final block, **every one of the
138 `/FontFile2`s reaching its `/Length1`**, and the two `/FontFile`s falling short of
`/Length1 + /Length2 + /Length3` — so `7557616.pdf` stays refused, which is the Type 1 arm
exercised in the negative direction by a real document.

Each fix is pinned by a test **run against the defect first** (trap 13), and two of the four are
negative twins a length test alone would pass: a program written as one non-final RFC 1951 stored
block, with `/Length1` reached and with it one byte short; the same block followed by a reserved
block type, so the decode reaches the length and *then* meets a grammar violation; and two cells in
succession whose boxes the table interns, asserting the interning before asserting the copy.

## What the head still holds

**`7926872.pdf` at −41.731** is `pdf_model::inline_image`'s own module comment coming true: answer
3, the forward search for `EI`, is "the one guess in the module", and this image's first `EI` token
stands 24 822 bytes into 2.9 MB of Flate. §8.9.7 makes the bytes a stream object's data and every
filter it admits states its own end-of-data, so a *filtered* extent is derivable rather than
searchable — `pdf_syntax::Pump` already counts consumed input on its Flate engine and does not
expose it, and `DCTDecode` and `CCITTFaxDecode` do not go through it at all. A round of its own,
named in `doc/todo/03` §20.

**Five silent rows diagnosed no further than their numbers**, each named there so the next round
does not re-derive them, and **65 rows of the 10 000 produce no number** — the same three shapes
613, 615 and 619 opened by hand.

## Gates

The full §2 sequence, because the change is in `pdf-font` and `pdf-render`.
`RUSTFLAGS="-D warnings"` on the clippy line, which caught three lints this round's own code
introduced, and the conformance gate caught a `§` written in front of an RFC section number.
**Two gates failed on the way and both were the round's own work**: `silent_fonts` on the first
version of the font condition, and the oracle on the first version of the clip predicate. §5's
binaries were **not** rebuilt: this is not a fifth round and nothing on the launch path was
measured.
