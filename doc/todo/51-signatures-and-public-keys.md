# A signature's second and third questions, public-key handlers, `/R` 5

Status: **two of three questions blocked on infrastructure this program does not have**, and each
says so at runtime.
Priority: 51
Corpus: 1 document (`/R` 5); 9 carry a signature dictionary, 10 between them
Clauses: §12.8.3, §7.6.5, §7.6.4.3, Table 21
Code: `crates/pdf-model/src/signature.rs`, `crates/pdf-model/src/cms.rs`,
`crates/pdf-model/src/der.rs`, `crates/pdf-syntax/src/crypt.rs`

## Signature validation (§12.8.3) — 7 ledger rows, and it used to be 17

**This file used to say the whole clause needed "a trust store and a network". That was true of one
of the three questions a signature asks and false of another**, and the three-hundred-and-seventy-seventh
session separated them (ADR 0215):

| | asks | needs | state |
|---|---|---|---|
| **1. Integrity** | has the document changed since it was signed? | the file and a hash function | **answered** |
| **2. Authenticity** | does the signature verify under the signer's public key? | an X.509 certificate parser and RSA or ECDSA | open, below |
| **3. Trust** | is the signer anyone to believe, and was the certificate revoked? | a trust store, a certification path, a network | open, and it is a project |

Question 1 is `Signature::integrity`: the digest over §12.8.1's `/ByteRange`, with the six
algorithms Table 260 and Table 256 name, compared with what `pdf_model::cms` reads out of
§12.8.3.3's `SignedData` — the `message-digest` attribute, an `adbe.pkcs7.sha1`'s encapsulated
digest, or an `ETSI.RFC3161` token's `messageImprint`. Nine corpus documents carry ten signature
dictionaries; five still hash to what they record, **four do not**, and one records no digest in the
open at all. Six of §12.8.3.4's structural rules are checked with it, because they need no
certificate either.

### What question 2 would take, and it is a decision rather than a project

An X.509 certificate parser over the `certificates` the CMS object already carries — this tree
counts them and reads none — plus an RSA PKCS #1 v1.5 verification and an ECDSA one over the curves
RFC 5480 names, and the re-encoding of `SignedAttributes` as a `SET OF` that CMS signs. The digest
side of it is done; what is missing is a big-integer modular exponentiation and a point
multiplication.

**It is a dependency decision nobody has been asked for**, in ADR 0186's and ADR 0214's shape: `rsa`
and `p256`/`p384` from RustCrypto are the obvious candidates, both `MIT OR Apache-2.0`, both pure
Rust; `x509-cert` and `der` would replace `pdf_model::der` with a general ASN.1 compiler, which is a
bigger change than the verification itself and would want its own argument. What answering question 2
buys is precisely this: **a matching digest would stop being "the absence of evidence against the
file" and become evidence about the signer's key.** It would still not say who that signer is.

### What question 3 would take, and it is still a project

A certificate store — the platform's, or one shipped and maintained — a certification path
validation per RFC 5280 clause 6, and either a CRL fetch or an OCSP request, which is a network in
the renderer's address space and therefore a security argument as well as a feature. §12.8.4's
document security store already tells this program whether a document *carries* what a validator
would need, and §12.8.3.3.2's revocation attribute is named where a signature carries one; using
either is what is missing. Add to it a policy for what a viewer does with an invalid signature, which
`doc/todo/38`'s four levels are the natural shape for.

## Public-key handlers (§7.6.5) — 0 corpus documents

CMS enveloped data, X.509, the user's private keys — an infrastructure and a threat model, not a
cipher. The standard security handler (§7.6.3, §7.6.4) is complete in both directions at every
revision and method, so this is the *other* handler family and nothing in the corpus asks. The
`der` and `cms` readers built for §12.8.3 are half of what it would need to *parse*; the private key
half is where the threat model starts.

## `/R` 5 — 1 document

Table 21 says `/R` 5 "shall not be used" and states no algorithm, so there is nothing to
implement. The one corpus witness is refused by the **encoding** rather than by the revision:
§7.6.4.3.2 step (a)'s conversion uses the whole of Annex D Table D.3, which has no code for
U+00A0 at all. (The row that said "this crate holds no Annex D table" was wrong for a hundred and
twenty-nine sessions — `text_string.rs` had held it since the ninety-second, put there for
§7.9.2.2. See `01-ledger-partial-rows.md`.)
