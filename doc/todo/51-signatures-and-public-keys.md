# The four curves left, question 3, public-key handlers

Status: **question 2 is answered for every algorithm family the standard names.** Table 260's
three — RSA under both of RFC 8017's paddings, DSA, and ECDSA — and the EdDSA row ISO/TS 32002
section 5.1.2 adds beside them, all verify; every digest either table names is computed. What is
left inside question 2 is **four curves out of ISO/TS 32002's eight**, each refused by package
availability and each named at runtime by its own identifier. Question 3 is still a project.
Priority: 51
Corpus: 5 documents (§7.6.5's public-key handlers, counted below). `/R` 5 left this file in the
eight-hundred-and-eighty-seventh session — it is implemented. For the signature populations, **run
the census rather than reading a number here**:

```sh
find -L corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' > /tmp/paths
cargo run --release -p pdf-model --example signature_algorithm_census -- @/tmp/paths
```

**`find -L`, and the `-L` is not decoration.** In a parallel worktree `corpus-cache` is a *symlink*
into the main checkout, and `find` without it descends nothing and reports zero paths — a false
zero of exactly the shape this file warns about, met by the six-hundred-and-eighty-ninth session.

Clauses: §12.8.3, §7.6.5, §7.6.4.3, Table 21, Table 256, Table 260; ISO/TS 32001 §5.1, ISO/TS 32002 §5.1
Code: `crates/pdf-model/src/signature.rs`, `crates/pdf-model/src/cms.rs`,
`crates/pdf-model/src/der.rs`, `crates/pdf-model/src/x509.rs`, `crates/pdf-model/src/pkcs1.rs`,
`crates/pdf-model/src/pss.rs`, `crates/pdf-model/src/dsa.rs`, `crates/pdf-model/src/ecdsa.rs`,
`crates/pdf-model/src/eddsa.rs`, `crates/pdf-model/src/bigint.rs`, `crates/pdf-syntax/src/crypt.rs`

## Signature validation (§12.8.3) — 5 ledger rows, and it used to be 17

**This file used to say the whole clause needed "a trust store and a network". That was true of one
of the three questions a signature asks and false of the other two**; the three-hundred-and-seventy-seventh
session separated them and answered the first (ADR 0215), the three-hundred-and-ninety-second
answered the second for RSA (ADR 0229), the four-hundred-and-seventy-ninth answered it for DSA
(ADR 0314), the four-hundred-and-eighty-seventh for the RSA family's other padding, RSASSA-PSS
(ADR 0322), and the six-hundred-and-eighty-ninth for the two elliptic-curve families (ADR 0532).

| | asks | needs | state |
|---|---|---|---|
| **1. Integrity** | has the document changed since it was signed? | the file and a hash function | **answered** |
| **2. Authenticity** | does the signature verify under the signer's public key? | an X.509 certificate parser and RSA, DSA, ECDSA or EdDSA | **answered for every family**; four curves below |
| **3. Trust** | is the signer anyone to believe, and was the certificate revoked? | a trust store, a certification path, a network | open, and it is a project |

Question 1 is `Signature::integrity`, question 2 is `Signature::authenticity`.

### What is left of question 2: four curves, and each is a package rather than a clause

**Everything here is *reported* at runtime by the object identifier the file states**, never
skipped: `Authenticity::AlgorithmNotVerifiable`, `Authenticity::KeyNotVerifiable`,
`Authenticity::CurveNotVerifiable` and `Authenticity::UnknownDigest` each carry the number, printed
as dotted decimal by `x509::dotted` rather than as a word, because this tree holds ISO 32000-2 and
not the documents that assign those numbers.

ISO/TS 32002 section 5.1.3's Table 3 names six ECDSA curves and its Table 4 two EdDSA ones.
`pdf_model::ecdsa` computes P-256, P-384 and P-521; `pdf_model::eddsa` computes Ed25519. The other
four are refused, and **the refusal is a fact about crates.io rather than about the standard**
(measured 2026-08-23, ADR 0532 — re-derive it before believing it):

| curve | why not | what would change it |
|---|---|---|
| brainpoolP256r1 | `bp256` is 0.14.0-**rc.15**; its stable 0.6 is the old `digest` line | a 0.14.0 release |
| brainpoolP384r1 | `bp384`, the same | a 0.14.0 release |
| brainpoolP512r1 | **no crate on crates.io at all** | somebody publishing one |
| Ed448 | `ed448-goldilocks` stable 0.9.0 has the field arithmetic and no signature scheme, on `rand_core` 0.6; 0.14 is `-pre.15` | a 0.14.0 release |

A round taking any of these should take the *curve* rather than the crate that exists: the set is
ISO/TS 32002 Table 3's and Table 4's, and `ecdsa::UnsupportedCurve` already names all three
Brainpool ones by the identifier `const_oid` reads out of RFC 5639.

**Two blockers this file used to carry are retired, and both were retired by reading rather than by
a release.**

- **BSI TR-03111's plain encoding is not a principle-5 blocker and never had to be.** Two corpus
  signatures state `0.4.0.127.0.7.1.1.4.1.3` with the value as fixed-width `r ‖ s`. This file
  recorded that as needing a document the tree does not hold. **ISO/TS 32002 section 5.1.3's NOTE 2
  settles it the other way**: "[t]his restriction implies that ECDSA signature values are required
  to be represented using the DER-encoded ECDSA-Sig-Value type in IETF RFC 5753:2010, section 7.2."
  Those two files are outside what the Technical Specification admits, and reporting them by their
  own algorithm identifier **is** the correct behaviour rather than a gap. Do not re-open this as a
  debt.
- **"The domain parameters are in no document this tree holds"** died in the
  four-hundred-and-ninety-sixth session (ADR 0331) and is fully spent now: the identifiers come from
  `const-oid`'s database — a second party's reading of the registries, at zero new packages, since
  it is already here through `digest` — and the curve constants from the curve packages.

### The one thing a round *could* still get from this family

`ecdsa::is_ecdsa` recognises RFC 9688's `id-ecdsa-with-sha3-256`, `-384` and `-512`, which ISO/TS
32002 Table 3 pairs with every curve, and `cms::Digest` computes all three SHA-3 digests — so a
SHA-3 ECDSA signature is verified today by the same path. **No document in the population states
one**, so nothing here is exercised by a real file, and that is a fact about documents rather than
about the code.

### ISO/TS 32001's four digests — done in the five-hundred-and-fifty-fifth session (ADR 0390)

`cms::Digest` computes SHA3-256, SHA3-384, SHA3-512 and SHAKE256 beside the base standard's six, on
`sha3` 0.12 and `shake` 0.1. Four things it left behind that a later round should not have to
rediscover:

- **Read ISO/TS 32001's errata before writing anything about it**, with `spec-errata emit` and not
  `doc/md/`. Two annotations amend it and neither is in the conversion: issue #236 **deletes clause
  5.1.3 entirely**, so Table 256's `/DigestMethod` is *not* extended with the SHA-3 family (this
  file said it was, for three sessions), and issue #404 strikes the sentence pinning `id-shake256`
  and defers to RFC 8702 and RFC 8419 instead. **The same errata run on ISO/TS 32002 finds #404
  again**, striking the matching footnotes in its section 5.1.2 and nothing that touches Table 3,
  Table 4 or NOTE 2.
