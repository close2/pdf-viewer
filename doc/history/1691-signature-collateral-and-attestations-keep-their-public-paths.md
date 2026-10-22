# 1691 — Signature collateral and attestations keep their public paths

Docs round, batch one hundred and six, 2026-10-22. ADRs 2216–2217; no new question.
**Implemented; scoped gates passed and final integration accepted (ADR 2209).** ISO 32000-2 §12.8.4.3's
security-store reader and §12.8.7's legal-attestation reader move into private children.
The root keeps the cryptographic implementation and every original test. Nine public
names remain explicit re-exports; helpers, fields, limits and attributes keep their visibility.
Root: 6595→6200 lines; children: 266 security-store and 146 legal-information lines.
The remaining large root is acknowledged; this is a bounded concern split, not a bug fix.

**Proof.** Snapshot hash equals baseline b9d05ee8's signature.rs. Exact moved bytes match
apart from outer blank lines: ten security-store items and four legal-attestation items.
proc_macro2 compares complete token trees and separate literal inventories: 108 and 72
literals, including documentation. Reconstructed-root tokens match with only approved
module/re-export/import scaffolding and item removal. All original test tokens remain.
Changing the 1024 limit to 1023 and removing its declaration each fail with exit 101.

**Gates.** Signature nextest: 236 passed, one skipped, 3.707 s tests. Warnings-denied
all-targets clippy passes, 8 s; transform/viewer-core all-targets caller compilation passes.
Initial rustdoc correctly rejects private security_store/module and function ambiguity.
A path-bound private collateral module removes that collision without changing any
original doc literal or public name. The final root proof passes; warnings-denied private
rustdoc passes, 3 s, and signature/transform/viewer-core all-targets checks pass, 31 s.
Final formatting/diff checks pass; doctests pass (zero cases), 9 s; conformance passes, 142 s.
No runtime test assertion changes.

Found, not fixed: the old §12.8.4.3 ledger note describes counts and names a Legal test;
this evidence predates the move and is reported separately from code-pointer relocation.
Narrow todo74/crate-map/ledger pointer edits completed in 361f6021; no clause status changed.
Owned six paths: scratchpad/r1691/owned-paths.txt. Proof and gates: scratchpad/r1691/.
Duration: 472 s from 2026-10-22T04:42:05+02:00, including checks and coordination.
