# 0962 — A gitignored directory is part of the oracle's population, and a neighbour writes into it

Status: accepted. Session 945.
Context: `crates/pdf-model/tests/oracle.rs`, `doc/traps/oracle-and-references.md` trap 9,
ISO 32000-2 §9.5 NOTE 5 and §9.8.1.

## What happened

The oracle failed this round with `1 page(s) newly contradicted by the reference consensus:
["ICC.1-2022-05.pdf page 1"]`, and the round it failed on had not moved a pixel. `corpus_items`
walks `doc/` for page one of every specification this project holds; `/doc/*.pdf` is line 44 of
`.gitignore`, so those files are local, untracked, and fetched by whichever round needs to read a
standard. Six of them landed on 2026-09-09 and -10, four of them ICC's. The judged population grew
by six pages: **agrees 985 → 990, contradicted 61 → 62, ambiguous 835 unchanged.**

So the ratchet's own message is misleading by construction here. It says *newly contradicted*,
which in every other case means a change this tree made, and its list is the only place the
difference shows. Before spending anything on the page, the check that settles authorship is
cheap and was run: the page states no mesh (`examples/mesh_triangle_census` over the one file), and
the round's own render of it with this branch's `mesh.rs`, `shading.rs` and `pattern.rs` against
the same three files at `HEAD` is **byte-identical**.

**The standing figures in a round's instructions are therefore a figure about a population two
other things can change**: what this tree draws, and which specifications happen to be on the
disk. Both are worth knowing; only the first is a regression.

## The page, and it is trap 9's sixth bullet with the camps split

The failing bound is the **worst tile alone** — 45.70 against 40.00 — with mean 2.64 of 5.00,
differing 2.61% of 5.00% and structural similarity 0.9652 of 0.9000 all inside. The worst tile is
at (256, 96), which is the `International Color Consortium®` wordmark, and `pdffonts` says what
that is: `Arial-ItalicMT`, **not embedded**, beside a non-embedded `ArialMT` and `Arial-BoldMT`.
Everything else on the sheet is an embedded Calibri or Arial subset.

Ink over the wordmark's own box, in levels of 255 at the page's own scale
(`-crop 380x110+230+60 -alpha off -channel R`, the gate's own artefacts):

```text
  ours          21.906
  poppler       21.878
  mupdf         22.994
  ghostscript   23.037
  hayro         23.160
```

Two camps, 5.3% apart, and each camp is a *foundry*. `ghostscript` names its own when it is not
passed `-q`: *Loading font Arial-ItalicMT (or substitute) from
/usr/share/ghostscript/Resource/Font/NimbusSans-Italic*, with `ArialMT` and `Arial-BoldMT` going
the same way; `mupdf` carries the URW set compiled in. `fc-match Arial:italic` on this machine
answers `Arimo-Italic.ttf` — the Liberation design under its Chrome OS name — which is where
`poppler` lands asking fontconfig for the document's own name, and Liberation Sans is what this
tree carries in `data/standard-fonts/`. The metrics agree and the weight does not, which is what
metric-compatible means: at 8× the wordmark's ink spans **3839 device columns for us and for
`poppler` against 3844 and 3843** for the two URW programs, 0.13% over 3839 columns, while the ink
inside that span differs by 5.3%.

`bug847420.pdf` in the same group is the case where **all three** voting references drew one face
and the tell was that they agreed to the pixel (ADR 0772). This is the same mechanism with the
references split two against one, and the consensus that convicts us is the pair that share a
foundry rather than a reading.

§9.5 NOTE 5 settles it and is quoted at the head of that group's note: the results "depend on the
availability of fonts in the PDF processor's environment". Nothing in ISO 32000-2 chooses between
URW's Helvetica clone and Liberation's, and a bound that convicts a reader for holding the second
is measuring this machine's font directory.

## Decision

`ICC.1-2022-05.pdf page 1` joins `CONTRADICTED_SUBSTITUTED_FONT` with the measurement above
written into the group's note, not on the group's membership rule — that group's own note records
five pages admitted on the rule and never opened, and calls it "the seventh time this file has
caught a group's name naming a hypothesis rather than a diagnosis". The failing bound is named in
the note, which is what `--bin unpriced` asks of a contradicted page's group.

## What is left, and it is not this round's

The other five pages agree, so nothing is owed on them. What is owed by nobody in particular is
the more general point: a round that fetches a specification into `doc/` changes another round's
gate, and nothing in the tree says so. Writing that down here is the whole of what this ADR can do
about it — the alternative, taking `doc/` out of the oracle's population, would lose twenty-five
real documents from the judged set to make a message less confusing.
