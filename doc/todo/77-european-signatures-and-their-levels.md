# 77 — European signatures, their levels, and *locally valid*

Status: **open** — the first slice is built (ADRs 2834, 2835); section 2 is the order for the rest.
Priority: the owner's, 2026-11-06, `doc/questions/A800` item 3: "we should be able to validate all
european digital signatures and be able to tell the user the different levels of signatures QES,
AES,..." and "We should have a local valid certificate setting as well."
The texts: ETSI TS 119 612 V2.3.1 (trusted lists), TS 119 615 V1.3.1 (procedures for using them),
TS 119 172-4 V1.1.1 (applicability rules for EU qualified signatures), EN 319 102-1 V1.4.1
(validation), EN 319 412-5 V2.4.1 (QC statements) — free from ETSI, cited by clause and
paraphrased (ADR 1085), never tracked. Fetch them into a scratch directory with
`curl -sfL -o <file> https://www.etsi.org/deliver/<path>` (TS 119 612's schema is the `p0.zip` beside
its PDF). DSS (`esig/dss`, LGPL-2.1) is evidence about a reading only, and its code is never copied.

## 1. What is built

- **Locally valid.** `pdf_signature::verdict::Standing`: a `Valid` says whether its path ended at the
  reader's own anchor (`--trust-anchors`, `TrustPolicy::anchors`) or at a service a trusted list
  publishes. `viewer_core::notes` words the first "locally valid" (ADR 2834).
- **A trusted list read.** `pdf_signature::trusted_list::TrustedList::read`, TS 119 612 clause 5's
  scheme, provider, service and history fields, bounded; `fuzz/`'s `trusted_list` target.
- **TS 119 615 clause 4.3** (`matching`), and TS 119 172-4 REQ-4.2-03 a)'s anchors
  (`listed_anchors`): services of type `CA/QC` that issued the signer, at its `notBefore`.
- **The host input.** `TrustPolicy::lists`, `--trusted-lists <dir>` in `viewer-host`, carried over
  the confined pipe; off by default.

## 2. What is left, in order

1. **Authenticate the lists** — TS 119 615 clause 4.1 (the list of trusted lists, against the
   Official Journal's certificate set, GPR-4.0-02's `OJEU-LOTL-Certs-Set`, a host's configuration)
   and clause 4.2 (each Member State's list, against the certificates the former's pointer names).
   Both are XMLDSig enveloped signatures under exclusive canonicalisation, and
   `policy_rules::canonical_xml` already canonicalises (ADR 2644): what is missing is the
   `ds:Signature` reader, the `ds:Reference` transforms and digest, `SignedInfo`'s verification with
   the families `pdf-signature` holds, and the LOTL's pointer reader. Then
   `trusted_list::Authentication` gains `Authenticated`, and the sentence drops its caveat for it.
2. **EN 319 412-5's QC statements** read off the signing certificate: `QcCompliance`, `QcSSCD`,
   `QcType`, `QcCClegislation` (pdf-pki's `x509` extensions).
3. **TS 119 615 clause 4.4** (EU qualified certificate determination at the best signature time):
   the service's qualifications extension (TS 119 612 clause 5.5.9.2, `CriteriaList` and
   `Qualifier`), additional service information, the status at the instant, the transition rules.
   This is the long one — fourteen pages of processing — and a round of its own.
4. **Clause 4.5** (QSCD determination), then **TS 119 172-4 REQ-4.4.2-06**'s join into a
   `verdict::Level` beside `Standing`: qualified signature or seal, advanced on a qualified
   certificate, advanced, indeterminate, each with the requirement that decided it. Only then does
   any report say "qualified" (ADR 2835).
5. **The list of trusted lists fetched** by a host at A98's `Submissions` level through ADR 1728's
   port, `ask` by default — a transmission, never the confined worker's.
6. **§12.8.3.4.4's `TAsInTrustedList`** (TS 119 172-2 clause 4.22.2.1 b)) evaluated against the
   lists a host supplied, which ADR 2782 named and the reader now makes possible.
7. **A C host** supplies lists: one `viewer-ffi` call beside the anchors' (`abi/host.rs`), and the
   FFI counts with it.
8. **Time stamps' authorities** in a list (TS 119 615 clause 4.7) for the document timestamp chain,
   which still asks the local anchors only.
