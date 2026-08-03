# Signature validation, public-key handlers, `/R` 5

Status: **blocked on infrastructure this program does not have**, and each says so at runtime.
Priority: 51
Corpus: 1 document (`/R` 5); 8 carry a signature dictionary
Clauses: §12.8.3, §7.6.5, §7.6.4.3, Table 21
Code: `crates/pdf-model/src/signature.rs`, `crates/pdf-syntax/src/crypt.rs`

## Signature validation (§12.8.3) — 17 ledger rows

Needs a **trust store and a network**. What a program without one can honestly say is said (ADRs
0088, 0089): who signed, when they claim to have signed, whether the `/ByteRange` covers the
whole file and how many bytes were appended after it, and — since the hundred-and-ninety-first
session — what §12.8.2.2's `/P` permits, which this program now *obeys* rather than only reports.
Every document that opens carries the sentence "signatures are not verified — this program has no
certificate store".

Taking it means a certificate store, a revocation story, and a policy for what a viewer does with
an invalid signature. It is a project, not a feature.

## Public-key handlers (§7.6.5) — 0 corpus documents

CMS enveloped data, X.509, the user's private keys — an infrastructure and a threat model, not a
cipher. The standard security handler (§7.6.3, §7.6.4) is complete in both directions at every
revision and method, so this is the *other* handler family and nothing in the corpus asks.

## `/R` 5 — 1 document

Table 21 says `/R` 5 "shall not be used" and states no algorithm, so there is nothing to
implement. The one corpus witness is refused by the **encoding** rather than by the revision:
§7.6.4.3.2 step (a)'s conversion uses the whole of Annex D Table D.3, which has no code for
U+00A0 at all. (The row that said "this crate holds no Annex D table" was wrong for a hundred and
twenty-nine sessions — `text_string.rs` had held it since the ninety-second, put there for
§7.9.2.2. See `01-ledger-partial-rows.md`.)