- **`Digest::ALL` is ten and `Digest::TRIED_WHEN_UNSTATED` is six**, and the split is §5.1.4's own:
  it adds its four "to the Message Digest value entry for adbe.pkcs7.detached, ETSI.CAdES.detached
  or ETSI.RFC3161", and §12.8.3.2's `adbe.x509.rsa_sha1` — the sub-filter whose digest has to be
  found by trying each in turn — is not one of the three.
- **The object identifiers were transcribed from a registry no document here holds, and nine of
  the ten now carry a second reading anyway** — `const_oid`'s database, which arrived for the
  elliptic-curve family and turned out to hold these too (ADR 0532). RIPEMD-160 is the exception
  and stays one: nothing in this graph publishes TeleTrusT's arc. SHAKE256's 512-bit output is
  still a documented choice rather than a requirement.
- **Table 256's `/DigestMethod` is read by nothing**, which this item found on its way past. It
  costs no mark — the digest it names belongs to §12.8.2.2.2's comparison, which is not done — and
  §12.8.1's ledger row now says so. What is missing there is a reader for a *name*, over the base
  standard's six.

### What question 3 would take, and it is still a project

A certificate store — the platform's, or one shipped and maintained — a certification path
validation per RFC 5280 clause 6, and either a CRL fetch or an OCSP request, which is a network in
the renderer's address space and therefore a security argument as well as a feature. §12.8.4's
document security store already tells this program whether a document *carries* what a validator
would need, and §12.8.3.3.2's revocation attribute is named where a signature carries one; using
either is what is missing. Add to it a policy for what a viewer does with a signature that fails,
which `doc/todo/38`'s four levels are the natural shape for.

