# ADR 0314 — The family whose parameters are in the file

Status: accepted, 2026-08-13 (session 479). Answers Table 260's second algorithm family and
refuses its third with an argument. Follows ADR 0215 (question 1) and ADR 0229 (question 2, RSA).

## Context

ISO 32000-2's Table 260 gives a PDF signature three public-key algorithm families. ADR 0229
implemented the first and named the other two at runtime by the object identifier the file states.
`doc/todo/51` recorded the debt as one decision — "take `p256`/`p384` … the day one arrives" for
ECDSA, "in tree is the likelier answer" for DSA — and left the choice between them to a later
round on the grounds that the corpus had neither.

This round was asked to take one of the two and say why. Two things decided it, and only one of
them was the corpus.

## The reading: the algorithm set is larger than this tree thought, and it is not evenly shaped

Table 260, in ISO 32000-2's own words, per `/SubFilter` column:

| | `adbe.pkcs7.detached`, `ETSI.CAdES.detached` or `ETSI.RFC3161` | `adbe.pkcs7.sha1` | `adbe.x509.rsa_sha1` |
|---|---|---|---|
| Message Digest | SHA1 (PDF 1.3), SHA256 (PDF 1.6), SHA384 (PDF 1.7), SHA512 (PDF 1.7), RIPEMD160 (PDF 1.7) | SHA1 (PDF 1.3) | the same five |
| RSA Algorithm Support | Up to 1024-bit (PDF 1.3) / 2048-bit / 4096-bit (PDF 1.5) | See `adbe.pkcs7.detached` | See `adbe.pkcs7.detached` |
| DSA Algorithm Support | Up to 4096-bits (PDF 1.6) | See `adbe.pkcs7.detached` | **No** |
| ECDSA Algorithm Support ( defined by Internet RFC 5480 ) | ANSI X9.62 … (PDF 2.0 ) | No | No |

Four things fall out of reading it beside the two Technical Specifications that amend it, and three
of them were not known to this tree before.

**1. Table 260 says DSA is not permitted for `adbe.x509.rsa_sha1`, and this tree's ledger said the
opposite.** §12.8.3.2's row has read, since ADR 0229, that "PKCS #1 also 'supports … DSA
signatures', which is not implemented and is named by its object identifier" — treating that
clause's sentence as a gap this program owed. It is not one. §12.8.3.2's sentence is about what the
PKCS #1 *standard* supports; Table 260 is what says which of those a given `/SubFilter` may carry,
and for this one it says **No**. So DSA needed implementing in the CMS path and nowhere else, and a
`/Cert` holding a DSA key is a file departing from the table rather than a case owed an
implementation. `pkcs1_authenticity` reports such a key by its number, with the table's "No" in the
comment above the arm.

**2. ISO/TS 32002:2022 rewrites the ECDSA row, and it more than doubles it.** Its section 5.1.1
says the document "extends the elliptic curve digital signature support in Table 260 to add support
for more recent ECDSA curves, as defined in IETF RFCs 5639 and 6932, and to add support for
Edwards-curve Digital Signature Algorithm (EdDSA) based digital signatures as defined in IETF RFC
8419." Its Table 3 enumerates the ECDSA curves — P-256, P-384, P-521, brainpoolP256r1,
brainpoolP384r1, brainpoolP512r1 — with the digests each admits, and its Table 4 adds Ed25519 with
SHA512 and Ed448 with SHAKE256. Its section 5.1.2 adds an EdDSA row to Table 260 itself.

So "ECDSA" in the standard family is **eight curves in two unrelated groups**: six short Weierstrass
prime curves, three of which are Brainpool and therefore have `a ≠ −3`, and two Edwards curves whose
group law is a different construction altogether. ADR 0229 wrote "five curves' field arithmetic" and
`doc/todo/51` wrote "`p256`/`p384` (and `p224`/`p521`/`p192` as RFC 5480's list requires)". Both were
counting RFC 5480's list without the Technical Specification that narrows and extends it.

**3. ISO/TS 32001:2022 adds four digests, and this tree computes none of them.** Its section 5.1.4
adds "SHA3-256 (PDF 2.x)", "SHA3-384 (PDF 2.x)", "SHA3-512 (PDF 2.x)" and "SHAKE256 (PDF 2.x)" to
Table 260's Message Digest row, and its section 5.1.3 adds the same four to Table 256's
`/DigestMethod`. `cms::Digest`'s documentation said the six it implements were "the digest
algorithms Table 260 and Table 256 name, and nothing else" — a claim about the standard that had
decayed, in `CLAUDE.md` principle 5's exact shape. Corrected, and the gap is now *named*: a
signature stating one of the four reaches `Authenticity::UnknownDigest` carrying the identifier
rather than a shrug.

