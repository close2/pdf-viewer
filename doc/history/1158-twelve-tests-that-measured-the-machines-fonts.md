# 1158 — Twelve tests that measured the machine's fonts, and two defects under them

2026-09-21. Files: `pdf-font/src/loading.rs`, `pdf-model/tests/` (`hostile_budgets.rs`,
`composite_fonts.rs`, `text_state.rs`, `substituted_shapes.rs`, `variable_text.rs`,
`vertical_forms.rs`), `.github/workflows/ci.yml`, `doc/habits/`, ledger, ADR 1154.

## The measurement
CI's two red pushes were both `hostile_budgets`; the workspace run with `/usr/share/fonts` under
an empty `tmpfs` and an empty `HOME` found eleven more, in seven binaries. §9.6.2.2's fourteen are
compiled in, so a **composite** font with no `/FontFile` is the only kind that depends on a
machine: §9.7.4.2 leaves it reachable only by character, which a name-keyed CFF cannot answer.

## Per test
- Seven — in `composite_fonts`, `text_state`, `hostile_budgets`, `variable_text` and
  `vertical_forms` — **need a face**: they skip with a sentence, via one `pdf-font` helper running
  the search the load runs, so a regression cannot become a skip. Eleven skip fontless with the
  two already there; two more in `pdf-vfs` and `viewer-confined` are about a broker: left alone.
- `silent_fonts`' §9.7.5.2 test **needed no skip**. `load_composite` searched the machine before
  asking whether §9.10.2 gave the codes a character; both refusals are one variant and only the
  second is the file's, so a reader with fonts was told `issue6127.pdf` broke the clause and one
  without was blamed on its own machine. Order reversed; nothing that loaded stops.
- `pdf-font`'s widths-against-advances test was a **defect in its population**: the compiled-in
  faces are bare CFF, so substitutes joined a population of producer-embedded programs — nine
  fontless, five with fonts, passing by luck. Filtered to `!substituted`.
- `substituted_shapes`' width test **asserted more than §9.2.4 gives**: a width is a displacement,
  so ink past it is a negative side bearing, and which letters do it is the face's business
  (`DejaVuSans`' `f` and `t`, the compiled-in `y`, none of this machine's). Now a share — a
  quarter of the line, worst 2 of 19 — controlled by `stretch`: unscaled, 14 or 15 of 19 are over.
- `composite_font_stating_ranges` draws `<034a>`, CID 843 where `Adobe-Japan1-UCS2` opens its
- `composite_font_stating_ranges` draws `<034a>`, CID 843 where `Adobe-Japan1-UCS2` opens its
  hiragana range: the character the face was *chosen* for. The old `<0001>` was CID 2, `!`, and
  `DroidSansFallback` covers あ with no Latin — the font loaded, the skip did not fire, nothing
  was drawn. CI's second red run.
## Verified
Every affected binary run with this machine's faces (nothing skips), fontless (eleven print a
skip), and in namespaces holding one candidate package — `DroidSansFallback` alone included,
where two `/ToUnicode` tests skip and the two `CMap` ones run. All pass in every arm.
`raster_golden` 974 held, 0 moved; `text_extraction` 11096/11131, ratchets at floor. CI names
`fonts-dejavu-core` and `fonts-droid-fallback`, which `fontconfig-config` and `ghostscript` bring
in already, as an alternative and a recommend — neither a guarantee.