**What already exists that question 3 would build on**: `pdf_model::x509` reads a certificate's
issuer, subject, serial number and key, and `Certificate::is_named_by` matches a signer to one.
What it deliberately does *not* read is every field a trust decision needs — validity dates, basic
constraints, key usage, the issuer's signature over the certificate — and that is a choice to
revisit rather than an oversight: reading a `notAfter` while saying nothing about who issued the
certificate would put an air of validation over a certificate the file's author could have written
five minutes ago.

**And question 3 is now the *only* thing between this clause and `implemented`,** which it was not
before: every `partial` in the §12.8.3 family names either trust or one of the four curves above.

## Public-key handlers (§7.6.5) — 5 corpus documents, and none of them a reader's

CMS enveloped data, X.509, the user's private keys — an infrastructure and a threat model, not a
cipher. The standard security handler (§7.6.3, §7.6.4) is complete in both directions at every
revision and method, so this is the *other* handler family.

**"0 corpus documents" was this heading for a long time and it is five**, counted in the
eight-hundred-and-ninety-second session over everything this tree can reach — 90 535 documents,
of which 2 374 name `/Encrypt` in their bytes and 2 360 state one in a trailer (ADR 0829):

```sh
find -L doc/pdf.js/test/pdfs doc/corpora corpus-cache -type f -iname '*.pdf' > /tmp/all
xargs -a /tmp/all -d '\n' grep -lF /Encrypt > /tmp/enc
xargs -a /tmp/enc -d '\n' cargo run --profile gates -p pdf-model --example encryption_census
```

| document | corpus | `/V` | `/CFM` | key bits |
|---|---|---|---|---|
| `3006236.pdf` | SafeDocs `cc-main-2021-31` | 5 | `AESV3` | 256 |
| `PDFBOX-4421-0.pdf` | tika-issue-tracker `batch1` | 4 | `AESV2` | 128 |
| `PDFBOX-4421-1.pdf` | tika-issue-tracker `batch1` | 5 | `AESV3` | 256 |
| `PDFBOX-4421-2.pdf` | tika-issue-tracker `batch1` | 4 | `AESV2` | 128 |
| `PDFBOX-4421-3.pdf` | tika-issue-tracker `batch1` | 5 | `AESV3` | 256 |

All five are `/Filter /Adobe.PubSec` with `/SubFilter /adbe.pkcs7.s5`, a crypt filter named
`DefaultCryptFilter` — which is what §7.6.4.1 requires of a public-key handler "when all document
content is encrypted" — and a `/Recipients` array of exactly one CMS `EnvelopedData` carrying a
single `KeyTransRecipientInfo` under RSA. All five are refused by name today.

