# 2101 — A compromised unit's time is not reported established, a /Background's fault is the file's, the windows' sizes argued

Session 2101, a leftovers round in the gwg worktree. Rows 12.8.3.4.8 and 8.7.4.3 (both `implemented`, unchanged).
ADRs 3105 and 3106. Three items in the brief's order, all built.

**The premises.** Item 1's held in part: round 2067 had put a token's unit into the report through document
timestamps, and `a_tokens_own_path_…` a token's chain; no report-level test had a *signature* timestamp (no
`TimeKind::SignatureTimestamp` outside the report's own code), so a chain carried there was untested. Item 2's held:
`note_unpainted_background` raises the variant only for an array of the wrong length. Item 3's held: ADR 2926 §5.

**Item 1 (ADR 3106).** The fixture is `policy_rules::revocation_checks`'s generated token (unit and intermediate
carried, root not; imprint over `DE AD`), put in a `signature-time-stamp` attribute and signed into
`verdict::tests::fixtures::document`; its `tests` module is `pub(crate)` for it. The first run found the defect: with
the intermediate's CRL listing the unit `keyCompromise` at `genTime`, the report said the token established, which
§12.8.3.4.8 says "cannot be used". `applicability::reliable` asks the clause's three cases (the rule ADR 2968 gave the
best signature time) and the signature timestamp's `established` takes it. The test reads four reports: root and
current CRLs, established, unit and authority at position 1 each outlived to 2036; no anchor, not, the unit alone; the
compromised CRL, not; a list's `TSA/QTST` service of the root, established, listed by `AT`, clause 4.7
`INDETERMINATE` for a unit with no country.

**Item 2.** `corpus.rs` classes `ShadingBackground` as the file's, the comment naming ADR 3094 and Table 77's
count. The walk before and after: 974 documents, 65 incomplete; the file 61 documents and 94 reports, neither one 4
and 4, this reader 0 and 0 — identical, since no document here raises the variant.

**Item 3 (ADR 3105).** 800 × 1000 (`quorra`, `quorra-confined`), 1000 × 1100 (GTK and Qt: the same page viewport with
their chrome), 794 × 1123 (the confined worker before its first `Resize`) and the gates' 1600 × 1000 are each argued
and kept; comments cite the ADR, and `launch_path.rs` no longer says no ADR argues its window.

**Plant.** `reliable` bypassed: the new test fails on the compromised case; restored, it passes.

**Gates.** Over `pdf-signature`, `pdf-model`, `viewer-gtk`, `viewer-ui`: fmt exit 0; clippy `-D warnings
--all-targets` exit 0 (after a split of the new test for `too_many_lines`); `pdf-sandbox --bins` exit 0; nextest `--lib
--bins --tests` under `xvfb-run -a` 3712 of 3712 passed, exit 0, failures equal to the manifest's (none); rustdoc exit
0. Conformance after `git add -N`: exit 101 on this record's gates paragraph, then exit 0. Fuzz not owed.

**Left.** Document timestamps under a compromised unit are §12.8.5.3's chain, not this rule; GTK and Qt windows taller
than a 1080-line display rely on the compositor's constraint (ADR 3105's stated cost).