**4. TS 32002 bounds the work in the one direction that helps.** Its section 5.1.3 requires
`namedCurve` and forbids `implicitCurve` and `specifiedCurve`, and ends: "PDF processors may ignore
or handle in an implementation-dependent manner PDF documents which are signed with elliptic curves
not listed in Table 3 or Table 4." That is a real permission and it caps the curve set at eight. It
does not make eight small.

**And the difference that decided the round is not size.** *Every number DSA verification needs is
in the file.* FIPS 186-4 section 4.7 computes `v = ((g^u1 y^u2) mod p) mod q` from `p`, `q`, `g` and
`y`, and RFC 3279 section 2.3.2 puts all four in the certificate's own `subjectPublicKeyInfo` —
`Dss-Parms` and `DSAPublicKey`. An elliptic-curve verification needs the **domain parameters of a
named curve**, and those are in no document this tree holds: not in ISO 32000-2, not in either
Technical Specification, not in an RFC this project has any claim on. Writing P-256's prime into a
Rust file would be writing a specification constant from memory, which is the thing `CLAUDE.md`
principle 5 exists to prevent and which `doc/HANDOVER.md` says outright about `doc/md/`: extract
spec data from there rather than writing it from memory.

## The population, measured before anything was built

`crates/pdf-model/examples/signature_algorithm_census.rs`, over **67 460 documents** — `doc/pdf.js`'s
974, `doc/corpora`'s 275 and all 66 211 of the SafeDocs crawl — reading three identifiers per
signature, because a producer can get them out of step: the `SignerInfo`'s `signatureAlgorithm`, its
`digestAlgorithm`, and the algorithm of the key in the certificate that `SignerInfo` names.

**811 signature dictionaries in 681 documents.** Their `signatureAlgorithm`s:

| identifier | | count |
|---|---|---|
| `1.2.840.113549.1.1.1` | `rsaEncryption` | 490 |
| `1.2.840.113549.1.1.11` | `sha256WithRSAEncryption` | 279 |
| `1.2.840.113549.1.1.5` | `sha1WithRSAEncryption` | 10 |
| `1.2.840.113549.1.1.13` | `sha512WithRSAEncryption` | 8 |
| `1.2.840.113549.1.1.10` | `id-RSASSA-PSS` | **6** |
| `0.4.0.127.0.7.1.1.4.1.3` | BSI TR-03111's plain ECDSA with SHA-256 | **2** |
| `1.2.840.10045.4.3.2` | `ecdsa-with-SHA256` | **1** |

and the signers' certificates hold 793 RSA keys (146 of 1024 bits, 634 of 2048, 4 of 3072 and 9 of
4096) and **3 `id-ecPublicKey`**. Fifteen more values are not CMS objects at all, which is what
`adbe.x509.rsa_sha1` and one file's `urn:pdfsigfilter:bka.gv.at:binaer:v1.1.0` are. **Not one document in 67 460 states a DSA key or a DSA signature algorithm.**

Three readings of that, and they pull in different directions, which is why they are all written
down.

- **DSA has no demand witness at all.** It is spec-track work, and `CLAUDE.md`'s two-denominator
  rule says that is a reason to do it deliberately rather than a reason not to: a corpus cannot rank
  a requirement no document exercises.
- **ECDSA's witness is three signatures in 811, and two of the three are not what a dependency would
  read.** `0.4.0.127.0.7.1.1.4.1.3` is BSI TR-03111's *plain* ECDSA, whose signature value is `r ‖ s`
  as fixed-width octets rather than RFC 3279's DER `Dss-Sig-Value` — a different encoding on top of
  a different curve set. So the candidate packages would close one signature of 811 today.
- **The finding nobody was looking for: `id-RSASSA-PSS` is twice as common as ECDSA, and it is
  inside the family this tree calls done.** Table 260's "RSA Algorithm Support" row states key sizes
  and names no padding, and ETSI's CAdES profiles permit PSS. Six real signatures use it and this
  program declines all six by number. That is a demand-track item with more evidence behind it than
  either of the two this round was choosing between, it needs no dependency and no external
  constant — RFC 8017 section 8.1.2's EMSA-PSS-VERIFY over the modular exponentiation that already
  exists — and it is now the top of `doc/todo/51` rather than a thing nobody had counted.

## Decision 1 — implement DSA, in tree, with no dependency

