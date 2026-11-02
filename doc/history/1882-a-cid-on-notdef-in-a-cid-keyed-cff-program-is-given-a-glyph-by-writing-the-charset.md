# 1882 — a CID on .notdef in a CID-keyed CFF program is given a glyph by writing the charset

Session 1882, Slot 2 of batch 129, an archive round in two sessions; ADR 2598 used, ADR 2599 and Q758 unused. I worked
in the shared worktree and counted in an export of HEAD plus my own hunks.

**Built.** R03a for a `CIDFontType0` over `/FontFile3 /CIDFontType0C` (ADR 2598): `pdf_font::cff::with_cids_given`
appends a copy of glyph 0's charstring per CID at its advance under glyph 0's Font DICT, restates the charset with the
CIDs, `FDSelect` in the source's format 0 or 3 and the `FDArray` beside it, raises `CIDCount`, and reads every table
back through `read-fonts`; the archive answers every `Type0` font over one descendant together and proves each. The
name-keyed writer passes no `FDSelect` and writes the same bytes. `cff_write` routes CID-keyed programs to it.

**Premise.** The count held (four CID-keyed CFF documents among R03a's); what frees them did not. `issue4650.pdf`,
`issue11718_reduced.pdf` and `PDFBOX-3062-005717-p1.pdf` show their own CID 0 through `Identity-H`, which ADR 2557
keeps refused; they now carry that reason by name. `issue12963.pdf` is cut short by the checker's list of 32 places
(199 on glyph 0) and is Slot 4's document: with the list lifted it converts at both targets under both profiles.

**Plant.** An `FDSelect` format 3 whose sentinel is one glyph short is refused by the read-back; no bytes leave.

**Gates.** Census in the export, no-pack and with a copy of the pack, under `bounded.sh`: exit 0 and 0 (473 s, 3.75
GiB peak; 325 s, 1.64 GiB); against HEAD's every count line identical, 3 documents moved reason under 4 headings each
(12 lines a log), none moved status; `rank.py` exit 0: the TrueType `.notdef` reason 10 to 7 documents, the CID 0
reason of a `CIDFontType0` 0 to 3. `issue12963` with the list lifted (export experiment, reverted): veraPDF passes the
four outputs and fails the source on 6.2.11.8-1; its 8 pages render with 0 differing pixels. `cargo nextest run -p
pdf-font` 0 (330 of 330). `cargo test -p pdf-transform --test archive --test profiles` 101: 506 passed, 5 failed, none
mine (two `cid_notdef_glyph` tests on Slot 3's new reason sentence, two `whole_population` tests of Slot 4, one
transient). Clippy `--no-deps -D warnings` on `pdf-font` and `pdf-transform --all-targets` 101 on siblings' code only
(`config.rs` `read`, `recovered.rs` `build` past 100 lines, `tests/archive/recovered.rs` arithmetic); mine 0. `rustfmt
--edition 2024 --check` on my files 0; clippy on `cff_write` 101 on a sibling's line only. `--bin cited` 0, `--bin
quotations` 0. `cargo test -p conformance --no-fail-fast` 101: 67 binaries passed, 2 failed on Slots 5 and 7's
records. `cff_write` 600 s under the bound in fork mode: 5.66 million runs, 0 failures in the CID-keyed branch; 335 in
a sibling's new advances branch and 1 in HEAD's name-keyed branch (HEAD fails it the same way). `getrusage` beside
HEAD on 30 000 glyphs, 30 000 `FDSelect` ranges and 35 000 CIDs: HEAD refuses in 0.3 ms at 12 160 KiB; here 35 000
glyphs are added in 16 ms at 12 356 KiB, and one past 65 535 is refused in 1.3 ms. No pixel of the viewer can move, so
the six arms were not owed.

**Found, not fixed.** `crates/pdf-font/src/cff.rs` `with_glyphs_given`: a source read as name-keyed whose Top DICT
holds an `ROS` entry is written into a program the second pass reads as CID-keyed (HEAD too).
