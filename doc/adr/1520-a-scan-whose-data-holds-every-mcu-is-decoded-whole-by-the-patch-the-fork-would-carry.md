# 1520 — A scan whose data holds every MCU is decoded whole by the patch the fork would carry

Status: accepted. Session 1342. Amends nothing; keeps ADR 1495 section 2's cut-plan decline and
ADR 1513 section 2's restart plan. Adds a second patch to the question `doc/questions/Q227` asks.
Context: ISO 32000-2 §7.4.8; ISO/IEC 10918-1 (ITU-T T.81, `doc/md/T.81.md`, cited by section and
paraphrased) sections B.2.1, E.2.3, E.2.4; `CLAUDE.md` principle 5; ADRs 1459 (a guard that waits
on a fork), 1495, 1513.
Code: `doc/patches/zune-jpeg-scan-complete-without-eoi.patch` (against `zune-jpeg` 0.5.15 as
crates.io publishes it, the `Repository:`/`Base:` preamble `tools/main-checkout.py` reads).
Tests: `crates/pdf-model/tests/banded_decodes.rs`'s
`a_complete_last_row_without_its_eoi_is_grey_until_the_fork_takes_the_patch`, and the frame it shares
with `a_scan_the_data_ends_inside_is_left_to_the_whole_decoder` (`grey_frame_without_its_eoi`).

**`crates/render-cpu/` was not opened** (`doc/questions/A76`).

## 1. The premise, checked

The brief named `restart/hv_truncated.jpg` as ADR 1495's fixture. That frame decodes correctly
today: its bands equal its whole decode (ADR 1513 section 2) and the patched and stock decoders give
it the same bytes. The frame the defect is on is the other one ADR 1495 found, the 348-byte grey
100 × 107 frame held inline in `banded_decodes.rs`: decoded whole without its `EOI`, lines 104 to
106 — its last MCU row — are 128; with `FF D9` appended they are the frame's.

## 2. Why the row is grey, and what is correct

Section E.2.3 ends a scan when its intervals are decoded and section E.2.4 decodes an interval MCU
by MCU; the scan is complete on its MCU count, and the missing `EOI` (section B.2.1) is a missing
marker after a complete scan — the reading ADR 1513 section 2 already rests on. All 182 blocks are
in the data. `zune-jpeg`'s bit reader refills ahead of the decoder, and the refill that reaches the
end of the data turns `overread_by` positive; the MCU loop fills the rest of the frame with 128 at
the next row whose start finds `overread_by > 0`, whether or not a bit past the data was consumed.
On this frame the lookahead reaches the end while the last row's bits are still in the buffer.

## 3. The patch

The bit reader counts the zeros it appends once the data has ended (a byte read when the reader
was already at the end, and the 32-bit zero fill after it), and sets a flag where `get_bits` takes
more bits than the buffer holds (its `wrapping_sub` becomes `overflowing_sub`). The loop stops a row
only once the decoder has consumed past the data: `overread_by > 0` and either the flag or more
appended zeros than bits left. `drop_bits` already saturates, so it needs no flag; a restart's
`reset` discards the buffered zeros with the buffer.

Measured on a scratch copy against stock, both lenient and strict: the reproduction decodes to the
`EOI`'d frame; 27 complete frames (the tree's fourteen `cjpeg` fixtures and thirteen photographs
on disk) decode to the same bytes; over every truncation of fifteen fixtures the patched decoder
never gives fewer rows equal to the whole frame's (it gives more at up to 65 truncations of a
fixture, and at none of six), and the most rows that are neither correct nor 128 at any truncation
is the same.
A first version without the flag decoded twice as many such rows: `get_bits` wraps `bits_left`
past zero and a wrapped count can fall back under the zeros' share. Flagging `drop_bits` as well
cost 4.1% of a 4 MB photograph's instructions; the flag in `get_bits` alone, 510.2 M → 509.0 M.
Applied to a scratch tree through `[patch.crates-io]`: `banded_decodes` passes but for the guard, which
fails as written to, and the two decodes are equal.

## 4. In the tree

The patch is not applied: the tree has no fork of `zune-jpeg`, and whether it carries one is the
owner's (`doc/questions/Q227`, which gains a dated paragraph naming this patch). The test holds the
current grey row by name; the day the fork takes the patch its first assertion fails, and the guard
is deleted for the frame's equality. Unlike the first patch, this one changes samples a release build
produces — on frames whose data ends without `EOI` and only there. Once it lands, the cut plan's
decline of a scan with no `EOI` (ADR 1495 section 2) can be reconsidered, since the whole decoder it
is held to will then read the tail as the restart plan's last band does.
