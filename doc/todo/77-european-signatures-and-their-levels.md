# 77 — European signatures, their levels, and *locally valid*

Status: **open** — locally valid, the list reader, the lists' authentication, RSASSA-PSS subject
keys, the QC statements, TS 119 615 clauses 4.4 to 4.7, TS 119 172-4 clause 4.4.2's level, most of
clause 4.5's report with a)'s text and h)'s TS 119 312 assessment, a policy's trusted-list anchors, a C host's lists, the lists' fetch and the
window that asks for it are built (ADRs 2834, 2835, 2842, 2843, 2852, 2853, 2864, 2865, 2866, 2872,
2874, 2878, 2879, 2882, 2886, 2896, 2897), and
Annex A's time-stamp type is read as a silence (ADR 2883);
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
  a)'s heading and sentence are carried as printed (`Scope::heading`, `Scope::text`; A855, ADR
  2897), and h)'s potential security issues are `applicability::ts_119_312`'s assessment against
  TS 119 312 V2.1.1's Tables 1, 2, 3, 4, 6 and 7 and clause 7.3, transcribed cell by cell — a
  component no row lists, a legacy one, a phase-out passed by the best signature time or by the
  validation, each naming its table (A856, ADR 2896) — with clause 9.2's imprint hash of every token
  taken as a time (ADR 2905) and clause 9.3's signer's key, whose legacy rows of Tables 2, 3, 6 and
  7 are asked to outlast its certificate's `notAfter` (ADR 2927).
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
- **The lists fetched** (ADR 2882). `trusted_list::refresh`: TS 119 615 PRO-4.1.4-01's LOTL, its
  pivots and its pointer to itself, then PRO-4.2.4-03 and -04's Member State lists, asked of a
  host's transport within `MAX_FETCHES`; clause 4.1 runs over what was fetched in front of the
  host's files and nothing is kept where it fails, and a Member State's list is kept only where
  clause 4.2 passes on it, so a held copy stays otherwise. Each fetched file is stored under its
  location whole, percent-encoded (`authentication::stored_name`), which is looked for before a
  last path segment — three Member States publish a `tsl.xml`. PRO-4.1.4-16 and -17's updates are
  returned as the configuration's files (b) 1)'s pivot certificates; b) 2) is not read, and then
  neither half is made, said), and a file stored from a location no longer led to is removed.
  `viewer_host::trusted_list_fetch`: `may_fetch_trusted_lists` at A98's `Submissions` level, `ask`
  by default, refused at every level without a `--trusted-lists` directory; the GETs through
  `submit::fetch`; each file written and renamed into place. Over the evidence of 2026-11-12 it
  authenticates the LOTL through five pivots, Austria's and Germany's lists, moves `OJEU-Loc` to the
  2026 publication with pivot 378's six certificates, and the next run passes with no pivot.
- **The window that asks for the fetch** (ADR 2886). Shift + T is `WindowAct::TrustedLists` in
  every window, `QUORRA_ACT_TRUSTED_LISTS` 23 in the C ABI (carried out inside the library, so a C
  caller is owed a question or a sentence, never the act). `may_fetch_trusted_lists` is asked; at
  `ask` the question names the directory (`Subject::TrustedLists`); the refresh runs on a
  `trusted_list_fetch::Refresher` thread — never the event thread, never the confined worker — at
  this machine's clock, and reads both directories again there through `reader::Trusting`, so the
  window says what became of each list and hands the core `Command::Trust`, which forgets every
  report worded under the old lists. One refresh at a time; a `no` and a refusal are said.
- **Annex A's `id-etsi-dst-euqtst`, read** (ADR 2883): TS 119 172-4 states no process that gives a
  time-stamp token a type — clause 4's processes are a signature's or a seal's, and REQ-4.5-02's
  identifiers are a "may" — so a document timestamp is reported by TS 119 615 clause 4.7's answer
  and given no type, a recorded silence.
- **A C host's lists** (ADR 2879). `quorra_trusted_lists` beside `quorra_trust_anchors`, each
  keeping the other's half of the policy the session holds; names required, the caller's clock
  carried where no anchor was given.

## 2. What is left, in order

3. **TS 119 172-4 REQ-4.5-01 h)**'s remainder (ADRs 2896, 2905, 2927): TS 119 312 clause 9.5's
   other keys — the CAs' on the path, the CRL issuers', the OCSP responders', the time-stamping
   units' — are not asked, because the report is handed none of their certificates (`verdict::Valid`
   keeps the path's length only); building it carries those certificates in and chooses what "at
   least as strong" compares across families (ADR 2927 item 5). No national rules are held.
4. **The fetch's remainder.** GPR-4.0-03's *should* — go on past a failed `https` validation with
   a warning — is not taken: the client has no unvalidated mode, and a failed connection is said as
   the location not fetched (re-read 2026-11-16, ADR 2886). A list a policy's `TAsInTrustedList`
   references outside the LOTL's tree is fetched by nothing.
