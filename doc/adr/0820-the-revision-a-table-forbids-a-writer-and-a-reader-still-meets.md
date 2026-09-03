# 0820 — The revision a table forbids a writer, and a reader still meets

Session 887. Status: **accepted**.

## Context

The project owner, 2026-09-03:

> the deprecated encryption algorithms are still used quite a lot because the itext library
> generates them. please implement them in one of the following rounds. you can extract the
> algorithm from Apache pdfbox, which can decrypt them.

`crates/pdf-syntax/src/crypt.rs` refused revision 5 by name, and had since the twenty-second
session. Its module comment, the corpus gate's `MAX_UNREADABLE_ENCRYPTION`, the oracle's
`NO_RENDER_ENCRYPTION_THE_STANDARD_DOES_NOT_STATE` and four ledger rows all rested on one
sentence of Table 21 and on one reading of it:

> 5 ( PDF 2.0; deprecated in PDF 2.0 ) Shall not be used. This value was used by a deprecated
> proprietary Adobe extension.

The reading was: *the standard states no algorithm for it, so implementing one would be
curve-fitting to another implementation.* The corpus gate called it "the one row in this gate
that is a **decision** rather than work owed".

## The tension with principle 5, argued rather than glossed

This is the interesting part of the round, and it deserves to be settled in the open.

**1. "Shall not be used" binds a writer.** Table 21's `/R` row is a table of values a producer
may store; the whole row is instructions for choosing one — *2 if the document is encrypted with
a V value less than 2*, *3 if…*, *4 if…*, *6 if the document is encrypted with a V value of 5*.
Read in its own column, the entry for 5 is "do not choose this". A reader chooses nothing. It
meets whatever a file says, and `CLAUDE.md`'s statement of *done* is unambiguous about what it
then owes: **every PDF that exists renders as its producer specified.** A file iText already
wrote exists.

**2. The standard does not fall silent about revision 5 — it states a requirement about it.**
§7.6.4.1, in the paragraph on crypt filters:

> If a security handler of revision 4 or 5 is specified, the standard security handler shall
> support crypt filters (see 7.6.6, "Crypt filters").

A clause cannot place a `shall` on a processor for a revision it treats as having no meaning.
The same paragraph ends:

> Use of security handler revisions 1, 2, 3, 4 and 5 is deprecated in PDF 2.0.

Revision 5 is deprecated in exactly the company of revisions 2, 3 and 4 — every one of which
this module has read since the twenty-second session, without anybody suggesting that
deprecation was a reason not to.

**3. Table 21 says where the algorithm is.** "This value was used by a deprecated proprietary
Adobe extension" is a *pointer*, not an absence. The extension is the **Adobe Supplement to ISO
32000-1, BaseVersion 1.7, ExtensionLevel 3**, and files declare it in the catalogue:
`/Extensions << /ADBE << /BaseVersion /1.7 /ExtensionLevel 3 >> >>`, which 8 of the 41 `/R 5`
documents the census below counts do, in so many words.

**4. Most of the algorithm is not the extension's at all.** Table 20's `/V` 5 entry — normative,
current, and what every one of these files writes — sends the *data* to §7.6.3.3's Algorithm 1.A
"with a file encryption key length of 256 bits". That is the same cipher path this module already
ran for revision 6, unchanged. What the extension supplies is the key retrieval and the two
password validations, and those are §7.6.4.3.3's Algorithm 2.A with §7.6.4.3.4's iterated hash
replaced by a single SHA-256.

So the entry in `CLAUDE.md`'s own list of decayed claims gains a new instance, and it is the
shape that file warns about: **"the specification defines nothing here" is itself a claim about
the specification, and it decays.** This one survived eight hundred sessions.

## What could not be obtained, said plainly

**One sentence of this section is false and session 892 proved it by doing the thing.** "[T]here is
no network access from this account" was inferred rather than tested; DNS, TLS and HTTPS all work
from this shell, and the supplement, PDFBox's `StandardSecurityHandler.java` and a `/R` 5 document
with a published owner password were all fetched in minutes. The section is left standing because
it is the record of what this round believed, and because what it cost is the lesson: a round
reasoning from an untested constraint gave up the one document that answers its own open question.
ADR 0829, and `doc/habits.md`.


**The supplement itself is not in this tree and could not be read.** `doc/md/` holds fourteen ISO
and PDF Association documents and the supplement is not among them; there is no network access
from this account. **PDFBox is not on this machine either** — the owner's suggested source. A
`find` for `*pdfbox*` and a look through `/usr/share/java` return nothing, so the Java the owner
pointed at was never available to read.

