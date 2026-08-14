# ECDSA and EdDSA, four digests, the third question, public-key handlers, `/R` 5

Status: **two of Table 260's three algorithm families are verified, the RSA one under both of RFC
8017's paddings**; what is left of question 2 is the elliptic-curve family and four hash functions
— and question 3 is still a project.
Priority: 51
Corpus: 1 document (`/R` 5). For the signature populations, **run the census rather than reading a
number here**:

```sh
find corpus-cache doc/corpora doc/pdf.js/test/pdfs -name '*.pdf' > /tmp/paths
cargo run --release -p pdf-model --example signature_algorithm_census -- @/tmp/paths
```

Clauses: §12.8.3, §7.6.5, §7.6.4.3, Table 21, Table 256, Table 260; ISO/TS 32001 §5.1, ISO/TS 32002 §5.1
Code: `crates/pdf-model/src/signature.rs`, `crates/pdf-model/src/cms.rs`,
`crates/pdf-model/src/der.rs`, `crates/pdf-model/src/x509.rs`, `crates/pdf-model/src/pkcs1.rs`,
`crates/pdf-model/src/pss.rs`, `crates/pdf-model/src/dsa.rs`, `crates/pdf-model/src/bigint.rs`,
`crates/pdf-syntax/src/crypt.rs`

## Signature validation (§12.8.3) — 5 ledger rows, and it used to be 17

**This file used to say the whole clause needed "a trust store and a network". That was true of one
of the three questions a signature asks and false of the other two**; the three-hundred-and-seventy-seventh
session separated them and answered the first (ADR 0215), the three-hundred-and-ninety-second
answered the second for RSA (ADR 0229), the four-hundred-and-seventy-ninth answered it for DSA
and refused the elliptic-curve family with an argument (ADR 0314), and the
four-hundred-and-eighty-seventh answered it for the RSA family's other padding, RSASSA-PSS
(ADR 0322).

| | asks | needs | state |
|---|---|---|---|
| **1. Integrity** | has the document changed since it was signed? | the file and a hash function | **answered** |
| **2. Authenticity** | does the signature verify under the signer's public key? | an X.509 certificate parser and RSA, DSA or ECDSA | **answered for RSA — both paddings — and DSA**; the rest below |
| **3. Trust** | is the signer anyone to believe, and was the certificate revoked? | a trust store, a certification path, a network | open, and it is a project |

Question 1 is `Signature::integrity`, question 2 is `Signature::authenticity`.

### What is left of question 2, in the order the population ranks it

**And the order binds: no further zero-witness algorithm family before a witnessed one.** DSA went
in with zero corpus signatures using it while PSS — then the commonest thing this program declined,
at twice ECDSA's share — sat behind it in this file. The work was sound (ADR 0314) and the ordering
was not: on `CLAUDE.md`'s two tracks, a second consecutive spec-side item in one family while the
demand side of the same family has real witnesses is the balance this file exists to keep. The
four-hundred-and-eighty-seventh session paid that debt: `pdf_model::pss` verifies `id-RSASSA-PSS`
and the census's six witnesses all answer `Verified` (ADR 0322).

**Everything here is *reported* at runtime by the object identifier the file states**, never
skipped: `Authenticity::AlgorithmNotVerifiable`, `Authenticity::KeyNotVerifiable` and
`Authenticity::UnknownDigest` each carry the number, printed as dotted decimal by `x509::dotted`
rather than as a word, because this tree holds ISO 32000-2 and not the documents that assign those
numbers.

**1. ECDSA and EdDSA — refused-for-now, re-grounded in the four-hundred-and-ninety-sixth session
(ADR 0331) after the owner's arithmetic decision killed the refusal's heaviest premise.**
(RSASSA-PSS stood here until the four-hundred-and-eighty-seventh session closed it: `crates/pdf-model/src/pss.rs`
is RFC 8017 sections 8.1.2 and 9.1.2 with Appendix B.2.1's MGF1 over `crate::bigint`, reading the
`RSASSA-PSS-params` from the `AlgorithmIdentifier`, kept separate from PKCS #1 v1.5 as this file
insisted — ADR 0322. And `crate::bigint` itself is a seam over RustCrypto's `crypto-bigint` since
the four-hundred-and-ninety-sixth, by owner decision — ADR 0331.) The short form on the
elliptic-curve family, on 2026-08-14's measurements:

