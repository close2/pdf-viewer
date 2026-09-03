# 0829 — The supplement, read; and a crypt filter name that defines nothing

Session 892. Status: **accepted**.

## Context

ADR 0820 implemented `/R` 5 and named one step as resting on a reading rather than on a sentence
about revision 5: whether a password is prepared with `SASLprep` (RFC 4013) before it is encoded
as UTF-8. This tree applies it; **pdf.js applies it only at revision 6**, and **Apache PDFBox
guards it with `dicRevision == REVISION_6`** on both the reading and the writing side. Two
independent implementations agreeing is, under `CLAUDE.md` principle 5, a question to take back to
the specification and never a target to move toward.

ADR 0820 also recorded what it could not obtain: **the Adobe Supplement to ISO 32000-1,
`BaseVersion` 1.7, `ExtensionLevel` 3** — the "deprecated proprietary Adobe extension" Table 21
points at, and the document that defines revision 5 — "is not in this tree and could not be read
… there is no network access from this account."

**That last clause was false, and it is the more useful half of this round.** DNS, TLS and HTTPS
all work from this shell. Round 887 reasoned from a constraint it had inferred rather than tested,
and the inference cost it the one document that settles its own open question. The habit that
comes out of it is in `doc/habits.md`.

## What was fetched, and from where

| document | fetched from | what it is |
|---|---|---|
| Adobe Supplement to the ISO 32000, `BaseVersion` 1.7, **`ExtensionLevel` 3**, Acrobat 9.0 SDK, June 2008 | the Internet Archive's copy of `adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/adobe_supplement_iso32000.pdf` (snapshot `20100923045157`), 1 373 256 bytes, `Creator: FrameMaker 7.2` | **the document that defines `/R` 5** |
| Adobe Supplement to ISO 32000-1, `BaseVersion` 1.7, `ExtensionLevel` 5, June 2009 | `adobe.com/content/dam/cc1/en/devnet/pdf/pdfs/adobe_supplement_iso32000_1.pdf` — still served | 7 pages on transparency, portable collections, rich text and XFA. **Not** the encryption one; the current Adobe URL does not carry ExtensionLevel 3 |
| Apache PDFBox `StandardSecurityHandler.java` (trunk) | `raw.githubusercontent.com/apache/pdfbox/trunk/…` | evidence about a reading, cited nowhere in the code as an authority |
| qpdf `qpdf/qtest/qpdf/c-r5-in.pdf` and `qpdf/qtest/encryption.test` | `raw.githubusercontent.com/qpdf/qpdf/main/…` | a real `/R` 5 document with **both** passwords published |

Neither supplement is vendored. `doc/md/` holds the ISO and PDF Association documents this project
is sponsored to keep; a copy of Adobe's is a licence question nobody has asked, and the two
sentences below are all this round needed from it.

## Decision 1 — revision 5 prepares its password with SASLprep, and the extension says so twice

**Keep the code. It was right, and it now rests on a quotation rather than on a reading.**

ISO 32000-2 alone could not settle it. §7.6.4.3.3's preamble says "Whenever UTF-8 password is used
below, steps (a) and (b) are to be applied to the relevant password string to generate the UTF-8
password", which binds the steps to the *algorithm*; but the clause's own title is "Algorithm 2.A:
Retrieving the file encryption key from an encrypted document in order to decrypt it (revision 6
and later)", and ISO 32000-2 states no key retrieval for revision 5 at all. Both readings are
available from the standard, which is exactly why the extension had to be read.

It states the preparation **twice**, and the second statement even names the two options:

> Algorithm 3.2a Computing an encryption key
>
> 1. The password string is generated from Unicode input by processing the input string with the
>    SASLprep (IETF RFC 4013) profile of stringprep (IETF RFC 3454), and then converting to a
>    UTF-8 representation.
> 2. Truncate the UTF-8 representation to 127 bytes if it is longer than 127 bytes.

and again, in the same subclause 3.5.2, under *Password Algorithms*:

> All passwords for revision 5 are based on Unicode. Preprocessing of a user-entered password
> consists first of normalizing its representation by applying the “SASLPrep” profile (see RFC
> 4013) of the “stringprep” algorithm (see RFC 3454) to the supplied password using the Normalize
> and BIDI options. Next, convert the password string to UTF-8 encoding, and then truncate to the
> first 127 bytes if the string is longer than 127 bytes.

**Those are the supplement's words, not ISO 32000-2's**, which is why they are here rather than in
a rustdoc blockquote: `tools/conformance`'s quotation gate resolves every blockquote against
`doc/md/`'s ISO text under the clause cited beside it, and would report these as misquotations of
§7.6.4.3.3.

