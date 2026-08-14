# ADR 0229 — A key that arrived in the same envelope

Status: accepted, 2026-08-08 (session 392). Answers the second of the three questions ADR 0215
separated. **Decision 1's "write the RSA half in tree" is superseded in part by ADR 0331 (session
496, an owner decision): the construction, budgets and refusals stay in tree, the modular
arithmetic under them is `crypto-bigint`'s.** Decisions 2 to 4, and Decision 1's case against the
whole-scheme `rsa`/`x509-cert` packages, stand — ADR 0331 re-measured that table and the stable
`rsa` line is still on the second hash stack.

## Context

ADR 0215 divided §12.8.1's one sentence about verification into three questions and answered the
first — has the document changed since it was signed — with a digest over `/ByteRange` compared
against what §12.8.3.3's CMS object records. It named the second as open and said what it would
take: an X.509 certificate parser over the `certificates` the CMS object already carries, an RSA
verification, and RFC 5652 section 5.4's re-encoding of the signed attributes. `doc/todo/51` carried
that as **a dependency decision nobody had been asked for**, in ADR 0186's shape.

**The todo file's list of what question 2 needs was one algorithm short, and Table 260 says so.**
It named "an RSA PKCS #1 v1.5 verification and an ECDSA one over the curves RFC 5480 names". The
table has three rows, not two:

| | |
|---|---|
| RSA Algorithm Support | Up to 1024-bit (PDF 1.3) Up to 2048-bit (PDF 1.5) Up to 4096-bit (PDF 1.5) |
| DSA Algorithm Support | Up to 4096-bits (PDF 1.6) |
| ECDSA Algorithm Support ( defined by Internet RFC 5480 ) | ANSI X9.62, Elliptic Curve Digital Signature Algorithm (ECDSA) (PDF 2.0 ) |

That matters for the dependency argument rather than for the code: a decision that closes "the
signature half" has to know it is three families and not two.

## Decision 1 — take no dependency, and write the RSA half in tree

**`rsa`, `p256`/`p384` and `x509-cert` are all declined. Nothing was added to `Cargo.toml` and
`cargo deny` is clean on all four checks with no new package and no change to the licence
position.** The four questions ADR 0014 asks, answered with numbers.

**Which crates, and what they cost.** Measured by resolving each in a scratch crate and reading
`cargo tree -e normal`:

| candidate | new packages | note |
|---|---|---|
| `rsa` 0.9.10 | **31** | brings a *second* hash stack — `digest` 0.10, `crypto-common` 0.1, `generic-array` 0.14 — beside this tree's `digest` 0.11 / `crypto-common` 0.2 / `hybrid-array` 0.4, so two copies of SHA-256 in one binary and `cms::Digest` unusable with its API |
| `rsa` 0.10.0-rc.18 | fewer | on the matching line, and **a release candidate**; this tree has no pre-release dependency and a signature verifier is a poor place for the first |
| `p256` + `p384` 0.14 | **28** | matching line, all MIT/Apache-2.0 — and covers **two** of the five curves RFC 5480 names, so it moves the silence rather than closing it |
| `x509-cert` + `der` 0.8 | — | see below; it would *lose* four corpus signatures |

**The argument ADR 0031 made for taking the ciphers has no instance here, and that is the decisive
point.** That ADR's reasoning — a widely reviewed implementation of a published algorithm beats a
fresh one, most strongly in cryptography — is about defects that leak a *secret*: timing and cache
side channels on key material, fault attacks on a private key. **An RSA signature verification has
no secret in it at all.** It is `s^e mod n` over three numbers a stranger wrote into the file, none
of them ours and none of them private. Nothing in `pdf_model::pkcs1` is written to run in constant
time and nothing needs to be. What is left is integer arithmetic against published algorithms, which
is testable — and is tested, against a real 2048-bit key, against small cases anyone can check, and
against every signature in the corpus.

**The one class of defect that does matter is a verifier that accepts a forgery, and the
construction rules it out rather than the review.** RFC 8017 section 8.2.2 step 4 compares two whole
encoded messages: the block recovered from `s^e mod n` and the block built from the digest the
verifier computed itself. A verifier that instead *parses* the recovered block — skipping padding to
a zero octet and reading whatever `DigestInfo` follows — accepts trailing bytes nobody signed, which
is Bleichenbacher's forgery against small exponents. Encode-and-compare has no room for one, and it
has a second property worth stating: **a mistake in this module is safe in one direction.** The
wrong padding, the wrong digest or the wrong algorithm produces "does not verify", never a false
"verifies".