**Five documents is still not demand, and the reason is stronger than the number.** Four are one
bug report's attachments — PDFBOX-4421 is Apache's own public-key issue — and the fifth,
`3006236.pdf`, names its recipient's certificate
`zune-tuner://windowsphone/b46fd244 - cd539804 - e37e34e4 - 6f90f8c0`: a document encrypted to a
*device*, whose private key was never a reader's. §7.6.5.1 puts one `shall` on a reader — "scan
the recipient list for which the content is encrypted and … attempt to find a match with a
certificate that belongs to the user" — and for every one of these five the honest outcome of that
scan is *no match*. Implementing the clause would turn five loud refusals into five loud refusals
with a certificate store behind them. **So this stays where it is, and the count is now a fact
rather than a guess.**

Two more documents state a `/Filter` that is neither `/Standard` nor a handler, and neither is
this clause's — recorded here because this is where somebody will look for them:

- `PDFBOX-4351-0.pdf` writes `/Filte^/Standard` — one byte of the **key name** corrupted — so the
  encryption dictionary states no `/Filter` at all and the refusal names the nearest one the
  reader does find, `/FlateDecode`. The refusal is right; its message points at the wrong entry,
  and a `/Filter` that is absent could say so instead of naming a stream filter.
- `GHOSTSCRIPT-695040-0.zip-77.pdf` carries a well-formed `/Filter /Standard /V 1 /R 2` dictionary
  in its body that the trailer's `/Encrypt` does not reach, so the reader sees an absent
  `/Filter`. A cross-reference recovery question rather than an encryption one.

**And the three-hundred-and-ninety-second session read this against what it built, with a result
that is smaller than it sounds.** `der`, `cms` and `x509` are the parsing half of §7.6.5 — perhaps
half of that half. What the clause needs on top is RFC 5652's `EnvelopedData` rather than
`SignedData`, which is a different structure with recipient information in it; an RSA *decryption*
rather than a verification, which is a private-key operation and therefore the one place in this
subject where constant time matters and ADR 0229's "there is no secret" argument reverses; and a way
to reach the reader's own private key, which is where the threat model starts and where nothing has
been decided.

**That reversal is sharper now than when it was written**, and a round taking §7.6.5 should notice
it: every dependency §12.8.3 runs on was chosen with the `_vartime` spelling on purpose, because a
verifier has no secret. A decryption does, and none of those choices carries over.

## `/R` 5 — implemented, and this section is what it replaced

**This heading used to read "`/R` 5 — 1 document" and the paragraph under it said "Table 21 says
`/R` 5 \"shall not be used\" and states no algorithm, so there is nothing to implement."** The
eight-hundred-and-eighty-seventh session read that sentence as binding a *writer*, found that
Table 21's "deprecated proprietary Adobe extension" is a **pointer** rather than an absence, and
implemented the revision (ADR 0820). 41 of the 90 535 documents state it; 33 open, 8 want a
password nobody here has, 0 are refused.

The eight-hundred-and-ninety-second session then fetched the extension itself — the Adobe
Supplement to ISO 32000-1, `BaseVersion` 1.7, `ExtensionLevel` 3 — and every step ADR 0820 derived
without it agrees with its Algorithm 3.2a, including the password preparation that was the one
step resting on a reading (ADR 0829). Nothing is owed here.

**What the corpus's one remaining `/R` 5 encoding refusal is about is a different clause.** A
revision-4 password containing a character `PDFDocEncoding` has no code for is refused by
§7.6.4.3.2 step (a)'s conversion, which uses the whole of Annex D Table D.3 and still has no code
for U+00A0. (The row that said "this crate holds no Annex D table" was wrong for a hundred and
twenty-nine sessions — `text_string.rs` had held it since the ninety-second, put there for
§7.9.2.2. See `01-ledger-partial-rows.md`.)