What *is* on this machine is `doc/pdf.js/src/core/crypto.js`, a full pdf.js checkout carried for
its corpus. Its `PDF17` class is one line — `calculateSHA256(input)` where `PDF20` runs Algorithm
2.B — dispatched from the same `#createEncryptionKey20` that serves revision 6. **That is evidence
about a reading, in principle 5's one permitted direction, and it is cited nowhere in the code as
an authority.**

The rest of the confidence comes from somewhere better: **a real document, decrypted.**
`doc/pdf.js/test/pdfs/issue21579.pdf` is `/V 5 /R 5`, and its page one comes out as

> `BT /F1 16 Tf 24 150 Td (repro1a: AES-256 R5, password 'passwoert' with umlauts) Tj …`

A 256-bit hash equality does not hold by accident, and a wrong AES key does not produce balanced
PDF operators. Each row of the table below says which of those two kinds of authority it has.

## Decision

Implement revision 5, in the shape `crypt.rs` already uses for 2, 4 and 6.

`Aes256Hash` is the whole of the difference — `UnIterated` for revision 5, `Iterated` for 6 — and
`authenticate_aes256` runs Algorithm 2.A's steps once for both. `hash_2b`'s own first line ("Take
the SHA-256 hash of the original input to the algorithm and name the resulting 32 bytes, K") is
now the function `sha256_of`, which is *also* the whole of revision 5's hash; the two revisions
therefore share the code down to the last call, and nothing about revision 5 is a second copy of
anything.

### Step by step, and where each one's authority comes from

| step | authority | how it is known to be right here |
|---|---|---|
| admitting `/R 5` at all | §7.6.4.1's two sentences above; Table 21 read as binding a writer | **argument**, section above |
| data: AES-256 CBC, IV in front, PKCS#7, key used directly | **normative** — Table 20's `/V` 5 row, §7.6.3.3's Algorithm 1.A, §7.6.3.1 | the corpus document's content stream decrypts to legible operators |
| `/O` and `/U` as 32-byte hash ‖ 8-byte validation salt ‖ 8-byte key salt | §7.6.4.3.3's preamble, normative for revision 6 | **verified**: `SHA-256(password ‖ U[32..40])` equals `U[0..32]` exactly on the corpus document |
| user validation: `SHA-256(password ‖ user validation salt)` against `U[0..32]` | supplement's Algorithm 3.2a; §7.6.4.4.10's Algorithm 11 with the hash substituted | **verified**, same equality |
| user key: that hash over the key salt, then AES-256-CBC, zero IV, no padding, over `/UE` | supplement; §7.6.4.3.3 step (e) with the hash substituted | **verified**: the 32 bytes it yields decrypt both the page and the `/Perms` block |
| `/Perms`: AES-256 ECB, `"adb"` at bytes 9–11, `/P` little-endian at 0–3, `T`/`F` at byte 8 | §7.6.4.4.12's Algorithm 13, **verbatim and unchanged** | **verified**: the block gives back exactly the `/P -1084` and `/EncryptMetadata true` written in the clear beside it |
| owner validation and owner key, salted with the whole 48-byte `/U` | §7.6.4.3.3 steps (c) and (d) with the hash substituted | **verified against a real file since session 892** — qpdf's `c-r5-in.pdf` publishes both passwords *and* the file encryption key they unwrap, and both branches reach it (ADR 0829). When this row was written it read *not verified*: no `/R 5` document here had a known owner password, and the fixture's `/O` and `/OE` were computed **outside this tree** around the real `/UE`'s key |
| `/U` and `/O` longer than 48 bytes: read the three sections off the front | **evidence** | 8 of the 41 `/R 5` documents the census below counts write both as 127 bytes — 48 significant followed by NUL — and they are exactly the 8 declaring ExtensionLevel 3 |
| crypt-filter pairing: `AESV3` or `Identity`, never `V2` or `AESV2` | Table 20's `/V` 5 row; §7.6.4.1's own sentence names revisions 4 and 6 and skips 5 | argument; a refusal by name rather than a 32-byte key handed to AES-128 |
| **password preprocessing: SASLprep, then UTF-8, then truncate to 127 bytes** | **settled in session 892** — the `ExtensionLevel` 3 supplement states it twice, once in Algorithm 3.2a step 1 and once with the Normalize and BIDI options named (ADR 0829). When this row was written it was the one step resting on a reading rather than on a checked sentence | see below, and read it as the record of what the reading was |

### The one step that rests on a reading

**Session 892 settled this, and the section below is left as it was written.** The supplement was
fetched — over the network this round believed absent — and it states the preparation twice, in
Algorithm 3.2a step 1 and again under *Password Algorithms* with the Normalize and BIDI options
named. The reading recorded below was right, and what follows is the record of a reading made
without the document rather than a live question. ADR 0829.


§7.6.4.3.3's preamble binds steps (a) and (b) to "[w]henever UTF-8 password is used below", and
"below" is the algorithm revision 5 shares. Taking the structure from Algorithm 2.A and its
password handling from somewhere else would be incoherent, so this reader applies SASLprep at
revision 5 as it does at revision 6. What is recalled of the supplement's Algorithm 3.2a step 1
agrees — but **this tree cannot check that against a copy**, and saying so is worth more than a
confident citation.

**pdf.js does it the other way**: `saslPrep` at revision 6, plain UTF-8 at revision 5. The
disagreement is very nearly unobservable — SASLprep is the identity on every ASCII password and on
the empty one, which is what 32 of these 41 documents open with — and it is invisible on the one
witness there is, whose password `pässwört` SASLprep leaves alone. The residual risk is stated
rather than hidden: a revision-5 password containing a character SASLprep *prohibits* would be
refused here and accepted by pdf.js. That is the same risk revision 6 already carries, and if a
document ever demonstrates it, this row is where to come back to.

### A refusal that was silent is now typed

Algorithm 2.A steps (d) and (e) both end "[t]he 32-byte result is the file encryption key", and
§7.6.3.3 takes a 256-bit key. `file_key_from` now checks that length for **both** revisions: an
`/OE` or `/UE` that unwraps to anything else is a wrong-password refusal at the door instead of a
stream that quietly declines to decrypt several layers further down (trap 5).

## The census

**The population is every corpus this tree can reach**, which `conformance --bin undenominated`
counts at 90 535 documents: `doc/pdf.js/test/pdfs`, the four submodules under `doc/corpora/`, and
`corpus-cache/`'s `safedocs` crawl, `tika-issue-tracker` and `openpreserve`. Across all of it, 41
documents state `/Filter /Standard` with `/R 5` — 1.7% of the 2 368 encrypted files and 0.045% of
the whole. 20 are in the crawl, 20 in the tracker, 1 in `doc/pdf.js`, and none in the four
submodules or `openpreserve`. Every one is `/V 5`, `/Length 256`,
`/StmF /StdCF /StrF /StdCF`, `StdCF` = `AESV3`, `/UE` and `/OE` 32 bytes, `/Perms` 16.

Before this round **all 41 were refused by name**, the refusal preceding authentication so that
none could reach it. After: **33 open** — 32 on the empty password §7.6.4.1 has a reader try
first, and `issue21579.pdf` on the `pässwört` `encryption.rs` records — **8 are locked**, wanting
a password nobody here has, and **0 are refused**. 19 of the 33 withhold at least one of the two
operations this program has, through the encrypted `/Perms` block rather than the plaintext `/P`.

Two ratchets move: the corpus gate's `MAX_UNREADABLE_ENCRYPTION` from 2 to 1 and its `MAX_LOCKED`
from 8 to 9, which is the same document moving from *refused* to *one sentence away*.

## Tests, and why one of them is a fixture in a file that argues against fixtures

`tests/encryption.rs` opens with a paragraph on why this clause is tested against real documents:
a fixture would have to *encrypt*, which is running the reader's own algorithms against
themselves. The owner asked for a fixture so the regression does not depend on the submodule, and
both hold at once: **the fixture is assembled around `issue21579.pdf`'s own `/U`, `/UE`, `/Perms`
and page-one ciphertext**, lifted verbatim. What is fabricated is the catalogue, the page tree and
the cross-reference table, which no cipher touches. Bytes this code did not encrypt still have to
come back out as their producer's operators.

Six tests: the user password opens it and the empty one does not; the owner password reaches the
same key (with `/O` and `/OE` computed outside this tree by an independent implementation of the
same steps); a third password is refused; the `/Perms` block overrules a plaintext `/P` of −1;
the 127-byte padded shape opens on both branches; and the corpus document itself opens, so that
if the fixture and the file ever disagree the file wins.

Two refusals keep their tests: `/R 7`, which Table 21 does not list, and a `/R 5` file whose
`/CFM` is `AESV2`.

## Consequences

- `CLAUDE.md`'s decayed-claim list gains its sharpest instance yet — a *refusal* that outlived
  its reason for eight hundred sessions, held in place by a sentence everybody read as addressed
  to us.
- Trap 27's illustration is now the tree's behaviour, and the trap is amended to say so without
  losing its incident.
- Revision 5's Table 22 flags reach `pdf_model::restriction` for the first time, which is
  `doc/todo/38`'s subject and is recorded there.
- What is still owed under §7.6 is unchanged and is the same short list: §7.6.5's public-key
  handlers, the operations §7.6.4.1 names that this program does not have, and a revision-4
  password containing a character `PDFDocEncoding` has no code for.
