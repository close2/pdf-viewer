# 77 — European signatures, their levels, and *locally valid*

Status: **open** — locally valid, the list reader, the lists' authentication, RSASSA-PSS subject
keys and the QC statements are built (ADRs 2834, 2835, 2842, 2843, 2852, 2853); section 2 is the
order for the rest.
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
  the confined pipe; off by default. A host given lists and no anchor directory carries its clock
  with them (ADR 2842).
- **The lists authenticated** (ADRs 2842, 2843). `pdf_signature::xml_signature`: XML Signature 1.1
  core validation of TS 119 612 Annex B's enveloped signature (RSA PKCS#1 v1.5, RSASSA-PSS, ECDSA's
  `r || s`; the element chosen by its ordinal in the tree; wrapping shapes refused; `XAdES`
  `SigningCertificateV2` compared). `trusted_list::authentication`: TS 119 615 clause 4.1 with
  pivots and clause 4.2 over the host's files — a URI's file is the one named its last path
  segment, a Member State's list is the one of its territory, GPR-4.0-02's parameters are files
  named `OJEU-Loc`, `OJEU-LOTL-Loc` and `OJEU-LOTL-Certs-Set*`. A list not authenticated is refused
  by name with the clauses' indications; a host that supplied neither the LOTL nor the Official
  Journal's certificates has its lists `VouchedForByTheHost`, and the report says which.

- **RSASSA-PSS subject keys** (ADR 2852). `pdf_pki::x509::PublicKey::RsaPss`: RFC 4055 section
  1.2's `id-RSASSA-PSS` key read as an RSA key restricted to PSS, its parameters carried;
  `pdf_signature::pss::admitted_by_key` is section 3.3, asked by every PSS verifier (CMS,
  certificate, CRL, OCSP, a list's XML signature). A PKCS #1 v1.5 signature under such a key is
  refused. Germany's list, whose signer's key is one, is the evidence (never committed).
- **EN 319 412-5's QC statements** (ADR 2853). `pdf_pki::x509::Extensions::qc_statements` and
  `pdf_signature::qc_statement`: `QcCompliance`, `QcSSCD`, `QcType`, `QcCClegislation` read by
  their clauses' syntax, every other statement named by identifier, bounded; `fuzz/`'s
  `qc_statement` target. `verdict::Listing::statements` carries them and `viewer_core::notes`
  names them as the issuer's claims, never as a level.

## 2. What is left, in order

2. **TS 119 615 clause 4.4** (EU qualified certificate determination at the best signature time):
   the service's qualifications extension (TS 119 612 clause 5.5.9.2, `CriteriaList` and
   `Qualifier`), additional service information, the status at the instant, the transition rules.
   This is the long one — fourteen pages of processing — and a round of its own.
3. **Clause 4.5** (QSCD determination), then **TS 119 172-4 REQ-4.4.2-06**'s join into a
   `verdict::Level` beside `Standing`: qualified signature or seal, advanced on a qualified
   certificate, advanced, indeterminate, each with the requirement that decided it. Only then does
   any report say "qualified" (ADR 2835).
4. **The list of trusted lists fetched** by a host at A98's `Submissions` level through ADR 1728's
   port, `ask` by default — a transmission, never the confined worker's — keying each file by its
   URI as ADR 2842's names do, and persisting PRO-4.1.4-16 and -17's configuration updates.
5. **§12.8.3.4.4's `TAsInTrustedList`** (TS 119 172-2 clause 4.22.2.1 b)) evaluated against the
   lists a host supplied, which ADR 2782 named and the reader now makes possible.
6. **A C host** supplies lists: one `viewer-ffi` call beside the anchors' (`abi/host.rs`), and the
   FFI counts with it.
7. **Time stamps' authorities** in a list (TS 119 615 clause 4.7) for the document timestamp chain,
   which still asks the local anchors only.