`crates/pdf-model/src/dsa.rs`. FIPS 186-4 section 4.7's steps in its order and with its names, over
the big integers `pkcs1` already had. ADR 0229's argument carries over unchanged and is the reason
this is not a dependency question at all: **there is no secret.** `p`, `q`, `g`, `y`, `r` and `s`
all came out of a file a stranger wrote, so the side-channel class of defect ADR 0031 takes reviewed
implementations for has nothing to act on, and nothing here runs in constant time.

What is *not* carried over is the direction in which a mistake is safe. RSA verification compares two
whole encoded blocks, so a wrong padding or digest produces "does not verify" and never a false
"verifies". DSA compares two numbers modulo `q`, and what keeps a forgery out is FIPS 186-4 step 1 —
`0 < r' < q` and `0 < s' < q`. That is the first thing `verify` does, and the test that matters feeds
it `r = 0`, `s = 0`, `r = q` and `s = q` one at a time, re-encoding the fixture's own
`Dss-Sig-Value` with one value replaced, and then verifies the real pair through the same builder so
that the four failures are their own rather than the builder's.

Three details are worth recording because each was a choice:

- **The inverse is Fermat's rather than the extended Euclidean algorithm.** FIPS 186-4 Appendix C.1
  states the latter and admits "an algorithm that produces an equivalent result"; `s^(q-2) mod q` is
  one, and it is the one that needs no division — `bigint` has none anywhere. The cost is a
  primality assumption on `q`, and it is safe in the closed direction: for a `q` that is not prime
  the result is not the inverse, `v` is not `r'`, and the signature does not verify.
- **`z` is a shift, not a byte slice.** Step 2's `z` is "the leftmost min(N, outlen) bits" and
  Appendix C.2.1 makes the first bit the most significant. Every `(L, N)` pair section 4.2 approves
  makes `N` a multiple of eight, so a byte slice would pass every real test; it is written by bits
  because `q` comes out of the file and its width is not this program's to assume. **The fixture
  exercises the truncation for real**: `openssl dsaparam 2048` produced `(L, N) = (2048, 224)` and
  the digest is SHA-256, so `z` is 224 of 256 bits and a verifier that skipped the rule fails.
- **Two budgets, both this program's and both reported.** `MAX_MODULUS_BITS` is twice Table 260's
  4096, on `pkcs1`'s reasoning. `MAX_SUBGROUP_BITS` is 512, twice the largest `N` FIPS 186-4 section
  4.2 approves, and it is the sharper of the two: `q` bounds the exponents of two modular
  exponentiations, so an unbounded `q` in a hostile certificate is unbounded work.

**FIPS 186-5 withdrew DSA for signing and kept it for exactly this.** Its section 4: "This standard
no longer approves the DSA for digital signature generation. However, the DSA may be used to verify
signatures generated prior to the implementation date of this standard". A viewer is a verifier, and
the documents already signed do not stop existing — which is the honest answer to "why implement a
dead algorithm" and is better than the population's silence.

## Decision 2 — refuse ECDSA and EdDSA now, and say what would change it

**No dependency is added and no curve is written.** `Cargo.toml` is untouched and `cargo deny` is
unchanged. The refusal rests on four things, in decreasing order of weight:

1. **The domain parameters are in no document this tree holds.** Six ECDSA curves and two Edwards
   curves, each a prime, two coefficients, a base point and an order. Transcribing them from memory
   or from an unattributable source is what principle 5 forbids, and a self-consistency check —
   `G` on the curve, `nG = O` — proves a constant was not mistyped and not that it is the curve the
   world means by P-256.
2. **The available packages cover a quarter of it.** `p256` and `p384` are two of TS 32002's six
   ECDSA curves and none of its two Edwards ones; `p521` is a third; there is no maintained
   Brainpool pair on the line this tree's `digest` 0.11 sits on, and Ed448's SHAKE256 is a hash
   family this tree does not compute at all. A dependency taken today would move the silence rather
   than close it — ADR 0229's own phrase, made concrete.
3. **The demand is one signature in 811**, and two of the three ECDSA witnesses use BSI TR-03111's
   plain encoding, which no candidate package parses.
4. **Half a curve is worse than none.** `CLAUDE.md` principle 1: if it cannot be done properly now,
   it is not started now. An ECDSA verifier missing its public-key validation or its `r`/`s` range
   checks accepts forgeries, and the failure is silent in the dangerous direction — unlike RSA's.

