# 1440 — A Lab colour is read under its own white point, and a JPX Lab under its illuminant's

Slot 3 of batch sixty-nine, 2026-10-08, a clause round and the batch's `partial`-row work. ADRs
1712 and 1713; Q336 not used. §8.6.5.4 stays `implemented`, its note now true; §7.4.9 stays
`partial` on PIMA 7667 and CIE 131 alone. **Premises.** All held: Table 64's `/WhitePoint` is
"(Required)" with no default, `Lab { range }` read none, `lab()` used D50.

**§8.6.5.4 (ADR 1712).** `Lab` carries `white` and `black`; the second stage multiplies by the
white, then Bradford onto D50 as for the Cal spaces. The EXAMPLE's D65 space is the fixture: two
chromatic colours move more than a level, three neutrals stay put; the D50 plant fails it.

**§7.4.9 (ADR 1713).** `jpeg2000::Illuminant` reads `IL` per T.801 Table M.29. D50 is T.4 E.6.4's,
D65 the EXAMPLE's, and six are CIE sums from `data/cie/` (CC BY-SA 4.0), recomputed by a test. `CT`
is the Planckian radiator's: CIE S 017 entry 17-23-067 defines colour temperature so, and the note's
"names no family" did not hold. `planckian.rs` puts 2856 K within 2e-4 of illuminant A. Three
fixtures; `redact.rs` writes the white it read.

**Witnesses.** `PDFBOX-3599-0.pdf` and `poppler-LINK-613-0.pdf` put a D50 `/ColorSpace` over their
D65 boxes, so §7.4.9 sets the boxes aside: rendered, looked at, unchanged. The new census skipped
encrypted files at first. The arms then moved `bug1782186.pdf` (D65 Lab under a `Separation`, its
stroke (64, 63, 70) both ways), and the census now opens it (trap 13).

**Hunks outside the slot's files.** `transparency.rs`'s space identity hashes the white,
`soft_mask.rs` loses a false comment, the JPX census's labels, a NOTICE section 6 for the compiled-in
observer table, and `raster_golden.tsv`'s three Lab rows. `doc/third-party-data.md` (slot 4's) owes
a `data/cie/` line.

**Unfinished.** e-sRGB, e-sYCC and CIE Jab wait on unheld texts; an undefined `IL` takes the fallback.

**Gates.** rustfmt `--check` on the eleven touched sources: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy --all-targets` for `-p pdf-colour`, `-p pdf-model` and `-p pdf-transform`: exit 0 each.
`cargo nextest run`: pdf-colour 128 passed; pdf-model 2002 passed, 19 skipped; pdf-transform 489
passed, 10 skipped. `cargo test -p conformance`: exit 0, 417 passed. The arms ran from copied
binaries, one hold, 349.1 s, 4.53 GiB. The 1x arms against `/home/AI/arms-1438/` ran 81 s, 77 s and
75 s, exit 0 each, 968 pages: 1 moved, `bug1782186.pdf`. The 4x arms ran on the census's four Lab
documents, 3 s, 3 s and 4 s, exit 0: 1 moved, the same page. `raster_golden`: exit 101, 3 moved, all
census Lab pages (`bug1782186` raster and list; `bug1721218_reduced`, `issue2761` list only),
regenerated, then exit 0. `pdf-model --test corpus`: exit 0, 9 s. `--test jpeg2000`: exit 0, 10 s.
Duration 4 600 s.