**And the second one is worth putting beside its ISO counterpart, because they are the same
paragraph.** §7.6.4.1 reads: "All passwords for revision 6 shall be based on Unicode. Preprocessing
of a user-provided password consists first of normalizing its representation by applying the
"SASLPrep" profile ( Internet RFC 4013 ) of the "stringprep" algorithm ( Internet RFC 3454 ) to the
supplied password using the Normalize and BiDi options. Next, the password string shall be
converted to UTF-8 encoding, and then truncated to the first 127 bytes if the string is longer than
127 bytes". Word for word the supplement's, with *revision 5* become *revision 6*, *user-entered*
become *user-provided* and the modal verbs an ISO document requires. Two paragraphs that differ in
their revision number and nothing else do not describe two different preparations. The `quotations`
sweep may print this ADR for the near-match, and a person reading it will see the two side by side,
which is the sweep working.

So the code is unchanged and its doc comment now says why. pdf.js and PDFBox are both narrower
than the document they implement. What that costs them is small — `SASLprep` is the identity on
every ASCII password and on the empty one, which is what 32 of the corpus's 41 `/R` 5 documents
open with — but it is not nothing: a writer following the supplement stored the *prepared* form,
so a password carrying a character the profile normalises away authenticates here and does not
there.

**Every other step of Algorithm 3.2a agrees with what ADR 0820 derived**, which is worth recording
because it was derived without the document: the three sections of `/O` and `/U`, both validations
(the owner's salted with the whole 48-byte `/U`), both key unwraps under AES-256-CBC with a zero
initialisation vector and no padding, and the `/Perms` block with its `"adb"` marker. The
supplement's Algorithm 3.13 adds one sentence ISO 32000-2 §7.6.4.3.3 step (f) does not — "Byte 8
should match the boolean value of the EncryptMetadata key" — and it is a `should` over an entry
this reader takes from the block already.

## Decision 2 — the owner branch, verified against a real document at last

ADR 0820's table has one row reading **"not verified against a real file — no `/R` 5 document here
has a known owner password"**. qpdf's test suite publishes one: `c-r5-in.pdf` is `/V` 5 `/R` 5,
declares `/Extensions << /ADBE << /BaseVersion /1.7 /ExtensionLevel 3 >> >>`, and
`qpdf/qtest/encryption.test` names its user password `user3`, its owner password `owner3`, and —
in `c-r5-key-user.out` and `c-r5-key-owner.out` — the **file encryption key both of them unwrap**,
`35ea16a48b6a3045133b69ac0906c2e8fb0a2cc97903ae17b51a5786ebdba020`.

A published key is what turns "the owner branch reaches *a* key" into "the owner branch reaches
*the* key". Both passwords open the document here, `owner3` is reported as the owner and `user3`
is not, and page one comes out as `BT /F1 24 Tf 72 720 Td (Potato 0) Tj ET` — a producer's
operators from a producer's ciphertext.

`crates/pdf-syntax/tests/encryption.rs` gains that as a second revision-5 fixture, built from the
file's own `/O`, `/OE`, `/U`, `/UE`, `/Perms`, `/P` and page-one content stream. **The fixture ADR
0820 built is not weakened and not touched**: its owner entries were computed outside this tree
because nothing better existed, and the two fixtures now check the same branch from two directions.
The file itself is not vendored — what is here is an encryption dictionary and one stream, which is
all the algorithm touches.

## Decision 3 — a crypt filter name `/CF` does not define is not `Identity`

**The fixture failed, and the failure was ours.** `c-r5-in.pdf` states `/StmF /StdCF /StrF /StdCF`
and carries **no `/CF` dictionary at all**, so `StdCF` is named and never defined. `crypt_filters`
read Table 20's "Default value: Identity" as covering that, and the consequence was the worst
shape a defect comes in: the password authenticated, the document opened, `Document::permissions`
answered correctly from the `/Perms` block — and every stream in the file was handed to
`FlateDecode` still encrypted, with nothing said. Trap 5's subject exactly.

Table 20 says otherwise in two rows. `/StmF` and `/StrF`: "The name shall be a key in the CF
dictionary or a standard crypt filter name specified in "Table 26 - Standard crypt filter names"".
`/CF`: "Every crypt filter used in the document shall have an entry in this dictionary, except for
the standard crypt filter names …". The default is for an entry that is **absent**; a name that
resolves to nothing breaks both `shall`s, and the standard says nothing about what to do next.

What a reader may do is decided by `/V` rather than guessed:

- **`/V` 5** — Table 20's own row states the algorithm outright: the document is encrypted "using
  7.6.3.3, "Algorithm 1.A: Encryption of data using the AES algorithms" with a file encryption key
  length of 256 bits", and §7.6.4.1 limits this handler to "the Identity crypt filter … and crypt
  filters named StdCF". One method is available, so it is taken.
- **`/V` 4** — the row says "Algorithm 1 … with a file encryption key length of 128 bits", and
  Algorithm 1 is RC4 *or* AES: Table 25's `V2` and `AESV2` both answer to it. Nothing is
  determined, so the file is refused by name.

One more sentence of Table 20 is now obeyed that was not: "Any keys in the CF dictionary that are
listed in "Table 26 - Standard crypt filter names" shall be ignored by a PDF processor. Instead,
the PDF processor shall use properties of the respective standard crypt filters." A `/CF` entry
called `Identity` was previously allowed to supply a `/CFM`.

**Nothing in the corpus moves.** Every encrypted document there whose `/StmF` names `StdCF` either
carries a `/CF` — often as an indirect reference, which is what made a first byte-level scan report
thirteen false witnesses — or is `/V` 2, where crypt filters have no meaning. The change is here
because the clause says so and because a real file demonstrated the cost, not because a page
changed.

## The census: §7.6.5's public-key handlers, over everything this tree can reach

Asked of the whole 90 535 documents in `doc/pdf.js/test/pdfs`, `doc/corpora/` and `corpus-cache/`
— 2 374 of which name `/Encrypt` in their bytes and 2 360 of which state one in a trailer:

```sh
find -L doc/pdf.js/test/pdfs doc/corpora corpus-cache -type f -iname '*.pdf' > /tmp/all
xargs -a /tmp/all -d '\n' grep -lF /Encrypt > /tmp/enc
xargs -a /tmp/enc -d '\n' cargo run --profile gates -p pdf-model --example encryption_census
```

**Five documents use a public-key security handler.** Every one of them is `/Filter
/Adobe.PubSec` with `/SubFilter /adbe.pkcs7.s5` and a single-recipient `/Recipients` array — a CMS
`EnvelopedData` whose one `KeyTransRecipientInfo` wraps the content-encryption key under RSA
(`1.2.840.113549.1.1.1`), the enveloped content itself being AES-128-CBC
(`2.16.840.1.101.3.4.1.2`) in all five:

| document | corpus | `/V` | `/CFM` | key |
|---|---|---|---|---|
| `3006236.pdf` | SafeDocs `cc-main-2021-31` | 5 | `AESV3` | 256 |
| `PDFBOX-4421-0.pdf` | tika-issue-tracker `batch1` | 4 | `AESV2` | 128 |
| `PDFBOX-4421-1.pdf` | tika-issue-tracker `batch1` | 5 | `AESV3` | 256 |
| `PDFBOX-4421-2.pdf` | tika-issue-tracker `batch1` | 4 | `AESV2` | 128 |
| `PDFBOX-4421-3.pdf` | tika-issue-tracker `batch1` | 5 | `AESV3` | 256 |

All five name their crypt filter `DefaultCryptFilter`, which is what §7.6.4.1 requires of a
public-key handler "when all document content is encrypted".

**The refusal is not costing real documents, and the reason is stronger than the count.** Four of
the five are one bug report's attachments — PDFBOX-4421 is Apache's own public-key issue — and the
fifth, `3006236.pdf`, names its recipient's certificate
`zune-tuner://windowsphone/b46fd244 - cd539804 - e37e34e4 - 6f90f8c0`: a document encrypted to a
*device*, whose private key was never the reader's. **§7.6.5.1's single `shall` on a reader is to
"scan the recipient list … and … attempt to find a match with a certificate that belongs to the
user", and for all five of these the honest answer to that scan is no match.** Implementing the
clause would turn five loud refusals into five loud refusals with more code behind them.
`doc/todo/51` carries this as the finding rather than as work owed.

Two documents state a `/Filter` that is neither `/Standard` nor a public-key handler, and neither
is a handler question:

- `PDFBOX-4351-0.pdf` writes `/Filte^/Standard` — one byte of the *key name* corrupted — so the
  reader finds no `/Filter` in the encryption dictionary and reports the nearest one it does find,
  `/FlateDecode`. The refusal is correct and its message names the wrong entry.
- `GHOSTSCRIPT-695040-0.zip-77.pdf` has a well-formed `/Filter /Standard /V 1 /R 2` dictionary in
  its body that the trailer's `/Encrypt` does not reach. Recovery, not encryption.

Both are recorded in `doc/todo/51` for whoever is next in this area.

## Consequences

- ADR 0820's "one step that rests on a reading" is closed, and its owner row is verified against a
  real file. Its own text is left as it was written — an ADR is not edited to follow later work —
  and the row is amended in place with a pointer here.
- `doc/todo/51`'s "**Public-key handlers (§7.6.5) — 0 corpus documents**" heading was wrong by five,
  and its "`/R` 5 — 1 document … there is nothing to implement" section was overtaken by ADR 0820
  five sessions before this one. Both are corrected.
- The §7.6.5 ledger row's count moves from one document to five, with the shapes beside it.
- A round that cannot reach something says so **after trying it**, not before. `doc/habits.md`.