- The standard family names **eight curves**, not five: ISO/TS 32002 Table 3 gives P-256, P-384,
  P-521, brainpoolP256r1, brainpoolP384r1 and brainpoolP512r1 for ECDSA, and its Table 4 adds
  Ed25519 (SHA512) and Ed448 (SHAKE256) for EdDSA, which is a second and unrelated group law.
- **"Their domain parameters are in no document this tree holds" no longer decides anything.**
  It was ADR 0314's first premise; once the owner accepted reviewed *arithmetic* as a dependency
  (ADR 0331), reviewed *constants* in a curve crate stand on the same footing. Do not cite that
  sentence as the blocker again — the blockers below are the live ones.
- **Stable-line coverage refuses.** `p256`/`p384`/`p521` 0.14.0 are now stable on this tree's
  `digest` 0.11 line (measured in ADR 0331's scratch crate — the pre-release objection has
  expired for the NIST curves, at about twenty new packages for the first one). But the Brainpool
  pair is release-candidate-only on that line (`bp256` 0.14.0-rc; its stable 0.6 is the old hash
  line), brainpoolP512r1 has no crate at all, and Ed448 none either. TS 32002 section 5.1.3
  requires `namedCurve` and permits a processor to "ignore or handle in an implementation-dependent
  manner" a document signed with a curve outside those tables, which caps the set without making
  it small.
- **The witnesses are now identified by curve** (ADR 0331 extracted the three signers'
  certificates): the one DER-encoded `ecdsa-with-SHA256` signature is **P-256**; the two BSI
  TR-03111 *plain* signatures (`0.4.0.127.0.7.1.1.4.1.3`, `r ‖ s` as fixed-width octets rather
  than RFC 3279's DER `Dss-Sig-Value`) are one **brainpoolP256r1** and one **P-256**. The plain
  encoding is defined in BSI TR-03111, a document this tree does not hold — a principle-5 blocker
  independent of any crate. So the stable packages would close **one signature of 811**.

What would change it: a stable Brainpool pair on the current line **plus** the BSI TR-03111 text
in `doc/` (which would close all three witnesses at once), or a population that makes the family
more than a rounding error. Take the curves TS 32002 Table 3 lists, not the ones a crate happens
to publish — and EdDSA's zero witnesses queue it behind the witnessed three, by this file's own
ordering rule above.

**2. ISO/TS 32001's four digests.** §5.1.4 adds SHA3-256, SHA3-384, SHA3-512 and SHAKE256 to Table
260's Message Digest row and §5.1.3 adds the same four to Table 256's `/DigestMethod`, with
SHAKE256 pinned to `id-shake256` so its output is fixed at 512 bits. `cms::Digest` computes the six
the base standard names and none of these; a signature stating one reports the identifier. This is
the cheapest of the three to close and the only one that needs a **new dependency** — a SHA-3
implementation on this tree's `digest` 0.11 line — so it is a `doc/stack.md` question rather than an
arithmetic one. No corpus document states one.

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

## Public-key handlers (§7.6.5) — 0 corpus documents

CMS enveloped data, X.509, the user's private keys — an infrastructure and a threat model, not a
cipher. The standard security handler (§7.6.3, §7.6.4) is complete in both directions at every
revision and method, so this is the *other* handler family and nothing in the corpus asks.

**And the three-hundred-and-ninety-second session read this against what it built, with a result
that is smaller than it sounds.** `der`, `cms` and `x509` are the parsing half of §7.6.5 — perhaps
half of that half. What the clause needs on top is RFC 5652's `EnvelopedData` rather than
`SignedData`, which is a different structure with recipient information in it; an RSA *decryption*
rather than a verification, which is a private-key operation and therefore the one place in this
subject where constant time matters and ADR 0229's "there is no secret" argument reverses; and a way
to reach the reader's own private key, which is where the threat model starts and where nothing has
been decided.

## `/R` 5 — 1 document

Table 21 says `/R` 5 "shall not be used" and states no algorithm, so there is nothing to
implement. The one corpus witness is refused by the **encoding** rather than by the revision:
§7.6.4.3.2 step (a)'s conversion uses the whole of Annex D Table D.3, which has no code for
U+00A0 at all. (The row that said "this crate holds no Annex D table" was wrong for a hundred and
twenty-nine sessions — `text_string.rs` had held it since the ninety-second, put there for
§7.9.2.2. See `01-ledger-partial-rows.md`.)
