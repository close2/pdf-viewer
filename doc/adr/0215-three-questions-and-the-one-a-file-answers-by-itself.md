# ADR 0215 — Three questions, and the one a file answers by itself

Status: accepted, 2026-08-07 (session 377).

## Context

`doc/todo/51` said signature validation is "blocked on infrastructure this program does not have"
and named it: "a trust store and a network". ADR 0088 had said the same thing more carefully — it
read every signature and computed no digest, deliberately — and seventeen ledger rows in §12.8.3
had sat `reported` behind that one sentence since.

**The sentence is true of one question and false of another, and §12.8.1 separates them itself.**
The clause states verification in one paragraph:

> To verify the signature, an appropriate signature handler is required. That signature handler
> shall match with the type of the signature that has been created. The signer's certificate shall
> be determined and verified by the signature handler to match with any of the validation
> parameters and other conditions. If the verification fails, the signature shall be considered
> invalid. The digest shall be recomputed and compared with the one stored in the document.
> Differences between the two indicates that modifications have been made since the document was
> signed and thus the signature shall be considered invalid.

Read that as three questions rather than one refusal:

| | asks | needs |
|---|---|---|
| **1. Integrity** | has the document changed since it was signed? | the file, a hash function, and the digest the signature records |
| **2. Authenticity** | does the signature verify under the signer's public key? | an X.509 certificate and an RSA or ECDSA verification |
| **3. Trust** | is the signer anyone to believe, and was the certificate revoked? | a trust store, a certification path, and a network |

Only the third needs what the todo named. The first needs `sha2`, which was already a workspace
dependency, and the arithmetic of §12.8.1's `/ByteRange` — which this tree was already reading, and
already reporting the *shape* of: whether the range covered the whole file, and how many bytes came
after it. It had the question and not the answer.

## Decision

**Answer question 1. Name questions 2 and 3 as unanswered, in the program's own words, on every
document that carries a signature. Never say "valid".**

### What answers question 1, and where the digest is

The digest is not hidden. For every signature format §12.8.3 defines except one, the value the
signature commits to is written into `/Contents` where any reader can find it:

- **`adbe.pkcs7.detached` and `ETSI.CAdES.detached`** — RFC 5652's `message-digest` signed
  attribute, which §12.8.3.4.3 (e) requires of a PAdES signature and CMS defines for all of them.
- **`adbe.pkcs7.sha1`** — §12.8.3.3.1 states it outright: "[t]he SHA-1 digest of the document's byte
  range shall be encapsulated in the CMS `SignedData` field with `ContentInfo` of type Data."
- **`ETSI.RFC3161`**, a document timestamp — Table 255: "[t]he value of the `messageImprint` field
  within the `TimeStampToken` shall be a hash of the bytes of the document indicated by the
  `ByteRange`".
- **`adbe.x509.rsa_sha1`** is the exception, and it is the interesting one. §12.8.3.2's PKCS #1
  value *is* the RSA signature, with the digest inside it, so there is nothing recorded in the open
  and question 1 cannot be answered without question 2's public key. `Integrity::UnderTheSignersKey`
  says that, rather than "not a CMS object", which would be true and useless.

`Signature::integrity` dispatches on `/SubFilter` and the CMS object's own shape, recomputes the
digest over the byte range with the algorithm the signature named, and compares.

**A fifth case arrived from the corpus rather than from the clause**: a CMS `SignerInfo` with no
signed attributes at all. RFC 5652 then signs the encapsulated content directly, so nothing records
the document's digest in the open — the same answer as PKCS #1, reached a different way.
`bug854315.pdf` is that document.

### The six digests, and a dependency decision

Table 260 lists "SHA1 ( PDF 1.3 ) SHA256 (PDF 1.6) SHA384 (PDF 1.7) SHA512 (PDF 1.7) RIPEMD160
(PDF 1.7 )" and Table 256's `/DigestMethod` adds the MD5 that was PDF 1.5's default. `sha2` and
`md-5` were already here for §7.6's algorithms and cover four. **`sha1` 0.11.0 and `ripemd` 0.2.0
are added**, from the same RustCrypto family, under the same `MIT OR Apache-2.0`, bringing exactly
two new packages between them and no transitive dependency this tree did not already build.

The argument is ADR 0014's and the aes/rc4 comment's, one domain over: a widely reviewed
implementation of a published algorithm beats a fresh one, and cryptography is where that argument
is strongest. The alternative was implementing SHA-1 and RIPEMD-160 in-tree, or implementing five
of the standard's six algorithms and being *silent* about the sixth — which is the failure this
project spends its rounds removing. `cargo deny` is clean on all four checks.

