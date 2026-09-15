# 1071 — The clauses that pointed somewhere free

Four rounds (1020, 1048, 1053, 1062) left §12.8's residue with one reason: ETSI EN 319 122 is "not a
document this tree holds". ETSI publishes it free. Fetched EN 319 122-1 **V1.3.1** (2023-06) and
EN 319 122-2 **V1.1.1** (2016-04) from etsi.org — a browser `User-Agent` is needed, plain `curl` gets
403 — into `doc/` and, via `tools/spec-md.py`, into `doc/md/`; `/doc/*.pdf` and `/doc/md` ignore both
already. **Their notice permits no reproduction without written permission**, so every ETSI rule here
is paraphrase under a clause number while ISO quotations stay verbatim. ADR 1085.
`cms::Attribute` is the enabling change: the reader recorded attribute *identifiers* and two values,
so no rule about an attribute's set or component count was writable. It now records all three.

| clause | rule, paraphrased | departures |
|---|---|---|
| 5.3 — (b) | unsigned; one component; imprint = digest of the `SignerInfo` `signature` field | 3 |
| 5.2.8 — (c) | signed; one component; imprint = digest of the detached external data, `/ByteRange` | 4 |
| 5.2.5 — (h) | signed; one component; names a country, locality or postal address | 3 |
| 5.2.6.1 — (j) | signed; one component; not empty, neither certified member empty | 3 |
| 5.2.9.1 + Table 1 | policy identifier signed, not the implied alternative; eleven cardinalities; (a) SHA-1 is the v1 attribute's; (c) a policy store needs a real digest | 6 + `PadesProfile` |

§12.8.3.4.8 — `timestamp::signature_timestamp_established` runs ADR 1071's four steps over the token
inside the attribute, which meant extracting `signature::authenticity_of(cms, Detached)` and
`signature::cms_trust(…)` from `Signature::authenticity`/`trust_for` — **one copy** of the
arm-per-construction match, not two. `viewer_core::notes` takes that instant after the chain's
(§12.8.3.4.5 (b)) and only where clause 5.3's imprint says it is about *this* signature; with no
anchor nothing is established (ADR 1039, 1076), so nothing a person sees changes.

**Census, 90 763 files.** 490 `ETSI.CAdES.detached`; 485 profiled: 444 PAdES-E-BES, 41 PAdES-E-EPES.
Attributes stated: signature-time-stamp 228, signature-policy-identifier 41, mime-type 11,
commitment-type-indication 4, signer-attributes-v2 1, and **zero** content-time-stamp, signer-location
or signature-policy-store, so (c)'s and (h)'s rules have no corpus witness and the fixtures are their
only calibration (trap 8). Firing on real files: `SignaturePolicyImplied` 26,
`SigningCertificateV2StatesSha1` 5, `CommitmentTypeAndReason` 2, one attribute twice, two of them
confirmed by `openssl asn1parse`. **The ledger said "no corpus document is a PAdES signature" twice —
wrong by hundreds**; the census now prints every witness by path.
Calibration (trap 13): `cms::fixtures::pades_referring` builds one deformation per rule beside a
`Conforming` case stating every attribute the others deform; the timestamp tests use `openssl ts`'s
real token with two planted negatives. Rows: §12.8.3.4.3 `partial` → **`implemented`**; §12.8.3.4.4
`reported` → **`partial`** (what is left needs a signature policy no file carries); §12.8.3.4.8
`partial` → **`implemented`**; §12.8.3.4's note corrected. A sibling's run found a blockquote the
checker read under §12.8.3.4.3 — verbatim quotation, wrong clause — which is now fixed.
