# 1905 — An XML signature policy is bound through the digest over its canonicalised components

Slot 5 of batch 131, 2026-11-02, Opus 5.5, two sessions; ADR 2644 used, ADR 2645 and Q781 unused. Ledger: §12.8.3.4.4
stays `partial` under ADR 1035, its residue human-readable policies and an XML policy whose `Digest` names Exclusive
Canonical XML or Canonical XML 1.1, which is buildable and is what is unfinished (A713).

**Premise.** One pass over the module's tree did not hold: ADR 2604's tree drops comments, instructions, prefixed
attributes and per-element declarations, which Canonical XML 1.0 sections 2.2 and 2.3 render, so the canonicaliser
reads the `xmlparser` tokens again under the reader's three budgets and streams the form into the digest.

**Built.** `crates/pdf-signature/src/policy_rules/canonical_xml.rs` and its tests: inclusive Canonical XML 1.0 in both
forms over the document subset TS 119 172-2 clause 4.3.2 names (`PolicyComponents`), section 2.4's inherited `xml`
attributes, the refusals of section 2.1 and XML Namespaces 1.0 section 6.3. `policy_rules.rs`: `enforce_xml` binds a
document whose canonical digest under the signer's function is the signed value, else the new
`NotEnforced::CanonicalDigestDiffers`; `CanonicalDigestNotComputed` names another method; `viewer-core`'s notes say
both. Fuzz target `policy_document`: the form is a function of the bytes and its own fixed point, in both forms;
`fuzz/seed_policy_document.py` seeds both methods. The row, `doc/todo/65` and `doc/verify.md` follow.

**Plant.** The claim's: the fixture's components with the root's declarations and the apex's attributes reordered give
the signed digest and are enforced, the signing period broken withholds the word; one character changed (2021 to 2029)
is `CanonicalDigestDiffers`. The instrument's, the namespace and apex attribute sorts dropped: the verdict test passed
by hash-map chance, and `a_subset_renders_…_in_section_2_order`, the subset test, is the plant that caught it.

**Measured.** The `getrusage` probe (five documents at the budgets, each canonicalised a hundred times) first took 118 s
for an ancestor filled with `xml` attributes: their de-duplication and the apex's merge were scans, quadratic. Both
are set look-ups now: maxrss 15 608 KiB in 5.35 s, the slowest document 23 ms a run, against 12 168 KiB for
TS 119 172-3's hostile probe and 12 364 KiB for a unit test in the same binary. The fuzz target's second run found a
processing instruction `<?xml<?…` whose form `<?xml <?…` reads back as a misplaced declaration; a target `xml` or one
run into its data is refused now (XML 1.0 section 2.6), three cases in the refusal test, failing with the check off.

**Gates.** `rustfmt --check` on this round's files 0; `cargo fmt --check -p pdf-signature -p viewer-core` 0; clippy
`--no-deps --all-targets -D warnings` on both 0; under `fuzz/`, `cargo fmt --check` 1 on a sibling's
`type1_write.rs:85`, clippy `--all-targets` 101 on a sibling's `formcalc_cascade.rs:174`, `--bin policy_document` 0
once this round's missing `;` at line 136 was put in; `cargo test -p pdf-signature` 0 (296 passed, 5 ignored),
`-p viewer-core` 0 (436 passed, 2 ignored); `policy_document` built `-s none` under the bound, three runs: 633 s,
17 304 717 runs, no crash, 30 seeds to 2 322 files, 1 467 to 2 733 edges, peak 0.07 GiB; after the set fix 606 s,
7 737 169 runs, the one finding above; after its refusal that input 0, then 631 s, 21 957 676 runs, no crash, 30 seeds
to 2 540 files, 3 038 edges, peak 0.24 GiB; `--bin cited` 0, `--bin quotations` 0; `cargo test -p conformance
--no-fail-fast` 0 (470 passed), then 101 (467 passed, 3 failed, siblings': `names` and `teaching` on
`archive/prepare.rs` names, `records` on records 1903 and 1904 still being written).
