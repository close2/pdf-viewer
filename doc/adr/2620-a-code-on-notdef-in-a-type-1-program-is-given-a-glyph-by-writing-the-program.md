# 2620 — A code on `.notdef`, or on no glyph, in a simple font's Type 1 program is given a glyph by writing the program

Status: implemented and measured (section 3). Session 1893 (a simple `Type1` font over `/FontFile`; ADR 2578 section 4's second item).
Context: ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1 and 6.2.10.9; ISO 32000-2
§9.6.5.2 and §9.9.1 (Tables 124 and 125); Adobe Type 1 Font Format sections 2.4, 5.12, 6.2, 6.4, 7.1–7.3; A694 item 3,
A48; ADRs 2578, 2556 (section 2).
Code: `crates/pdf-font/src/type1_glyphs_given.rs` (new, `with_glyphs_given`, `GlyphsGiven`, `MOST_BYTES_WRITTEN`) and
its tests; `type1.rs` (`Type1Error::NotWritten`, `Program::glyph_names`); `loading/drawing.rs`
(`LoadedFont::draws_notdef_in_place`); `crates/pdf-transform/src/archive/fonts/cff_notdef_glyph.rs` (`Format`,
`is_simple_type1`, `written`, `with_lengths`), `truetype_cmap.rs` (`CmapRestated::type1`, the history sentence),
`notdef_format.rs`; `tests/archive/type1_notdef_glyph.rs`; `fuzz/fuzz_targets/type1_write.rs` (new).

## 1. Decision

ADR 2578's construction, unchanged on the PDF side, for the third program a simple font embeds: the conversion appends
one `CharStrings` entry per glyph asked for and leaves the content stream and `/Widths` alone; the names and the
`/Encoding` restatement are ADR 2578 section 2's. The writer is a module of its own beside the reader. It decrypts the
`eexec` section (key 55665, its four leading bytes kept), finds `lenIV` and the `CharStrings` dictionary by a tokenizer
that skips every `<n> RD` blob, and refuses unless `read-fonts` reads the same names in the same order (but for
`.notdef`, which it swaps to index 0). A copy of `.notdef` is its charstring decrypted under `lenIV`, the width operand
of its opening `hsbw` or `sbw` restated, and encrypted again under key 4330 behind `lenIV` zero bytes (section 7.3
restricts no leading byte; a negative `lenIV` stores the charstring plain, as `read-fonts` reads it); a glyph that draws
nothing is `0 <w> hsbw endchar`. Each entry copies `.notdef`'s own `RD` and `ND` spellings, the dictionary's stated size
is raised by as many, the section is encrypted again, and the header and trailer are copied, so `/Length1` and
`/Length3` keep their values and `/Length2` is restated. A hexadecimal or PFB-segmented section, lengths that do not end
the header at `eexec`, and a program with no `.notdef` are refused.

**What the reader drew decides what the code gets, and for a Type 1 program it is not ADR 2578's split.** §9.6.5.2
substitutes `.notdef` for a name the program lacks, and `LoadedFont::outline` draws it (`substitutes_notdef`), so a
`/Differences` name absent from the program drew `.notdef`'s outline; such a code is given a copy of `.notdef` under its
own name (`draws_notdef_in_place`), where a bare CFF's reached nothing and drew nothing.

## 2. Bound and proof

At most `MOST_GLYPHS_GIVEN` glyphs and `MOST_BYTES_WRITTEN` (64 MiB) written, both checked before the section is built;
`lenIV` is bounded by `.notdef`'s own length, since its charstring must hold those bytes. **The read-back** (trap 301)
runs before the program is returned: this module's scan of the rewritten section finds every entry held byte for byte
and each new one decrypting to the charstring it was written from, and `read-fonts` reads the count, each new glyph's
name at its index, its advance and its outline (`.notdef`'s, or none). The caller then loads the font from the
in-memory objects and holds all 256 codes to ADR 2578 section 3's proof. **The plant**: the writer's charstring
encryption given `lenIV` 1 where the program states 4 — five of the eight writer tests went red, each on "the rewritten
program does not read back as it was written", and no program was returned.

## 3. Measured

- **Census**, against HEAD's, with and without the CJK pack alike: three pdf.js documents (`issue11477_reduced`,
  `issue6721_reduced`, `issue6901`) convert at all four lines (as-if-printed 2b 884 → 887 and 4 895 → 898 with the pack,
  883 → 886 and 894 → 897 without; as-shown 2b 821 → 824, 4 826 → 829 in both); the Type 1 rows of
  `fonts/embedded-programs-define-every-glyph-shown` (3 documents) and `fonts/no-notdef-glyph-shown` (1) are gone; corpora
  `504076.pdf` stays refused on the rows it already held and moves to R03a's line for another requirement. No document
  converts at HEAD and not here. `rank.py`: R03a frees 13 → 9 (as-if-printed) and 17 → 15 (as-shown).
- **veraPDF** 1.30: the six outputs (three documents at 2b and 4) are compliant; the planted source fails at 2b.
- **Fuzz** `type1_write`, 602 s in fork mode from the six writer seeds: 21 338 199 executions, 1 501 edges, 7 133
  features, 1 998 inputs, 0 out-of-memory, 0 timeouts, peak 0.08 GiB. Six crashes, all the target's last property (a
  written program written again): the second call's read-back refused, `read-fonts` reading no `CharStrings`. Five headers
  end `eexec\r`, where `read-fonts` follows FreeType's rule and skips the `\r` only if a `\n` follows somewhere in the
  data, so the section's start moves with the bytes the writer re-encrypts; one header opens a procedure it never
  closes, and the `eexec` it swallows was found only by a second copy later in the file. The writer wrote nothing wrong
  in either: section 2's read-back refused as designed. Named in the round's record, not fixed here.
- **Memory**, `getrusage` of `quorra-transform archive --to 2b`: a 251 516-byte file whose `.notdef` is 246 009 bytes and
  whose 255 shown codes reach it peaks at 796 MiB, 4.1 s, writing 63 MB of program (HEAD refuses it at 13 MiB); the same
  program held as 256 distinct glyphs, which HEAD converts, peaks at 373 MiB at HEAD and 372 MiB here. The bound is the
  writer's 64 MiB times the copies of a section the read-back and the proof hold.
