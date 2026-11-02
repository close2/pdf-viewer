# 1892 — A photographed page's own damaged stream, an unclosed text object and a keyword inside an array are baked in

Slot 2 of batch 130, 2026-11-02, an "archive" round (Opus 5.5), three sessions; ADRs 2618 and 2619 written, Q768
unused. No ledger row moved.

**The premise that did not hold.** Not two kinds admit but three: `Content { Damaged }` (ADR 2516 decision 5 named it;
the code built it only for a form), "BT without ET" (§7.8.2's order and §9.3.6's clip at `ET` make the window's picture
the standard's reading) and a keyword inside an array (§7.3.6 states no recovery, but it is the class decision 5 admits
for an operator short of its operands). ADR 0816 is the fence's reason, ADR 2516 its list.

**What changed.** `pdf_model::content::StreamSyntaxFault` and `Unsupported::stream_syntax_fault` name the two
interpreter sentences from the one constant each is written with; `fonts/photographed.rs` admits them and `Content {
Damaged }` under the wide gate. A part of `/Contents` drawn of nothing stays refused, with an image or shading this
reader cannot build and a drawing stopped at a bound; the two PDF/A documents say so. ADR 2619, the checker's half: the
survey reads a run §7.3.3 spells as no number as the keyword §7.2.3 makes it, and `fonts/font-programs-embedded` fails
a program whose decode stops short.

**The census**, against HEAD's, without and with the pack (identical moves): corpora `as-if-printed` 2b 411 → 419
documents converted, 4 416 → 427, either part 417/8/59 → 425/12/47; pdf.js unchanged (`bug1953099` freed, `issue11651`
lost); `as-shown` one fewer in each corpus and target. Freed at both targets: `050734`, `073439`, `226387`, `367594`,
`379049`, `498264`, `509284`, `T02-05-01_011_…`, `bug1953099`, and `303226` at 2b; at 4 also `020806`, `033689`,
`040669`, `477139`. "Does not draw whole" 19 → 5 documents (an image, a shading or a bound each). Refused under ADR
2619 decision 2, each on a program the file holds the front of: `435321`, `178360`, `268346` at 4, `F001u_3_7j` and
`issue11651` (two converted at HEAD). `027613` (`as-shown`) loses its `.notdef` row and stays refused on four others.
New `output-fails`: `064034` (XMP) and `507618` (widths). veraPDF at 2b and 4: PASS for every document freed (`509284`
photographs pages 2, 4, 9, 14, 18 for 6.2.2). A planted `pdfaid:part` of 7 in `509284`'s 2b output FAILs 6.6.4. **The
plants.** ADR 2618's three admissions taken back fail its unit and two integration tests; ADR 2619's two readings taken
back fail both its unit tests; each restored.

**Gates.** `cargo check -p pdf-model -p pdf-transform --tests` exit 0; `cargo test -p pdf-transform --test archive
--test profiles` exit 0 (524, 13 passed); `cargo test -p pdf-archive` exit 0; clippy `--no-deps -D warnings` on
pdf-model, pdf-transform, pdf-archive `--all-targets` in the export exit 0; both censuses and `rank.py` exit 0;
`archive_corpus --ignored` exit 0; `cross_check` and `witnesses --ignored` exit 0 (13560 selections compared, 0
disagreements); `--bin cited`, `--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 101, 467
passed, 3 failed: this record before this paragraph, `type1_write` unseeded (slot 3's) and two round numbers in
`pdf-font/src/cff_stop_tests.rs` (slot 7's).

**Found, not fixed.** `064034`: the conversion breaks an XMP packet it met (`pdf-archive/src/table/metadata.rs:157`);
`507618`: widths disagree with the program (`table/fonts.rs:291`). A remedy that rewrote a salvaged run to the number
the reader draws would free `509284`'s pages without a photograph; it is not built.