**`x509-cert` would refuse four of the corpus's ten signatures, and this is measured rather than
argued.** ADR 0215 accepts X.690 clause 8.1.3.6's indefinite lengths, which DER forbids and Adobe's
handler emits: four of the ten signature values begin `30 80`. A strict DER reader will not have
them — `openssl pkcs7 -inform DER` fails on exactly those four with `not enough data`, and
`x509-cert` sits on `der` 0.8, which is a DER decoder. Replacing `pdf_model::der` with a general
ASN.1 compiler would cost this tree the commonest real signature there is.

**What would change the answer, because a decision that says no owes this.** Two things, and
neither is RSA:

- **A document with an ECDSA or DSA signature.** An elliptic curve group law is not a modular
  exponentiation: five curves' field arithmetic, point addition, doubling and inversion, written
  fresh, is where ADR 0031's argument *does* have an instance. The day one arrives here, take
  `p256`/`p384` (and whichever of `p224`/`p521` the file needs) and argue the remaining curves
  separately. The corpus has none today — every one of its ten signatures is RSA, checked by reading
  the algorithm identifier out of all eleven extracted signature values.
- **A key beyond `pkcs1`'s budgets appearing in the wild.** The budgets are this program's and are
  reported by name; a real file that trips one is evidence the budget is wrong, not that the file is.

## Decision 2 — what the certificate reader reads, and what it refuses to

`pdf_model::x509` is **213 lines of code**, over `pdf_model::der`, allocating nothing a length in
the file sizes. It reads the serial number, the issuer, the subject, the `subjectKeyIdentifier`
extension and `subjectPublicKeyInfo`. That is what verifying needs and no more.

**Every field that is a trust decision is deliberately not read**: validity dates, basic
constraints, key usage, the issuer's signature over the certificate, the chain above it. Reading a
`notAfter` and saying nothing about who issued the certificate would be the worst of both — an air
of validation over a certificate that could have been written five minutes ago by whoever wrote the
file.

**A name is compared and never decoded**, which is both smaller and safer. RFC 5652 identifies the
signer's certificate by issuer and serial number; both sides are compared as the encodings the file
wrote. So nothing here can print who a certificate says it belongs to — and that is a feature: a
subject name out of an unverified certificate is a claim by whoever wrote the file, and showing one
beside the word "verifies" is how a viewer says *valid* without using the word.

**An algorithm this program cannot verify is named by its number.** `x509::dotted` decodes X.690
clause 8.19's encoding to dotted decimal, so an ECDSA certificate is reported as
`1.2.840.10045.2.1` rather than as a word. `CLAUDE.md` principle 5 is why: this tree holds ISO
32000-2 and not the documents that assign those identifiers, and printing the file's own digits is
the only claim a reader can check. `pdf_model::der` decodes no identifier — every use there is a
comparison against a constant — and that stayed true until something had to *report* one it did not
recognise.

**And every RFC sentence quoted in this round's code was checked against the published RFC**, not
paraphrased from memory: RFC 8017 sections 3.1, 5.2.2, 8.2.2, 9.2 and Appendix A.2.4, and RFC 5652
section 5.4. They appear as inline quotations rather than rustdoc blockquotes, because the
conformance checker's blockquote form is reserved for ISO 32000-2's own words and an unverifiable
quotation in that form would be checked by nothing. The first draft of `pkcs1.rs` attributed the
NULL-parameters rule to section 9.2's Note 1; it is Appendix A.2.4's, and reading the RFC is what
found that.

## Decision 3 — the arithmetic, and the two budgets

`pdf_model::pkcs1` is **356 lines of code**. A fixed-size big unsigned integer of at most 128
64-bit limbs, Montgomery multiplication in the coarsely-integrated operand scanning form, and
square-and-multiply. **There is no division anywhere in the module**: entering the Montgomery domain
is `64 * limbs` doublings, and every reduction is one pass of the multiplication itself. Every loop's
trip count is a constant of the module rather than a number out of the file.

Two budgets, both stated as this program's rather than as anything the standard says, and both
reported by name:

- **`MAX_MODULUS_BITS` 8192**, twice Table 260's ceiling of 4096. A key beyond the standard is a
  file to report rather than one to refuse silently; a key beyond *this* is `ModulusTooLarge`.
- **`MAX_EXPONENT_BITS` 256.** `modpow` costs one modular squaring per exponent bit, so an unbounded
  exponent is unbounded work over a number a stranger chose. The two public exponents in practical
  use are 3 and 65537 — seventeen bits between them — and this leaves the worst case at 512 modular
  multiplications.