**What would change it**, so that a later round does not have to re-derive this: a corpus that makes
ECDSA more than a rounding error, or a decision to accept the whole `elliptic-curve` stack with its
package count and licence position argued in `doc/stack.md`. If that day comes, take the packages —
the arithmetic genuinely is where ADR 0031's argument has an instance — and take them for the curves
TS 32002 Table 3 lists rather than for the ones a crate happens to publish. Every ECDSA and EdDSA
signature and key reaches a person today as the identifier the file wrote, by
`Authenticity::AlgorithmNotVerifiable` and `Authenticity::KeyNotVerifiable`.

## Decision 3 — the arithmetic moved out, because a second caller arrived

`crates/pdf-model/src/bigint.rs` is `pkcs1`'s fixed-size integers, Montgomery modulus and
`modpow`, moved unchanged, plus three operations DSA needs and RSA does not: `reduce` (a value of
any size taken modulo `n`, shift-and-subtract, which is the one place a division would ordinarily
be), `multiply_reduced`, and `invert`. `pkcs1` is now RFC 8017 and nothing else; `dsa` is FIPS
186-4 and nothing else; neither reaches into the other. The alternative — making `pkcs1`'s
internals `pub(crate)` and having `dsa` use them — would have made "DSA borrows RSA's integers" a
sentence in the crate map, and it is not true of the mathematics.

## Decision 4 — the words keep every asymmetry, and gain one

`Authenticity::Verified` and `NotUnderThatKey` carry a `Family`, so the sentence names which of
Table 260's families did the verifying — "the 2048-bit DSA key in a certificate the file itself
carries" — where it used to say RSA unconditionally. Nothing else about the wording moved:
`Verified` is still not `Valid`, the certificate still arrived in the same file as the signature it
verifies, and the once-per-document sentence still says that of three questions this program
answers two.

One variant is new and it is the one two families make possible: `KeyDoesNotMatchAlgorithm`, where a
`SignerInfo` states DSA over a certificate holding an RSA key or the reverse. Both identifiers are
carried, because which of the producer's two contradicting claims is the wrong one is not something
a reader here can know, and picking one would be this program inventing a fact.

## Consequences

- **`Signature::authenticity` verifies two of Table 260's three families.** All ten of the
  corpus's signatures still verify, unchanged, and the new fixture's DSA one verifies through the
  whole path a document takes — algorithm recognition, certificate lookup by issuer and serial
  number, `Dss-Parms` key, arithmetic — and stops verifying when one bit of the message moves.
- **`Authenticity::UnknownDigest` carries the identifier**, so ISO/TS 32001's four digests are a
  named gap rather than an anonymous one. `Integrity::UnknownDigest` deliberately stays a unit
  variant: it is `Copy` and every caller holds it by value, and the number is delivered by the
  sentence beside it.
- **Two claims about the standard were corrected in the tree**, both found by reading rather than by
  a gate: `cms::Digest`'s "Table 260 and Table 256 … and nothing else", and §12.8.3.2's ledger row
  calling DSA a gap where Table 260 says "No".
- **The fixture is hand-built and says why.** `openssl dsaparam 2048` / `gendsa` / `req -x509 -sha256`
  once, pasted in as `dsa::fixtures`. Trap 8 is the whole justification: 67 460 documents contain no
  DSA signature, so a corpus can rank this requirement not at all and a hand-built pair is the only
  witness there is — the same footing `adbe.x509.rsa_sha1`, `PAdES` and the document timestamp are
  already on.
- **The `x509` fuzz target covers the new decoder and the new arithmetic**, with the DSA
  certificate added to its seed corpus, and asserts the same properties it asserts for RSA: nothing
  outgrows its input, the budget is reported by name, and no signature ever verifies against a
  digest the target chose and nobody signed.
- **Nothing on the launch path changed.** `signature::signatures` reads dictionaries and does no
  arithmetic; the verification lives in `notes::about`, on the document's own thread beside the
  window (ADR 0182), and it runs only for a document that carries a signature at all.
- **No gate that draws anything moves**, for ADR 0215's and ADR 0229's reason: no gate in this tree
  looks at a signature, and nothing outside §12.8 was touched.

## The lesson

**A refusal whose reason is a *count* should have the count taken before it is believed, and the
count should be of everything the refusal touches.** `doc/todo/51` had ECDSA at "five curves" and
DSA at "a modular exponentiation", and both figures came from reading one table in one document.
The Technical Specifications that amend that table were sitting in `doc/md/` — the same directory
`CLAUDE.md` already tells a round to read the *titles* of before recording a silence — and they
turned five curves into eight, added a second group law, added four digests, and settled a
`/SubFilter` question in the other direction. The reading changed the answer; the corpus only
confirmed that neither had demand and, incidentally, named a third thing that does.
