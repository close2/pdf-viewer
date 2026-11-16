# 77 — European signatures, their levels, and *locally valid*

Status: **open** — locally valid, the list reader, the lists' authentication, RSASSA-PSS subject
keys, the QC statements, TS 119 615 clauses 4.4 to 4.7, TS 119 172-4 clause 4.4.2's level, most of
clause 4.5's report, a policy's trusted-list anchors and a C host's lists are built (ADRs 2834, 2835,
2842, 2843, 2852, 2853, 2864, 2865, 2866, 2872, 2874, 2878, 2879);
section 2 is the order for the rest.
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

- **TS 119 615 clause 4.4** (ADR 2864). `trusted_list::qualifications`: TS 119 612 clause
  5.5.9.2's `Qualifications` extension read, criteria lists (key usage, policy sets, clause
  5.5.9.2.2.3's extended key usage and subject attributes, nesting) asked of a certificate in three
  answers, a critical extension's undecidable criteria discarding the certificate.
  `trusted_list::determination`: PRO-4.4.4-01 to -36 at the best signature time on the list of the
  issuer's territory, Tables 1 to 5, the run at `notBefore`; `verdict::Listing::qualification`
  carries it and `viewer_core::notes` says the clause's indications as being about the certificate.
  No level is worded.
- **TS 119 615 clause 4.5** (ADR 2865). `trusted_list::determination::qscd`: PRO-4.5.4-01 to -05
  over clause 4.4's `CHECK_1_SET-OF_QE` and `CHECK_2_SET-OF_QE` and the certificate's `QcSSCD`,
  Tables 6 and 7, unknown qualifiers refused under a critical extension; carried beside clause
  4.4's outputs and said as `QSCD_YES` and its siblings, about the key.
- **TS 119 172-4 clause 4.4.2** (ADR 2866). `verdict::level`: REQ-4.4.2-02 to -06 over clauses 4.4
  and 4.5's outputs and clause 4.2's validation, run again over the listed service under
  REQ-4.2-03 c)'s constraints (status checked, none unknown accepted, information issued after the
  best signature time: `trust::PathInputs::revocation_issuance`). A `Level` is qualified signature
  or seal, indeterminate (stopped at -02 or -05) or not qualified (-06), with the requirement that
  decided it and Annex A's type and identifier where the outputs support one; a locally valid path
  has none. `viewer_core::notes` says "qualified" about a signature only from it. REQ-4.4.2-03
  names no check.
- **TS 119 172-4 REQ-4.5-01's report** (ADR 2872). `verdict::applicability::Report`: a)'s scope as a
  type, b) the signing certificate's subject attributes and `subjectAltName` names, c) its
  `pseudonym`, d) the claimed signing times, document and signature timestamps (policy, accuracy,
  established or not), the material's revocation, issue and next-update times about the signer and
  the best signature time, f) every attribute by set with a commitment named, h) the suite;
  `Cryptography::of` is REQ-4.2-03 d) and e), an unsupported suite named and indeterminate.
  `viewer_core::notes` says one sentence per item beside a listed signature's level, and e)'s
  warning wherever lists were given. `verdict::level`'s join is pinned end to end by a listed
  signer whose issuer's CRL was issued after the best signature time (`TOTAL-PASSED`), with the
  same signer and an earlier CRL as its control.
- **A policy's trusted-list anchors** (ADR 2874). `trusted_list::anchors`: TS 119 172-2 clause
  4.22.2.1 b)'s three steps — the services' certificates of the referenced specific list, or of every
  specific list a referenced list of trusted lists points at, filtered by the stated service types
  and by the stated statuses at the host's current time — over the host's files by ADR 2842's names.
  Both policy syntaxes read `TAsInTrustedList` whole; each anchor is a trust point of the
  referencing one (`certification_path::ListedTrustPoint`), a reference not resolved or a pointed-to
  list not supplied is counted unsearched, and `viewer_core::notes` names the list that determined
  the anchor a path ended at.
- **Time stamps' authorities in a list** (ADR 2878). `trusted_list::time_stamp`: TS 119 615 clause
  4.6 (PRO-4.6.4-01 to -09, the list named by the subject's `countryName`) and clause 4.7 (the two
  runs at the instant asked and at `genTime`, compared), and the services of types `TSA/QTST` and
  `TSA` clause 4.3 returns for a token's authority at its `genTime`. Those services' identities
  anchor a document timestamp's and a signature timestamp's authority where the reader's anchors
  reach none (`timestamp::Authority`); clause 4.3's path test states the time-stamping purpose for
  those types. The report's d) iii) 3) is clause 4.7's answer (`applicability::EuQualified`, not
  asked for a token not established) and d) i) names the listed services or their absence.
- **A C host's lists** (ADR 2879). `quorra_trusted_lists` beside `quorra_trust_anchors`, each
  keeping the other's half of the policy the session holds; names required, the caller's clock
  carried where no anchor was given.

## 2. What is left, in order

3. **TS 119 172-4 REQ-4.5-01**'s remainder: a)'s text verbatim, which waits on
   `doc/questions/Q855` (ETSI's notice, ADR 1085); h)'s potential security issues, assessed against
   ETSI TS 119 312's tables once they are fetched and transcribed (free from ETSI, never tracked;
   the fetch answered 403 on 2026-11-15, and `doc/questions/Q856` asks the owner to supply it).
4. **The list of trusted lists fetched** by a host at A98's `Submissions` level through ADR 1728's
   port, `ask` by default — a transmission, never the confined worker's — keying each file by its
   URI as ADR 2842's names do, and persisting PRO-4.1.4-16 and -17's configuration updates.
8. **Annex A's `id-etsi-dst-euqtst`**: whether a document timestamp is owed that type. TS 119 172-4
   clause 4.4.2 joins a level for a signature and names no join over a time-stamp token; until a
   reading finds one, a document timestamp is reported by clause 4.7's answer and given no type
   (ADR 2878).
