# 621 — Two bounds, and a diagnosis that was a guess

Session 615's two leftovers, both filed under "images", and neither turned out to be the thing it
was filed as. One was a decoder's constant, taken as a commit because no release carries it. The
other was two pages called trap 9's family from their dictionaries, of which one is and one was a
silent defect of this tree.

Date: 2026-08-20.
ADR: [0456](../adr/0456-two-bounds-one-belonging-to-a-decoder-and-one-to-the-page-underneath-a-mask.md).

Touched: `Cargo.toml`, `Cargo.lock`, `deny.toml`, `crates/pdf-sandbox/src/decode.rs`,
`crates/pdf-model/src/content/transparency.rs`, `crates/pdf-model/tests/soft_masks.rs`,
`doc/conformance/ledger.toml` (§7.4.7, §8.7.2, §11.6.5.1, §8.6.5.8),
`doc/traps/oracle-and-references.md` (trap 9),
`doc/todo/_image-codecs-and-the-sandbox.md` §7, `doc/todo/03-more-corpora.md` §18, the ADR and this
file.

## The release that is a commit

`hayro-jbig2` 0.3.0's flat 10 000 symbol-instance cap costs three crawled documents a page each, in
silence — a refusal inside the worker is one image, so the page draws around the hole. Upstream's
replacement is `1be7ab10`; `cargo search` still answers 0.3.0 and the crate has two tags, so there
is no release to take. There did not need to be a branch either: the commit is in `upstream/main`
and inside `64efcaca`, the revision already pinned for `hayro-jpeg2000`. `hayro-ccitt` moves with it
because `hayro-jbig2` names it by path, and brings #1304's unified `push_pixels` with it, which the
packer in `pdf-sandbox` now splits itself.

`1653119.pdf` −35.695 → +0.012, `3375154.pdf` −16.417 → +0.032, `3252105.pdf` −6.390 → −0.215, and
the first was a blank sheet where three references drew a whole broadsheet. **The next bound did not
appear** — nothing is reported on any of the three, before or after — which was the thing to check
rather than assume, because that is exactly what 615 found last time a ceiling came off.

## The diagnosis that was a guess

Two rows below −8, both handed over as trap 9. The instrument that separated them is one PDF: four
objects, a strip of colour patches in the space under test, rendered by all four programs. It cost
minutes and it answered both.

`5589519.pdf` was filed as `/DeviceCMYK` JPEGs. On plain `DeviceCMYK` patches this tree and
`poppler` produce *identical* values, so the conversion was never the question. `pdfref-hayro` — a
fourth interpreter sharing no colour code — agreed with the other three about the page, and a
bisection of the page's own 51 105-line content stream, rebuilt at the same byte length so every
offset stayed valid, named one operator: a photograph drawn under a luminosity mask whose group
paints a shading pattern. §8.7.2 anchors that pattern to "the form coordinate space at the time the
form is painted", §11.6.5.1 says what that is for a mask, and `build_soft_mask` was running the
group without swapping the interpreter's `base` — so the gradient landed in the page's default space
and the page's own green washed up through the photograph. Two lines, the same two `draw_xobject`
has had since the clause was first read. **The fourth way of becoming a parent content stream, and
the fourth time §8.7.2's row was short by one.** −8.212 → +0.713.

`6696954.pdf` *is* trap 9, and the mechanism is new: not shared code and not shared data, but a
shared **default argument**. `objdump -p` says `libpoppler` and `libgs` link one `liblcms2.so.2` and
`libmupdf` defines 445 `lcms2mt_*` symbols of Artifex's fork, so on an `ICCBased` page the three
voting references are one colour library; `INTENT_PERCEPTUAL` is 0, which is what a caller passing
nothing passes. This tree's own evaluator pointed at the document's own profile reproduces
`poppler` byte for byte through `A2B0` and sits twenty levels away through `A2B1` — and Table 51,
§8.6.5.8 and §11.4.7 each say the default intent is RelativeColorimetric, the third of them for the
page group this document actually has. The page stays contradicted with the evidence written down.

## What moved

**Nothing a gate can see.** The oracle's 1794 per-page lines are byte-identical with and without the
mask fix, run both ways — so `doc/todo/00` step 7's sweep cannot move and no ambiguous page's ink
changed. `examples/raster_digest` over the 125 pdf.js documents naming any of the three sandboxed
codecs is byte-identical before and after the dependency change, all 125 lines. Both defects are on
the crawl, where no ratchet reaches.

## Gates

The full §2 sequence, because the change is in `pdf-model` and `pdf-sandbox`. `fmt` silent;
`RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` silent (three real errors on the
first run, all mine); `nextest` 2284 passed / 16 skipped; doctests green. Corpus 974 documents, 68
incomplete. Oracle 1794 pages: 907 agree, 66 contradicted, 786 ambiguous, 2 our geometry, 2
reference geometry, 13 not comparable, 18 no render. Text extraction 10969/11163 words in bounds
over 508 documents. `selection_census`, `accessibility_census`, `dates`, `xmp`, `jpeg2000` and
`conformance` green; `render-quorra` 957 pages, 932 agree / 23 differ / 2 refused. `cargo deny`:
`bans`, `licenses` and `sources` ok, `advisories` failing on a yanked `arrayref` reached through
`tiny-skia`, which is not this round's and is unchanged in the lockfile diff.

§5's binaries were **not** rebuilt: this is not a fifth round and nothing on the launch path was
measured.
