# 892 — The document that defines revision 5 is fetched and settles its own question, an owner password published by qpdf verifies the branch ADR 0820 could not, and a crypt filter name `/CF` never defines stops being read as `Identity`

Date: 2026-09-03.
ADR: [0829](../adr/0829-the-supplement-read-and-a-crypt-filter-name-that-defines-nothing.md),
amending [0820](../adr/0820-the-revision-a-table-forbids-a-writer-and-a-reader-still-meets.md).
Touched: `crates/pdf-syntax/src/crypt.rs`, `crates/pdf-syntax/tests/encryption.rs`,
`doc/conformance/ledger.toml`, `doc/todo/51-signatures-and-public-keys.md`,
`doc/todo/README.md`, `doc/habits.md`, `doc/adr/0820-…`, `doc/adr/0829-…`.
Asked for by the project owner as a follow-up to round 887.

## The constraint that was not there

Round 887 implemented `/R` 5 and wrote into ADR 0820 that the Adobe supplement defining it "could
not be read … there is no network access from this account", then settled a step of §7.6.4.3.3
from a reading instead. **There is network access, and there always was.** DNS, TLS and HTTPS all
answer from this shell. Five minutes produced the supplement, Apache PDFBox's
`StandardSecurityHandler.java`, and a `/R` 5 document with a published owner password — every one
of the three things that round recorded as unobtainable, including the one that answers its own
open question.

That is the round's most transferable finding and it is in `doc/habits.md` beside the other claims
that decay: **an environmental limit a round is about to reason from is one command away from
being checked.**

## SASLprep at revision 5 — the code was right, and now it is quoted

pdf.js applies `SASLprep` only at revision 6; PDFBox guards it with `dicRevision == REVISION_6` on
both the reading and the writing side. Two implementations agreeing is a question to take back to
the standard, and this one had to be taken to a *different* document, because ISO 32000-2 states no
key retrieval for revision 5 at all: §7.6.4.3.3's preamble binds steps (a) and (b) to the
algorithm, while the clause's own title binds the clause to "revision 6 and later". Both readings
are available.

The Adobe Supplement to ISO 32000-1, `BaseVersion` 1.7, `ExtensionLevel` 3 (Acrobat 9.0 SDK, June
2008, fetched from the Internet Archive — Adobe's live URL now serves `ExtensionLevel` 5, which is
seven pages about transparency and XFA and has no encryption in it) states the preparation twice.
Algorithm 3.2a step 1, and again under *Password Algorithms* with the two options named: "applying
the “SASLPrep” profile … using the Normalize and BIDI options". ADR 0829 has both verbatim.

So the tree is right and two implementations are narrower than the document they implement. Every
other step ADR 0820 derived without the supplement agrees with Algorithm 3.2a as well — the three
sections of `/O` and `/U`, both validations, both key unwraps, the `/Perms` block.

## The owner branch, against a real file

ADR 0820's step table had one row reading *not verified against a real file*. qpdf's test suite
publishes `c-r5-in.pdf`: `/V` 5 `/R` 5, `/Extensions /ADBE /ExtensionLevel 3`, user password
`user3`, owner password `owner3`, and — in `c-r5-key-user.out` and `c-r5-key-owner.out` — **the
file encryption key both of them unwrap**. A published key is what turns *reaches a key* into
*reaches the key*. Both passwords open it here, `owner3` is reported as the owner and `user3` is
not, and page one comes out as `BT /F1 24 Tf 72 720 Td (Potato 0) Tj ET`.

`tests/encryption.rs` gains that as a second revision-5 fixture, built from the file's own eight
constants. ADR 0820's fixture is untouched; the two now check the same branch from two directions.

## What the real file found

**The fixture failed the first time it ran, and the failure was ours.** `c-r5-in.pdf` writes
`/StmF /StdCF /StrF /StdCF` and **no `/CF` dictionary at all**. `crypt_filters` read Table 20's
"Default value: Identity" as covering a name that resolves to nothing — so the password
authenticated, `permissions()` answered correctly out of the `/Perms` block, and every stream in
the document went to `FlateDecode` still encrypted, silently. Trap 5 exactly, and it is what a real
producer's bytes are for (trap 4).

Table 20 says the default is for an entry that is **absent**: both rows also say "[t]he name shall
be a key in the CF dictionary or a standard crypt filter name specified in Table 26", and `/CF`'s
row says every filter used "shall have an entry in this dictionary". What a reader may do instead
is decided by `/V` rather than guessed — at `/V` 5 Table 20's row states Algorithm 1.A with a
256-bit key and §7.6.4.1 allows only `Identity` and a filter "named StdCF", so one method is
available; at `/V` 4 the row says Algorithm 1, which is RC4 *or* AES, so nothing is determined and
the file is refused by name. One more sentence is obeyed that was not: a `/CF` entry called
`Identity` no longer supplies a `/CFM`.

No corpus document moves. A first byte-level scan said thirteen do and every one was the scan
reading `/CF 7 0 R` — an indirect reference — as an absent dictionary.

## The public-key census

Over all 90 535 documents in `doc/pdf.js`, `doc/corpora/` and `corpus-cache/`: 2 374 name
`/Encrypt` in their bytes, 2 360 state one in a trailer, and **five** use §7.6.5's public-key
handler — `3006236.pdf` from the SafeDocs crawl and `PDFBOX-4421-0` through `-3` from the
tika-issue-tracker, all `/Filter /Adobe.PubSec /SubFilter /adbe.pkcs7.s5` with a single-recipient
CMS `EnvelopedData` under a `DefaultCryptFilter`.

**The refusal is not costing real documents.** Four are one bug report's attachments; the fifth is
encrypted to a *device* — its recipient certificate is named
`zune-tuner://windowsphone/b46fd244 - …` — so §7.6.5.1's one `shall` on a reader, to find a
certificate "that belongs to the user", has no match to find in any of the five. `doc/todo/51`
carries that as the finding; its heading read "0 corpus documents" and its `/R` 5 section still
said "there is nothing to implement" five sessions after ADR 0820 implemented it. Both corrected.

Two further documents state a `/Filter` that is neither `/Standard` nor a handler, and neither is
an encryption question: `PDFBOX-4351-0.pdf` corrupts the *key name* to `/Filte^`, and
`GHOSTSCRIPT-695040-0.zip-77.pdf`'s trailer `/Encrypt` does not reach the well-formed dictionary in
its body. Both are recorded in `doc/todo/51`.
