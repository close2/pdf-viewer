# 2807 — A composite font's machine face the window drew is embedded as a `CIDFontType2` subset

Session 1998, a single archive round.
Status: accepted and built. In `pdf-transform`: `archive/fonts/machine_cid.rs` (new),
`archive/fonts/composite.rs` (`unnamed` and `embed` take `embed_machine_face`), `archive/fonts.rs`
(the call), `rekeyed.rs`'s `descendant_object` and `cid_to_gid_map` and `described.rs`'s `resolved`
and `same_once_wound` shared with it.
Tests: `crates/pdf-transform/tests/archive/machine_cid.rs` (two); `archive/cjk_pack.rs`'s refusals
now authorise `machine-face`, the profile under which the pack is the route taken.
Context: ADR 2773 (the simple-font construction and its proof), ADR 2774 (which profile takes it),
ADR 2302 (the CJK pack), ADR 2378 (re-keying an `Adobe-Identity` font), ADR 0270 (the window's
substitute), `doc/pdfa-blocking-issues.md` R09 (a).
Clauses: ISO 32000-2 §9.7.4.2, Table 115 (`/CIDToGIDMap`), Table 117, §9.7.4.3, §9.7.5.2, §9.9.1,
§9.9.2, Table 124; ISO 19005-2 sections 6.2.11.3.1, 6.2.11.3.2, 6.2.11.4.1, 6.2.11.5; ISO 19005-4
sections 6.2.10.3, 6.2.10.4.1, 6.2.10.5.

## Context

ADR 2773 embeds the machine's face the window drew for a simple font. A composite font the file
does not embed took the CJK pack (a collection's font) or the re-keying route (an `Adobe-Identity`
font with a `/ToUnicode`), both of which embed a face other than the one the window drew, which is
the loss `machine-face`, so `as-shown` stopped on every such document. On this VM the window draws
these fonts from `DroidSansFallbackFull` (`glyf`, 49 382 glyphs, 4.0 MB, `OS/2` `fsType` 0x0008,
editable embedding, subsetting not forbidden), read from the face itself before anything was built.

## Decision

1. **Ahead of the pack and re-keying, where the profile withholds `machine-face`** (ADR 2774's
   switch), the face the window drew goes in: the bytes it loaded (`substitute_program`), never a
   second search, and only where its `fsType` permits embedding (§9.9.1), it is not a face this
   program ships, not a collection, not `CFF ` and not fitted.
2. **A `glyf` subset under `/FontFile2`, the descendant restated as `CIDFontType2`** (Table 124,
   Table 117), named with §9.9.2's tag, by `pdf_font::embed::for_embedding`, which keeps every
   glyph a shown CID reaches, `.notdef` and the components they name. The face is not carried
   whole: four megabytes for a page that shows a dozen characters is not what §9.9.2's subset is
   for, and a `glyf` subset needs no compacting writer.
3. **A `/CIDToGIDMap` stream** sends each shown CID — the predefined `CMap`'s (§9.7.5.2) — to the
   subset's index of the glyph the window drew for it (`LoadedFont::glyph_index`), so a vertical
   form the window chose is the glyph written (§9.7.4.2, Table 115; ISO 19005-2 section 6.2.11.3.2
   requires the entry). The `CMap`, `/CIDSystemInfo` and `/ToUnicode` are untouched, so section
   6.2.11.3.1's agreement holds as before.
4. **Advances restated to `/W` and `/DW`, and vertical ones to `/W2` and `/DW2` where the program
   states `vmtx`** (§9.7.4.3; ISO 19005-2 section 6.2.11.5, ISO 19005-4 section 6.2.10.5), then
   read back (`proves`).
5. **Proved before it is written**: the font as it will stand is assembled into a document of its
   own and held to the window code by code — the outline once wound one way, the advance and the
   vertical metrics bit for bit, and a program of its own.
6. **What any condition does not admit takes the route it took before**, and the reason is not
   carried: that route names `machine-face`, which the same profile stops on, so nothing drawn is
   lost in silence.
7. **The loss left**: none for a collection's font, whose CIDs reach characters through Adobe's
   published map; `glyphs-from-to-unicode` for an `Adobe-Identity` font, whose glyphs the window
   chose through an extraction aid. `as-shown` withholds that loss too (A570), so those documents
   stay refused; whether `as-shown` should admit it where the output draws exactly what the window
   drew is the owner's (`doc/questions/Q832`).

## Consequences

On this VM the window drew `DroidSansFallbackFull` for every collection font of R09. Four documents
convert under every profile that withholds `machine-face` (`as-shown`, `refuse-any-loss`,
`no-data-loss`, `keep-everything`, `only-metadata-loss`, `derive-attachments`), at both parts:
`issue8372`, `noembed-eucjp`, `noembed-jis7`, `noembed-sjis`. `as-if-printed` and
`state-every-loss` keep the pack's face. A machine with other faces writes other faces, which is
what `as-shown` means (ADR 2773).

Left of R09 (a), each falling back to the route it took before:

- **A shown CID the window drew no glyph for** (`issue11555`, `issue13343`, `issue15053`,
  `issue19182`, `issue6286`, `mixedfonts`, `issue2128r`): the window drew nothing there, and the
  only glyph a `/CIDToGIDMap` could send it to without adding one is `.notdef`, which ISO 19005-2
  section 6.2.11.8 forbids. Keeping the absence needs a `glyf` writer that adds an empty glyph at
  the CID's advance, a `pdf-font` change.
- **A descendant with no descriptor object** (`90ms_rksj_h_sample`), which needs a descriptor read
  off the face, as the pack route builds one (ADR 2461).
- **The four `Adobe-Identity` documents**, on Q832; and the fonts with no descriptor
  (`described`), which also lose `layout-from-this-viewer` and so would not move under `as-shown`
  by this construction alone; and a `CFF ` machine face for a composite font, with no witness.
