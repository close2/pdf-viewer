# 1870 — A signature walked from its field to the window's sentences, the fifth teaching document

Slot 10 of batch 127, 2026-10-28, Opus 5.5, one session, a docs round; ADR 2574 used, ADR 2575 and
Q736 unused. Ledger: no row moved — a walk changes no behaviour and cites the clauses it reads.

**Premise.** The brief's hypothesis held: five stages, six diagrams. The subsystem was read from a
snapshot of the shared tree taken after round 1863 reported, which is HEAD 2d54febf with 1863's
`policy_rules.rs` and `notes.rs` sentences on it (ADR 2560); the walk's opening says so. One
premise of the slot's stage list did not hold as written: the digest (§12.8.1) is read through the
CMS reader, so stage 1 names `cms::signed_data` and stage 2 says what it does.

**Built.** `doc/teaching/how-a-signature-is-verified.md`: the field and its range, the CMS
container and the key, the path with the file's own revocation material and the proven instant,
the signer's policy, and the verdict as a type and the report deferred past the first frame. The
running example is `bug854315.pdf` (named by file name alone — the corpus is a submodule), opened
in a `dev` build of `quorra` from the export under Xvfb with no anchors; its four `note:` lines are
in the walk verbatim. `WALKS` gains it with six diagrams; `doc/HANDOVER.md` and `doc/crate-map.md`
point at it. Every ADR the walk cites was opened for what it decides; three attributions were
corrected (ADR 0229 for the ASN.1 package, ADRs 1385/1386 for the curve stopgaps, the module's own
rule rather than ADR 2560 for "never counted as met").

**Plant.** `policy_rules.rs::enforce` renamed `policy_rule.rs` in the walk: the gate failed naming
the absent path; restored.

**Found, not fixed** (stale module comments since anchors became a host's, and one behaviour
question): `crates/pdf-signature/src/signature.rs:27` and `:1443`; `x509.rs:18`; `timestamp.rs:26`;
`trust.rs:466`; `policy.rs:15`; `viewer-core/src/notes.rs:738`'s reader-facing "can now say";
`verdict.rs`'s `Verdict::reached` takes no policy finding, so a broken bound constraint does not
withhold the word.

**Gates.** `cargo test -p conformance --test teaching` exit 0 in the shared tree and exit 0 in the
HEAD export carrying 1863's files and ADR 2560 (without that ADR: exit 101, four "ADR 2560 has no
record", the expected merge dependency); the plant exit 101; `cargo test -p conformance
--no-fail-fast` exit 0, 69 test binaries ok; `--bin quotations` exit 0, `--bin pointers` exit 0
(none of the eight absent is mine), `--bin cited` exit 0. The export's `dev` build of `quorra`
exit 0 (125 s, 1.43 GiB peak); the shared tree's release build exit 101 on a sibling's
`pdf-transform` mid-edit, so the example ran in the export (habit 138).