### Reading the blob: a DER reader in-tree, and a tolerance with a cost

§12.8.3.3.1 says the signature value is "a DER-encoded CMS binary data object", so answering
question 1 means an ASN.1 reader over **untrusted input**. `pdf_model::der` is that reader, in-tree,
about two hundred lines, and the argument is the one ADR 0186 made the other way for XML:

- everything §12.8 needs is a tag, a length and a sub-slice — no arithmetic on values, no decoding
  into owned types, **no allocation at all**;
- `#![forbid(unsafe_code)]` covers it, which is what `CLAUDE.md` principle 3 asks of anything a
  PDF's bytes reach;
- the bounds are ours to state and to check: `MAX_DEPTH` 16, `MAX_VALUE` 2 MiB, a length in at most
  four octets, and 64 attributes per signer.

Where ADR 0186 took a dependency, it was for a *format* — XML, with namespaces, entities and
character references — that no small reader gets right. This is a length-prefixed tuple grammar.

**One tolerance, and it is a decision rather than an oversight.** X.690 clause 8.1.3.6 lets a
constructed value state an indefinite length and close itself with an end-of-contents marker;
clause 10.1 forbids that in DER, and §12.8.3.3.1 says DER. **Four of the corpus's ten signature
values are written that way** — Adobe's handler emits `30 80` for the outer `ContentInfo` — so a
reader that took the clause's word for it would answer "unreadable" for the commonest real
signature there is. The reader accepts them. The cost, written down here so it is not discovered
later: **this reader cannot be used to check that a producer wrote DER**, and §12.8.3.4.2's row says
so rather than claiming the constraint is checked.

A neighbouring trap came free with it. §12.8.3.3.1 has a producer pad `/Contents` with zeros to fill
the space allocated for it, and a pair of zero octets is X.690's end-of-contents marker — so
stripping the padding before parsing cuts the marker off an indefinite-length encoding. The first
draft of this module did exactly that and could read none of those four files.
`trailing_padding_is_not_read_as_a_value` is the test that keeps it.

**Fuzzed at 1 000 000 runs, clean, with no artefact left behind**, seeded with all eleven signature
values the corpus's nine signed documents hold. `fuzz/fuzz_targets/cms.rs` checks four properties:
that parsing terminates and never panics, that the bounds hold and are observable, that nothing
handed back is larger than the input, and that a successful parse is idempotent.

### What the program says, and what it refuses to say

Per signature, when the document opens:

> the bytes that signature covers no longer hash to the SHA256 digest it records — this document
> was modified after it was signed (§12.8.1)

or

> the bytes that signature covers still hash to the SHA256 digest it records, so nothing changed
> after signing — who signed is a separate question and is not answered

and, once per document:

> of the three questions a signature asks, this program answers one: whether the document changed
> since it was signed (§12.8.1's digest, recomputed above). It does not check the signature against
> the signer's public key, and it does not know whether the signer is trusted or had been revoked —
> it has no certificate store and makes no network request. So nothing here says a signature is
> valid

**The asymmetry in the first two sentences is the whole point.** The recorded digest sits *beside*
the signature rather than inside it, so whoever changed the document could have changed the digest
to match; what they could not do is make the signature over it verify, and that is question 2. So a
**mismatch is decisive** — §12.8.1 says a difference "indicates that modifications have been made"
— and a **match is the absence of one kind of evidence against the file**, nothing more. A viewer
that said "signature valid" having checked only a digest would have been the worst outcome of this
round; a viewer silent about a document whose signed bytes no longer hash was nearly as bad, and
that is what this tree was.

### And three structural rules that need no cryptography at all

§12.8.3.4's PAdES requirements divide the same way. Its *validation* steps (§12.8.3.4.5) are
certificates and a network throughout. Its *structural* rules are arithmetic over what the file
says, and `Signature::pades_departures` answers six of them: §12.8.3.4.2's three — the range must
cover the whole file, the dictionary must state no `/Cert`, and the signing time may be in `/M` or
in the signed attributes but not both — and §12.8.3.4.3's (a), (d) and (e).

**Not (i) in full, and the reason is principle 5's.** The clause forbids four attributes and RFC
5652 states the object identifier of one of them; the other three are defined in documents this tree
does not hold. Naming an identifier nobody here has checked against its source would be asserting a
fact about a specification. Half of (i) is implemented and the row says which half.

The whole subclause is scoped to `ETSI.CAdES.detached` by §12.8.3.4.1's own sentence, so a signature
with another `/SubFilter` is not held to it — which one of the tests asserts beside the one that
finds the departures, because a rule applied where the clause does not address it is trap 11.

## What the corpus said, and one defect it found

**Nine documents carry ten signature dictionaries**, which is more than the six this tree could see
before: §12.8.1 puts a usage rights signature's dictionary in the permissions dictionary "(not from
a signature field)", so `signatures` cannot reach one and three documents carry nothing else.
`Permissions` now carries both signature dictionaries it names.

