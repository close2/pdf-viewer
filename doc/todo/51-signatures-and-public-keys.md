# A signature's third question, DSA and ECDSA, public-key handlers, `/R` 5

Status: **one of three questions left, and it is the one that is a project**; two algorithm families
of Table 260's three are unimplemented and named at runtime.
Priority: 51
Corpus: 1 document (`/R` 5); 9 carry a signature dictionary, 10 between them, **all ten RSA**
Clauses: §12.8.3, §7.6.5, §7.6.4.3, Table 21
Code: `crates/pdf-model/src/signature.rs`, `crates/pdf-model/src/cms.rs`,
`crates/pdf-model/src/der.rs`, `crates/pdf-model/src/x509.rs`, `crates/pdf-model/src/pkcs1.rs`,
`crates/pdf-syntax/src/crypt.rs`

## Signature validation (§12.8.3) — 5 ledger rows, and it used to be 17

**This file used to say the whole clause needed "a trust store and a network". That was true of one
of the three questions a signature asks and false of the other two**; the three-hundred-and-seventy-seventh
session separated them and answered the first (ADR 0215), and the three-hundred-and-ninety-second
answered the second (ADR 0229).

| | asks | needs | state |
|---|---|---|---|
| **1. Integrity** | has the document changed since it was signed? | the file and a hash function | **answered** |
| **2. Authenticity** | does the signature verify under the signer's public key? | an X.509 certificate parser and RSA, DSA or ECDSA | **answered for RSA**; the other two below |
| **3. Trust** | is the signer anyone to believe, and was the certificate revoked? | a trust store, a certification path, a network | open, and it is a project |

Question 1 is `Signature::integrity`, question 2 is `Signature::authenticity`. All ten of the
corpus's signature dictionaries verify — 1024-bit ×3, 2048-bit ×6, 4096-bit ×1, in 4.583 ms between
them — and each stops verifying when one bit of its signature value is turned over.

### What is left of question 2: two of Table 260's three algorithm families

**The row this file was missing.** Table 260 names three, not two: "RSA Algorithm Support", "DSA
Algorithm Support | Up to 4096-bits (PDF 1.6)" and "ECDSA Algorithm Support ( defined by Internet
RFC 5480 )". This file listed RSA and ECDSA and forgot DSA for fifteen sessions.

Neither is implemented and both are **named at runtime by the object identifier the file states** —
`Authenticity::KeyNotVerifiable` and `Authenticity::AlgorithmNotVerifiable`, printed as dotted
decimal by `x509::dotted` rather than as a word, because this tree holds ISO 32000-2 and not the
documents that assign those numbers.

**No corpus document needs either.** All eleven signature values the nine signed documents hold are
RSA, checked by reading the `signatureAlgorithm` out of each. So this is spec-track work with no
demand-track witness, and ADR 0229 says what would change the dependency answer the day one arrives:

- **ECDSA** is where ADR 0031's argument for taking a reviewed implementation *does* have an
  instance — five curves' field arithmetic, point addition, doubling and inversion — so take
  `p256`/`p384` (and `p224`/`p521`/`p192` as RFC 5480's list requires) rather than writing it, and
  argue the packages then rather than now.
- **DSA** is a modular exponentiation over the big integer `pdf_model::pkcs1` already has, plus a
  modular inverse; `rsa`'s RustCrypto sibling `dsa` is on the old `digest` 0.10 line and would cost
  the second hash stack ADR 0229 declined. In tree is the likelier answer, and it needs a witness
  before it needs code.
- **RSASSA-PSS** (`id-RSASSA-PSS`, `1.2.840.113549.1.1.10`) is deliberately *not* treated as
  PKCS #1 v1.5 — it is the same OID arc and a different padding — and reaches the same report.

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
that is smaller than it sounds.** `der`, `cms` and now `x509` are the parsing half of §7.6.5 —
perhaps half of that half. What the clause needs on top is RFC 5652's `EnvelopedData` rather than
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
