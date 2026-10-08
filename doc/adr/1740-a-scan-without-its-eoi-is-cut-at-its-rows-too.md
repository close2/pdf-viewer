# 1740 — A scan without its `EOI` is cut at its rows too

Session 1452. Status: **accepted** and **built**. Amends ADR 1495 section 2's "both plans now
decline a scan with no `EOI`" for the row plan, the half ADR 1513 left: `image::cut` no longer
declines a scan the data ends without `EOI`. Supersedes nothing. Context: ISO 32000-2 §7.4.8;
ISO/IEC 10918-1 (ITU-T T.81, cited by section, paraphrased) sections B.2.1 and E.2.3; ADRs 1481,
1495, 1513, 1520, 1730; trap 13. Code: `crates/pdf-model/src/image/cut.rs` (`decode_at`, the module
comment), `image/restart.rs` (the module comment's sentence about `cut`). Tests:
`crates/pdf-model/tests/banded_decodes.rs`'s
`a_complete_scan_without_its_eoi_is_cut_at_its_rows_and_is_the_whole_frame` (which replaces
`a_scan_the_data_ends_inside_is_left_to_the_whole_decoder`, a test that pinned the refusal) and
`a_scan_without_its_eoi_is_never_cut_differently_at_its_rows`. Row: §7.4.8, which stays
`departed` on Table 13's default alone.

## 1. Why the decline stood, and why it no longer has to

The row plan's last band is the frame's last rows re-coded as a codestream of its own and ended by
an `EOI`; the whole decoder reads the data as it stands. Section E.2.3 ends a scan on its count of
MCUs, so a scan whose data holds every MCU is complete without the marker, and the pass already
declines a scan whose data ends before its last MCU (`pass` checks that its last bit lies inside the
data). What made the two readings differ was the decoder: `zune-jpeg` 0.5.15 stopped at the row
after its lookahead reached the end of the data and filled that row with 128 (ADR 1520). The fork
the manifest pins carries `doc/patches/zune-jpeg-scan-complete-without-eoi.patch`, which stops a row
only once it has consumed past the end, and round 1447's
`a_complete_last_row_without_its_eoi_is_decoded_as_the_frame` holds the whole decode equal with and
without the marker (ADR 1730). So where the pass admits a scan, the band plan and the whole decoder
read one frame, and the decline is lifted; where it does not, the frame is the whole decoder's.

## 2. The evidence

- **The fixtures, every way the tail can end.** Each of the four row-cut fixtures, its `EOI` taken
  off, is cut and equals the whole frame; ended at every byte of its last 1 024 it is refused or
  equal, never cut where the whole decoder refuses, and refused at least once; carried on by
  `00`, `FF`, `FF 00`, `12 34 56` or `FF FF FF` it is refused or equal. ADR 1495's 100 × 107 grey
  frame is cut at 8, 16 and 64 lines and equals its decode with the `EOI` appended.
- **Calibrated (trap 13).** With the decline put back, both new tests fail; with the pass's
  end-of-data check taken out, the truncation test fails on a band that is not the whole frame.
  Both plants were patches of this round's, applied and reversed.
- **The corpus.** The 179 `DCTDecode` codestreams `pdfimages -j` takes out of the 974 tracked
  documents, each decoded as it is and with its `EOI` taken off, in bands of 16 lines below the
  floor: the row plan cuts 57 with the marker and the same 57 without it, every one the whole
  frame; the whole decoder reads 134 without the marker, each byte for byte its reading with it;
  none is cut where the whole decoder refuses. With the decline planted back the 57 without the
  marker fall to 0, so the census sees the lift. The census was a test of this round's, run and
  deleted.
- **No page moves.** A cut answers the whole decoder's bytes or declines, so the change is where a
  frame is decoded, never what it is; `raster_golden` holds all 974 first pages, 0 moved.

## 3. What is left

Nothing of §7.4.8: the row's one departure is Table 13's default of 1 beside `R`, `G`, `B`
identifiers (ADRs 1183, 1622), and how a frame is scheduled is not one, which the row's note now
says.