An even modulus is refused (`ModulusNotOdd`): RFC 8017 section 3.1 makes an RSA modulus "a product
of u distinct odd primes", and Montgomery reduction has no inverse modulo `2^64` for an even one. The
first draft of the small-numbers test used a modulus of 1000 and that refusal is what found it.

## Decision 4 — the words, which are as much of this round as the cryptography

Answering question 2 changes what a matching digest *means*. It stops being the absence of one kind
of evidence against a file and becomes evidence about a key. It still does not say who the signer is.
So, per signature:

> and that signature verifies under the 2048-bit RSA key in a certificate the file itself carries,
> over the attributes that record the digest above (RFC 5652 section 5.4) — so that digest is the
> signer's

or, where question 1 said the bytes moved:

> and that signature does verify under the 2048-bit RSA key in a certificate the file itself carries
> — but what it signs is the digest above, which these bytes no longer produce. The signature is a
> real one and the document under it is not the document it was made over

or, for a signature over the byte range itself:

> and that signature verifies under the 2048-bit RSA key in a certificate the file itself carries,
> directly over the bytes its /ByteRange names — so those bytes are the ones that were signed and
> nothing has changed since

and, once per document:

> of the three questions a signature asks, this program answers two: whether the document changed
> since it was signed (§12.8.1's digest, recomputed above) and whether the signature verifies under
> the public key in the certificate the file itself carries (§12.8.3.3.1). It does not answer the
> third — it has no certificate store and makes no network request, so it does not know whether that
> certificate belongs to anyone you have reason to believe, nor whether it had been revoked. A
> signature that verifies here was made by whoever holds the key in a certificate that arrived with
> the document, which is not the same as a valid signature. Nothing here says valid

**"A certificate that arrived with the document" is the load-bearing phrase.** §12.8.3.3.1 requires
the signer's certificate to be *inside* the signature value, so the verification is a
self-consistency check between two things the same stranger wrote. It is a real fact — it is exactly
what a forger who edits the document cannot produce — and it is not a statement about a person. A
viewer that said "signature valid" having checked a key it has no reason to trust would have been
the worst outcome of this round, and the sentence above is what stops it.

**The second sentence is the one that could not be said before.** Four of the corpus's ten
signatures answer `Changed` to question 1 and `Verified` to question 2; either answer alone
misleads, and only the pair says what actually happened to the file.

## What the corpus said

`cargo test --profile gates -p pdf-model --test signatures`, over the nine documents carrying ten
signature dictionaries:

| | |
|---|---|
| verifies under the key in a certificate the signature itself carries | **10** |
| does not verify | **0** |
| question 2 not answered | **0** |
| key widths | 1024 × 3, 2048 × 6, 4096 × 1 |
| verifying binds the document's own bytes | **1** |
| stop verifying when one bit of the signature is turned over | **10** |
| processor time for all ten | **4.583 ms** |

**All ten verifying is exactly the shape a verifier stuck at `true` would produce, so the
measurement asks each of them again with one bit of its signature value turned over** — the bit is
found inside the `SignerInfo`'s own signature octets rather than in `/Contents` at large, because
§12.8.3.3.1's zero padding is a region a verifier is *right* to ignore. All ten stop verifying.

Two findings are worth their own paragraphs.

**`bug854315.pdf` had its first question answered by its second.** Its `SignerInfo` states no signed
attributes at all, so RFC 5652 signs the content itself — which for a detached signature is the byte
range — and ADR 0215's `Integrity::UnderTheSignersKey` was this program saying it could not answer
without a key it did not have. The key was in the file all along, where §12.8.3.3.1 requires it. It
verifies, over the document's own bytes, so **that document has not changed since it was signed** and
the note now says so instead of reporting an unanswerable question.

**The four `Changed` signatures are all authentic.** `issue6127.pdf`, `poppler-395-0-fuzzed.pdf` and
both of `xfa_filled_imm1344e.pdf`'s verify under their signers' keys, over signed attributes recording
a digest the files no longer produce. These are not broken signatures; they are real signatures whose
documents were re-saved underneath them. ADR 0215 had reached the same conclusion from the *shape* of
`xfa_filled_imm1344e.pdf` — the gap its `/ByteRange` names is 4213 bytes from where its `/Contents`
sits, while the gap's size still matches to the byte — and this is the same conclusion reached from
the cryptography.

**Two independent instruments, and both of them disagreed with each other before either agreed with
us.** `openssl cms -verify -noverify` over the extracted signature value and the reconstructed byte
range says "CMS Verification successful" for `bug854315.pdf`, and refuses two others for reasons that
are not the signature: `issue16553.pdf` because its outer `ContentInfo` states an indefinite length
(`not enough data`), and `signed_verified.pdf` with `no matching digest`, because its `SignedData`'s
`digestAlgorithms` set names SHA-1 while its one `SignerInfo` names SHA-256 — a producer defect in a
field this verification does not consult. So a twenty-line Python re-implementation over `pow()` was
written as a third opinion, and it verifies all three. `CLAUDE.md` principle 5's direction of
inference is why this is recorded rather than hidden: agreement raises confidence, disagreement is a
question about the file, and neither instrument defines correct.

## Consequences

- **Two §12.8 rows move and none is left dishonest.** §12.8.3.2 (PKCS #1 signatures) `reported` →
  `partial` — its `/Cert` certificate is now read and its signature verified, with DSA named as the
  gap — and §12.8.3.4.5 (validation of PAdES signatures) `reported` → `partial`, because half of its
  step (a) is done: "use the public key contained in the signer's certificate to verify that the
  document digest found in the signature is correctly signed". Ledger-wide `partial` 249 → **251**
  and `reported` 21 → **19**, of 875 rows. §12.8.1, §12.8.3, §12.8.3.1, §12.8.3.3, §12.8.3.3.1,
  §12.8.3.4, §12.8.3.4.3 and §12.8.2.2.2 keep their status and have their notes corrected.
- **Six claims expired and the sweep found them**, four in the ledger and two in the source:
  §12.8's own row said one of three questions is answered; §12.8.3.4.6 said "this program answers
  question 1 only"; §12.8.4 said "[p]arsing any of it is the trust decision §12.8 refuses" and
  §12.8.4.4 said "reading them is X.509" — both of which were two claims wearing one coat, since
  reading the bytes was never the hard half. In the source, `signature.rs`'s `SecurityStore` and
  `cms.rs`'s module documentation said the same thing.
- **§7.6.5's row is corrected downwards rather than upwards**, which is the honest direction. The
  round was asked to read the public-key handler family against what it built, and the answer is
  that a certificate parser is a smaller fraction of §7.6.5 than it sounds: that clause needs CMS
  `EnvelopedData` rather than `SignedData` — a different structure, with recipient information in it
  — an RSA *decryption* rather than a verification, and a way to reach the reader's own private key,
  which is where its threat model starts and where nothing has been decided. The row stays
  `reported` and now names `pdf_model::x509` in its `code` list so that a later round starts from
  what exists.
- **A new fuzz target, and one extended.** `fuzz/fuzz_targets/x509.rs` parses a certificate and runs
  a verification on whatever key comes out, checking that the budgets are reported, that nothing
  outgrows its input, that a parse is idempotent, and — the property that matters — that no signature
  ever verifies against a digest the target chose and nobody signed. Clean at **1 000 000 runs**,
  seeded with the 22 certificates the corpus's signatures carry plus the two test vectors — 24
  units, which two million runs grew to 374 — with no artefact left behind. (The corpus directory is
  `.gitignore`d, as every fuzz corpus in this tree is; `fuzz/seed_x509.py` rebuilds the seed.) `cms` was extended for the new
  fields and re-run clean at 1 000 000.
- **Nothing on the launch path grew.** `open_cost` puts §12.8's walk at **0.201 ms** on
  `xfa_filled_imm1344e.pdf`, against ADR 0215's 0.192 ms on the same file — the walk reads
  dictionaries and does no arithmetic. The verification itself lives in `notes::about`, on the
  document's own thread beside the window (ADR 0182), and costs **4.583 ms for all ten of the
  corpus's signatures between them**, hashing included.
- **No gate that draws anything moves**, and the reason is the same as ADR 0215's: no gate in this
  tree looks at a signature, and nothing outside §12.8 was touched.

## The lesson

**A refusal that names an *artefact* rather than a *decision* should be split before it is
believed.** Four rows in this ledger said some version of "that is X.509", and X.509 turned out to
be two things: a structure to parse, which is two hundred lines, and a trust decision, which is a
project. The first was doing the second's arguing for it, exactly as ADR 0214 found "the host is
better placed to map the role" doing the arguing for two different mappings. `doc/todo/01`'s sweep
for a note whose reason is a *capability* is the right instrument and it was pointed at "this
program has no ___"; the shape here is "that is ___", and it hid in four rows and two module comments,
the oldest of them written long before the round that could have retired it.
