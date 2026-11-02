# 2648 — A Font DICT's offsets are shifted with the bytes they name, and a name-keyed `FDArray` is rewritten

Session 1907, Slot 7 of batch 131, 2026-11-02.
Status: implemented.
Context: ADR 2628 (its Open section: `font_dict_shifted` shifted a Font DICT's `Private` alone, so
the fuzz artifact `64656d0d`'s Font DICT `charset 114` landed inside the re-encoded String INDEX),
ADR 2649 (the same session's local-subroutine walk). Code: `crates/pdf-font/src/cff.rs`
(`dict_restated`, `font_dict_shifted`, `Layout::with_glyph_tables`, `Layout::with_charstrings`),
`crates/pdf-font/src/cff_stop_tests.rs`.

## Decision

1. **A Font DICT is restated with the Top DICT's operators.** Adobe Technical Note #5176 gives a
   Font DICT no operator table of its own: section 18 says only that each Font DICT "will specify a
   corresponding Private DICT", and Table 9 marks `charset`, `Encoding`, `CharStrings`, `FDSelect`
   and `FDArray` as offsets "(0)", from the start of the CFF data. An offset of those operators in
   a Font DICT therefore names the same bytes the Top DICT's would, and moves with them when the
   String INDEX or the Top DICT INDEX in front grows. The Top DICT's restatement became the free
   function `dict_restated(entries, moved)`, and `font_dict_shifted` runs it with every offset
   moved by `delta`; a `charset` or `Encoding` operand naming a predefined table stays as stated.
   Shifting, not refusing: each such offset has one reading, so a refusal would lose a program
   the reader opens for a reason the bytes do not give.
2. **A name-keyed program's `FDArray` is rewritten where it reads.** It was copied whole, its
   Font DICTs' `Private` offsets unshifted, so a reader that follows a name-keyed `FDArray` met
   bytes `delta` away. A CID-keyed program's `FDArray` must still read, else the write is refused
   as before; a name-keyed one that does not read is copied as it was.
3. **The premise named section 19**, which is `FDSelect`; the Font DICT is section 18's, and the
   doc comments cite Table 9 and section 18.

## Measured

- `a_font_dict_offset_is_shifted_with_the_bytes_it_names` is the minimised `64656d0d` shape (the
  artifact was not kept on disk); with the shift taken out it fails (`79 == 79`).
- One fork-mode run of `cff_write` with `-ignore_crashes=1`, `-ignore_timeouts=1`, `-ignore_ooms=1`
  on this code (built from an export that differs from it in two doc comments only): 634 s,
  1 305 105 executions in 34 jobs, 0 crashes, 0 timeouts, 0 out-of-memory stops, no artifact;
  coverage 2 389 edges, corpus 1 620 inputs from 65 seeds; peak 0.11 GiB resident.
- `getrusage` maximum resident size under the hostile fan-out fixture (`a_fan_out_of_five_at_ten_levels`):
  HEAD 12 032 KiB, this code 12 576 KiB.
- The archive census, no pack, is line for line HEAD's but for its timing figures, and `raster_golden` and
  the six corpus arms are page for page HEAD's (frame digest and mean error): the Font DICT repair
  on the loading path, which runs the walk, moved no pixel.

Rejected: refusing a Font DICT that states any offset but `Private`. Section 18 does not forbid
the others, and the reader opens such a program.