| | |
|---|---|
| signed bytes still hash to what they record | **5** — SHA-1 ×2, SHA-256 ×3 |
| signed bytes **no longer hash** | **4** |
| no digest recorded in the open | 1 |
| signature value unreadable | 0 |

The four that changed:

- **`xfa_filled_imm1344e.pdf`, both of its signatures** — the corpus's one certification signature
  and a usage rights signature beside it. This is the file ADR 0088 quoted as §12.8.2.2 demonstrated,
  with 2 542 822 bytes appended after the signed range. The appended bytes are not the problem: the
  *signed* range no longer hashes, and the file says why itself — the gap its `/ByteRange` names is
  not where its `/Contents` sits any more. The gap's **size** still matches the signature value's to
  the byte, and its position is 4 213 bytes out, so the file was re-saved rather than incrementally
  updated. Whatever this document is, it is not the one that was signed.
- **`issue6127.pdf`** — the same shape, 1 378 bytes out.
- **`poppler-395-0-fuzzed.pdf`** — a fuzzed file, which is expected to fail everything.

**And the demand track named a defect, in `pdf-syntax` rather than in §12.8.** `issue17069.pdf` came
back `NoSignatureValue`: a 33 680-byte `/Contents` read as zero bytes. The document is encrypted,
and §7.6.2's fourth exception keeps a signature's `/Contents` out of the cipher — but
`is_signature_dictionary` recognised one by `/Type` alone, and its doc comment said in as many words
that "`/Type` is the only thing that identifies one". **Table 255 makes `/Type` "(Optional if Sig)"
with a default of `Sig`.** That document states none, so the exception did not apply, the signature
value went through AES and came back empty. It is now recognised by the `/ByteRange` and `/Contents`
pair Table 255 requires of every signature carrying a byte range digest — nothing else in ISO 32000-2
has a `/ByteRange` — and its digest matches.

Two claims died with it, both of which had been written down as measurements. The handover's trap 8
said "eight documents carry a signature dictionary, twenty-six an `/Encrypt`, and the two sets are
disjoint, which is one `grep`"; nine of the §12.8.3 ledger rows said the same. The grep was right
about the *sets* and the conclusion drawn from it — that signatures never meet encryption here — was
false, because the reader that had to handle the overlap could not see it.

## Consequences

- **Nine of §12.8's `reported` rows become `partial`**: §12.8.2.2.2, §12.8.3, §12.8.3.1, §12.8.3.3,
  §12.8.3.3.1, §12.8.3.4, §12.8.3.4.1, §12.8.3.4.2 and §12.8.3.4.3. Ledger-wide, `partial` 238 →
  **247** and `reported` 30 → **21**, of 875 rows.
- **Seven stay `reported` and each says which question it is**: §12.8.3.2 (the digest is under the
  key), §12.8.3.3.2 and §12.8.3.4.4 to §12.8.3.4.8 (trust and revocation). §12.8.3.3.2 gained the one
  thing it could have: the clause prints `adbe-revocationInfoArchival`'s object identifier itself, so
  the presence of revocation material is now named to a person even though nothing checks it.
  `issue17069.pdf` is the witness.
- **The launch path pays for it only where there is a signature to pay for.** `open_cost` puts
  §12.8's walk at **0.024 ms** on ISO 32000-2, which has no signature field and where ADR 0181's
  `/SigFlags` reading stops it at once, and at **0.192 ms** on `xfa_filled_imm1344e.pdf`, which has
  two signatures in 3 MB. The digests themselves are in `notes::about`, on the document's own thread
  beside the window (ADR 0182): three traced launches of that document put **document joined** at
  2.657, 2.764 and 3.936 ms of the whole 73 to 77 ms, hashing 1 MB across two algorithms on the way.
  Nothing on the critical path grew.
- **No gate that draws anything moves**, and one that does not draw might have: the corpus, the
  oracle, the text and quorra gates never look at a signature. The encryption fix changes what one
  document's signature dictionary holds and not what any page draws.
- **What is still not answered is questions 2 and 3**, and `doc/todo/51` now says so in those terms:
  an RSA and ECDSA verification with an X.509 certificate parser is a decision somebody can take on
  its own merits, and a trust store with a revocation story is the project ADR 0088 called it.
