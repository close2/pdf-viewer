# ADR 0322 — The padding inside the family called done

Status: accepted, 2026-08-14 (session 487). Completes Table 260's RSA family with its second
padding, RSASSA-PSS. Follows ADR 0215 (question 1), ADR 0229 (question 2, RSA PKCS #1 v1.5) and
ADR 0314 (question 2, DSA — and the census that ranked this item first).

## Context

ADR 0314's census read 67 460 documents and found the thing nobody was looking for: six real
signatures state `id-RSASSA-PSS` (`1.2.840.113549.1.1.10`) — twice ECDSA's share — and every one
was declined by number, `AlgorithmNotVerifiable 1.2.840.113549.1.1.10`, while this tree's own
documentation called the RSA family verified. Table 260's "RSA Algorithm Support" row states key
sizes and no padding at all, so the six are *inside* the row this tree had implemented; ETSI's
CAdES profiles permit PSS, and four of the six witnesses' dictionaries are `ETSI.CAdES.detached`.
`doc/todo/51` moved the item to the top of its list on that count and added a rule it had just
paid for: no further zero-witness algorithm family before a witnessed one.

Two constraints were in the file before this round started, and both held:

- **It is not PKCS #1 v1.5 with different constants.** `id-RSASSA-PSS` shares RFC 8017's `pkcs-1`
  arc with `rsaEncryption`, so a reader that matched the arc would verify the wrong construction;
  `cms::SignatureAlgorithm` had enumerated the v1.5 identifiers one by one since ADR 0229 for
  exactly this reason, and the todo file insisted the two verifiers stay separate whatever was
  built.
- **It needs no dependency and no external constant.** RFC 8017 section 8.1.2's
  `RSASSA-PSS-VERIFY` is section 9.1.2's `EMSA-PSS-VERIFY` over the same `RSAVP1` primitive
  `pkcs1` already runs, and everything the scheme is parameterised by — hash, mask generation
  function, salt length, trailer field — arrives in the file, inside the `signatureAlgorithm`
  `AlgorithmIdentifier`, as Appendix A.2.3's `RSASSA-PSS-params`. That is ADR 0314's own
  dividing line, on its other side: the elliptic-curve family was refused because its domain
  parameters are in no document this tree holds, and PSS's parameters are in the document being
  verified.

## Decision 1 — a separate module over the shared primitive

`crates/pdf-model/src/pss.rs`. RFC 8017 sections 8.1.2 and 9.1.2 in the RFC's order and with its
names — `emLen`, `maskedDB`, `H`, `dbMask`, `DB`, `salt`, `M'`, `H'` — plus Appendix B.2.1's MGF1.
No dependency was taken and `Cargo.toml` is untouched; ADR 0229's argument carries over unchanged
— **there is no secret**, every number came out of the file, nothing runs in constant time.

**What the two schemes share is exactly what the RFC shares between them.** Section 8.1.2 step
2.b and section 8.2.2 step 2.b both read "m = RSAVP1 ((n, e), s)", so that primitive — section
5.2.2's `s^e mod n`, with the length check both schemes state as their step 1 and `pkcs1`'s two
budgets in front of it — moved into `pkcs1::rsavp1`, and `pkcs1::verify` and `pss::verify` are
both callers of it. This is ADR 0314's Decision-3 shape one level up: sharing what is true of
the mathematics, and nothing else. Everything after the exponentiation is a different padding
and stays in its own module. Both paddings therefore also share `Pkcs1Error` for the refusals the
primitive makes, and `Authenticity::Refused` carries either's.

**The `I2OSP` edge is real and `openssl` cannot exercise it.** Section 8.1.2 step 2.c makes the
encoded message `emLen = \ceil ((modBits - 1)/8)` octets and notes that "emLen will be one less
than k if modBits - 1 is divisible by 8 and equal to k otherwise" — a case only a key whose bit
length is ≡ 1 (mod 8) reaches, and `openssl genpkey` rounds a 2049-bit request down to 2048. So
the test suite implements section 9.1.1's *encode* operation from the RFC's own steps and runs
the verifier against that construction at both `emBits = 2047` (every 2048-bit key; steps 6 and
9's spare-bit masking does real work) and `emBits = 2048` (the one-less-than-k case), beside the
`openssl` vectors. A vector pins the arithmetic against an independent implementation; the
RFC-derived construction pins it against the specification, which is the direction principle 5
wants the inference to run.

## Decision 2 — the parameters are read, and what cannot be acted on is refused by number

`pss::parameters` reads `RSASSA-PSS-params` over `pdf_model::der` — the four members are explicit
context tags with the appendix's defaults (SHA-1, MGF1 with SHA-1, salt 20, trailer 1). Four
refusals, each carried to a person through `Signature::authenticity` rather than skipped:

- **A hash this program does not compute** — SHA-224, the SHA-512/* truncations, anything newer —
  is `Authenticity::UnknownDigest` with the identifier, the same channel every other unknown
  digest already used.
- **A hash the scheme does not admit** — MD5 or RIPEMD-160, which `cms::Digest` has for Table
  256's sake and Appendix A.2.1's `OAEP-PSSDigestAlgorithms` set does not contain — is the new
  `Authenticity::PssParametersNotVerifiable`, worded so as not to claim the program cannot
  compute what it can. The accepted set is the intersection: SHA-1, SHA-256, SHA-384, SHA-512.
- **A mask generation function other than `id-mgf1`** is refused by its identifier; Appendix
  A.2.1 says `PKCS1MGFAlgorithms` "for this version SHALL consist of id-mgf1", so there is no
  second one to implement.
- **A trailer field other than 1**: Appendix A.2.3 — "[i]t SHALL be 1 for this version of the
  document, which represents the trailer field with hexadecimal value 0xbc."

**Absent parameters are refused rather than defaulted, and that is a decision.** Appendix A.2.3
says the parameters field "SHALL have a value of type RSASSA-PSS-params". A `SignerInfo` stating
`id-RSASSA-PSS` with no parameters has not said which hash, mask or salt length it means, and
defaulting on its behalf would be this program guessing three algorithms at once — in the one
place where a wrong guess reads as "does not verify" against a signature that is real. All six
witnesses state their parameters in full; if a producer that omits them ever appears, the report
names exactly what is missing and the decision can be revisited on a witness rather than on a
guess.

**The salt length is deliberately unbounded.** It is used only in saturating comparisons against
the encoded message's own width — RFC 8017 section 9.1.2 step 3 makes an over-long salt
"inconsistent", a verification failure rather than a malformed file — so a hostile value costs
nothing before it fails, and no budget was invented for it.

**One asymmetry against the v1.5 path is worth writing down**: the digest that matters is the
parameters', not the `SignerInfo`'s `digestAlgorithm`. RFC 5652's `digestAlgorithm` describes the
`message-digest` attribute — question 1's comparison, which is unchanged — while section 9.1.2
step 2's `mHash` is computed with the `RSASSA-PSS-params` hash. The two are the same in every
witness (and Appendix A.2.3 recommends they be), but the code takes each from where its own
specification says it lives, so a producer that splits them verifies rather than surprises.

## What the census said

`signature_algorithm_census` over the same 67 460 documents as ADR 0314 (811 signature
dictionaries in 681 documents), before and after, the only lines that moved:

```
before:  6  AlgorithmNotVerifiable 1.2.840.113549.1.1.10
after:   6  Verified (2048-bit RSA (RSASSA-PSS))
```

All six real PSS signatures verify — SHA-256 throughout, under 2048-bit keys — and the list of
documents whose signature names an algorithm this program does not verify drops from four
documents to three, all elliptic-curve. Every other census line is byte-identical, which is the
regression statement for the v1.5 and DSA paths: 775 signatures answered exactly as before.
The witnesses are evidence about this reading, never the definition of correct — the definition
is the RFC-derived construction and the vectors above.

## Consequences

- **`Family` gains `RsaPss`** and the sentence a person reads names the padding: "verifies under
  the 2048-bit RSA (RSASSA-PSS) key in a certificate the file itself carries". Nothing else about
  ADR 0229's wording moved: `Verified` is still not `Valid`, and the once-per-document sentence
  still says that of three questions this program answers two.
- **`cms::SignedData` carries the `signatureAlgorithm`'s parameters value**, which no reader
  wanted before because no recognised algorithm was parameterised by one.
- **Five ledger rows change their claims and none its status**: §12.8's, §12.8.3's, §12.8.3.1's,
  §12.8.3.3's and §12.8.3.3.1's notes now state the RSA family verified under both paddings, with
  `pss.rs` in the last four rows' code lists. They stay `partial` for what is genuinely left —
  the elliptic-curve family, ISO/TS 32001's digests, and trust.
- **The `x509` fuzz target exercises the new path**: every RSA key it parses is now also asked to
  verify a PSS signature against a digest the target chose, at a stated salt length and at
  `usize::MAX`, and `Ok(true)` is a panic. The `cms` target covers the parameters capture through
  `signed_data` unchanged; both were re-run seeded and clean for this round.
- **Nothing on the launch path changed**: the verification runs in `notes::about`, on the
  document's own thread beside the window (ADR 0182), only for a document that carries a
  signature, and the six documents that newly verify spend one extra modular exponentiation each.
- **No gate that draws anything moves**, for the same reason as ADRs 0215, 0229 and 0314: no gate
  in this tree looks at a signature, and nothing outside §12.8's machinery was touched. The
  corpus and oracle raster gates cannot see this change.

## The lesson

**"Done" said of a family can hide a padding, exactly as "X.509" hid a parser inside a trust
decision (ADR 0229's lesson) and "five curves" hid eight (ADR 0314's).** The row this tree calls
"RSA Algorithm Support" was implemented in the only sense anyone had checked — every corpus
signature verified — and the instrument that showed otherwise was a census over a population
forty times larger, keyed on the identifier rather than on the verdict. A claim of coverage is
only as wide as the population it was measured on, and the demand track's job is to keep
re-measuring it.
