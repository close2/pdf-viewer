# 1863 — An ASN.1 signature policy bound to its signature is enforced over it, one constraint at a time

Slot 3 of batch 127, 2026-10-28, Opus 5.5, one session; ADR 2560 used, ADR 2561 and Q729 unused. Ledger: §12.8.3.4.4
stays `partial`, its residue narrowed (A713). Part 2, TS 119 172-2's XML, not started: part 1 ended past call 70.

**Premise.** The brief's hypothesis held: the parser is a sibling of `cms.rs`'s readers on `der.rs`, unchanged, and
enforcement is one function from a parsed policy and a `SignedData` to findings. One premise did not: "the ASN.1
syntax" is two. ETSI TR 102 272 clause 6 (RFC 3125's) states the signing period, mandated attributes and algorithm
constraints the brief names; ETSI TS 119 172-3 is a second ASN.1 syntax with another vocabulary (clauses 4.2 to 4.29),
so it is told apart by its top level and refused by name, and stays owed.

**Built.** `pdf_signature::policy_rules`: `PolicyDocument::parse` under a 256 KiB byte budget, `der::MAX_DEPTH`
through `der::is_canonical`'s pre-pass (which also enforces clause 6.1's DER and walks the whole document before any
list is allocated) and a 4096-entry budget charged before each push; clause 6.1's own hash re-computed, clause 6.3's
placement and clause 6.4's uniqueness refused when broken. `enforce` acts only on a stored copy bound by the signed
digest or by the policy's own hash (Annex B.11's second way) and naming the signer's policy; each constraint a
`Finding` met, broken or not evaluated, displayed as a sentence. `viewer-core`'s `notes.rs::policy_constraints` says
them beside the count broken, and keeps the old sentence where the file stores no copy. A broken constraint withholds the verdict: `Verdict::under_policy`
makes a `Valid` into `Reservation::PolicyConstraintBroken` (the orchestrator's correction). `signature::signer_certificate`
became `pub(crate)` for the signer's key length. ADR 2560 records four readings (the period's weaker modal, clause 6.10
read literally, vacuous placement, TS 119 172-3 told apart).

**Plants.** The signing period's end comparison inverted: `a_policy_whose_signing_period_ended_…` failed, restored.
`der::is_canonical`'s pre-pass removed: the depth test got `OtherSyntax` instead of `Encoding(TooDeep)`, so the
refusal is the pre-pass's, before allocation; restored. Memory: the ignored hostile test (a 256 KiB document opening
nested indefinite values, and one entry past the budget, each parsed a thousand times) peaked at 8040 KiB by
`getrusage`, beside 8044 KiB for an existing `policy.rs` test in the same binary.

**Unfinished.** TS 119 172-3's rules; TS 119 172-2's XML (part 2); a copy fetched at the URL qualifier is bound but
not enforced (`viewer_host::policy`); `SignerFacts::of` takes the time from a signature timestamp only, not `/M`.

**Gates.** `cargo test -p pdf-signature` exit 0 (247 passed); `cargo test -p viewer-core` exit 0 (432 passed);
`RUSTFLAGS="-D warnings" cargo clippy -p pdf-signature --all-targets` exit 0; `cargo fmt --check -p pdf-signature`
exit 0; `clippy -p viewer-core` exit 101 on a sibling's `pdf-render/src/dash_end.rs` lines 123 and 146, none mine;
`--bin cited` exit 0; `--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 101, 468 passed and 2 failed, neither mine: `adr_status` on a
sibling's ADR 2573 and `fuzz_workspace` on a sibling's `read-fonts` pin in `fuzz/Cargo.lock`; `cargo doc -p
pdf-signature` under `-D warnings` exit 0; the fuzz target is not owed, `der.rs` is unchanged.
After the correction: `cargo test -p pdf-signature -p viewer-core` exit 0 (680 passed); `--test conformance --test records
--test adr_status` exit 0; `clippy --no-deps -D warnings -p pdf-signature --all-targets` exit 0.
