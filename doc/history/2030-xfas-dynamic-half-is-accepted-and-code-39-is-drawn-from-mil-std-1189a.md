# 2030 — XFA's dynamic half is accepted, and Code 39 is built from MIL-STD-1189A

Session 2030, a single ledger round in the audit worktree. Rows K and K.1; owner answer A840; ADRs 2876 and 2877;
Q857.

**The slice.** A840's items 1 and 3 in the brief's order: the amendment, XFA 3.3 into the archive with a checker
that reads it, then the barcodes — Code 39, PDF417, Data Matrix — each whole or not at all.

**The premise.** Nothing of the dynamic half or of any barcode was built; no `xfa-barcode` existed and no test read
XFA 3.3. The amendment and the archive were effort only. The barcodes were not: the public sources decide them.
Code 39's table is in MIL-STD-1189A (reproduced in DTIC ADA180561); PDF417's only complete public text is
Public.Resource.Org's ISO/IEC 15438:2006, whose use is the owner's call; Data Matrix's encodation and placement are in
ISO/IEC 16022 alone. Both go to Q857.

**Built.** `CLAUDE.md` loses the exclusion and gains the fourth amendment; `doc/scope-ratifications.md` carries its
argument; `doc/todo/78` is scheduled after the ledger rows' final states with A840 item 2 as the first requirement.
`doc/specifications.zip` carries XFA 3.3 (32 entries, round trip compared by hash). `tools/conformance/tests/
xfa_citations.rs` holds 291 page citations to pages the text has, titles to their pages, and refuses a blockquote.
`crates/xfa-barcode`: the 62 identifiers matched without case, `checksum` keywords, page 440's widths, Code 39 from
table E.1; `auto` and `code3Of9extended` refused, as is every other symbology, by name.

**Tests.** `xfa-barcode/tests/code39.rs` (nine; every symbol read back by `rxing` at ratios 2.0, 2.5, 3.0);
`xfa_citations.rs` (two). **Plants**: table E.1's `1` given `2`'s bars fails three tests; a citation of page 1661
fails the sweep. Two real mis-titled citations it first found were sections begun a page earlier, and the rule
reads the page before.

**Gates.** Over `xfa-barcode` and `conformance`: `cargo fmt --check` exit 0; clippy `-D warnings --all-targets`
exit 0; nextest of `xfa-barcode` 9 of 9 passed; rustdoc `-D warnings` exit 0; `cargo deny check licenses bans` ok.
Conformance after `git add -N`: exit 101 on this record's gates paragraph alone, then `--test records` exit 0.
`state.sh --round 2030 docs` exit 0. Tier 2 and the fuzz check are not owed: no crate a walk or a fuzz target
builds changed, and no existing public type did; the lock gained only `rxing`'s dev-only tree.

**Left of the rows.** K and K.1 stay `implemented`: §K.1's permission is still taken out loud until the layout is
built, after the ledger. PDF417 and Data Matrix wait on Q857.
