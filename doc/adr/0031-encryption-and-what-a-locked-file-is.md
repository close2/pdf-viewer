# ADR 0031 — Encryption, and the difference between a locked file and an unreadable one

Status: accepted, 2026-07-30.

## Context

Encryption was the largest thing left on the demand list that is straightforwardly code:
**20 corpus documents**, of which the handover said 11 could not reach page one at all and 9
more drew a blank page. It was also the largest hole in clause 7, which is otherwise the
tree's most complete clause, and §7.6 is a clause family of **34 subclauses, every one of
them `unreviewed`** before this session. Demand item and spec item are the same family
again, which is the ninth session's ideal shape and the fourth time it has been available.

What the corpus actually held turned out to be sharper than "20 documents". Twenty-six files
carry an `/Encrypt`, and between them they use every revision Table 21 specifies and every
crypt filter method Table 25 names: revision 2 with 40-bit RC4, revision 3 with 128-bit RC4,
revision 4 with both `V2` and `AESV2`, and revision 6 with `AESV3`. Seven have passwords the
pdf.js manifest records. That is a better test set than anything that could be written here,
because a fixture for this clause would have to *encrypt*, which means running the same
algorithms the reader runs and comparing them with themselves.

## Decision

### The primitives are taken, not written

`aes`, `cbc`, `rc4`, `md-5` and `sha2` from RustCrypto, plus `stringprep` for §7.6.4.1's
SASLprep. The alternative was about 700 lines of in-tree cipher code, every line of it
covered by published test vectors and none of it reviewed by anyone but us.

This follows the precedent ADR 0014 set for JBIG2 and JPEG 2000 and ADR 0006 for font
parsing: a widely-reviewed implementation of a published algorithm beats a fresh one, and
cryptography is the domain where that argument is strongest. The cost is written down rather
than assumed away — twelve transitive crates, and `aes` reaches x86 AES-NI intrinsics
through `unsafe`, which is the same posture `tiny-skia`, `flate2` and `zune-jpeg` already
have. `#![forbid(unsafe_code)]` continues to mean what it has always meant here: no `unsafe`
in *our* code.

What is *not* delegated is the clause. Every algorithm §7.6 numbers — 1, 1.A, 2, 2.A, 2.B, 3
in part, 4, 5, 6, 7, 11, 12, 13 — is written out in `crates/pdf-syntax/src/crypt.rs` against
its own subclause, because those are the part a library cannot supply.

### Decryption happens where an object is loaded

§7.6.3.3 states where: "Stream data shall be encrypted after applying all stream encoding
filters and shall be decrypted before applying any stream decoding filters." So `Document`
decrypts an indirect object as it parses it, and the filter chain above needs no change at
all. §7.6.3.2 step (a) makes the same choice for strings from the other side — "If the string
is a direct object, use the identifier of the indirect object containing it" — so the unit of
decryption is a whole object walked from its own identifier, not a string found in place.

That placement is also what makes §7.6.2's four exceptions expressible. The trailer's `/ID`
is read before a handler exists; the encryption dictionary is exempted by object number,
recorded before authentication; an object inside an object stream is parsed out of
already-decrypted bytes and so is exempt by construction; and a signature dictionary's
`/Contents` is skipped by name.

### A failure to decrypt is loud, which cost a field on `Stream`

The first draft replaced an undecryptable stream's data with nothing. That is precisely the
failure `CLAUDE.md` principle 3 forbids: a page whose `/Contents` silently became empty draws
blank and reports `unsupported: []`, which is indistinguishable from a page that *is* blank.
`Stream` therefore carries `decryption_failed`, and `Document::decoded_stream_data` refuses
such a stream, which every caller already treats as something to report.

Encryption is syntax — ISO 32000-2 puts it in clause 7 — so the flag is not a layering
violation, but it is a fact about a stream that the reader must not be able to lose.

### A locked file is not an unreadable one, and the gate now says which

`Document::open` uses the empty password, which §7.6.4.1 requires a reader to try first, and
returns `SyntaxError::PasswordRequired` when that fails. The corpus gate used to have one
bucket for "cannot be opened", ratcheted at zero and meaning *every file yields something*.
Eight documents would have broken it.

They should not: a file that needs a password is waiting for a person, not for work. The gate
now counts three things where it counted one — unopenable (0, and it should stay there),
**locked (8)**, and **encrypted in a way this reader does not implement (2)** — and the third
is the only one that is work owed. Of those two, one is a decision rather than a debt.

### Revision 5 is refused, and that is a reading of the standard rather than laziness

`issue21579.pdf` writes `/R 5`. Table 21's entry for that value is complete:

> 5 ( PDF 2.0; deprecated in PDF 2.0 ) Shall not be used. This value was used by a deprecated
> proprietary Adobe extension.

The standard states no algorithm for it. Implementing it would mean reading another renderer's
source and copying what it does, which is the one thing principle 5 forbids outright. It is
refused by name, with the clause quoted in the error.

The same reasoning refuses §7.6.5's public-key handlers — but for the opposite reason, and the
distinction is worth keeping. Those *are* specified; what they need is CMS enveloped data,
X.509 certificates and access to the user's private keys, which is a public-key infrastructure
and a threat model rather than a cipher. That is a debt, and its ledger rows say `reported`.

### A document whose attachment alone is encrypted opens without a password

The session's one genuine reading finding, and it changed two documents.

`encrypted-attachment.pdf` and `auth-event-ef-open.pdf` write `/StmF /Identity /StrF
/Identity` with a `StdCF` reached only through `/EFF`. Their `/U` entries authenticate against
no password anybody has — checked with three independent implementations of Algorithm 2.A, not
assumed. `mupdf` and `ghostscript` refuse them ("This file requires a password for access");
`poppler` opens them and reports `Encrypted: no`.

Two against two is not a tie. §7.6.6 answers it:

> Authorization to decrypt a stream shall always be obtained before the stream can be
> accessed.

and

> PDF readers and security handlers shall treat any attempt to access a stream for which
> authorization has failed as an error.

Both sentences bind the failure to *a stream*, not to the file. `/AuthEvent /DocOpen` says
*when* authorization is attempted, not what fails if it does not succeed. A document none of
whose own strings and streams are encrypted needs no key to be read, so it is read; the
attachment, which does need one, refuses. §7.6.4.1's "Documents in which only file attachments
are encrypted shall use the same user and owner passwords" is the sentence that shows the
standard has this arrangement in mind.

`Encryption::authenticated` is what carries the consequence: with no key, every method except
`Identity` returns `None`, which reaches the caller as a refusal rather than as plaintext.

### Table 22's permissions are read and carried, and enforced nowhere

§7.6.4.1 says both halves of this plainly:

> Once the document has been opened and decrypted successfully, a PDF reader technically has
> access to the entire contents of the document. There is nothing inherent in PDF encryption
> that enforces the document permissions specified in the encryption dictionary. PDF readers
> shall respect the intent of the document creator by restricting user access to an encrypted
> PDF file according to the permissions contained in the file.

That is an obligation on copying, printing and editing — none of which this application has
yet. `Document::permissions` returns Table 22's flags plus which password matched, and the
ledger row for §7.6.4.1 is `partial` naming exactly that. Building a viewer that ignores them
later would be the defect; a renderer that does not restrict *drawing* is not.

One consequence of reading the clause rather than the table: at revision 6 the permissions
come from the encrypted `/Perms` block when it is readable, because §7.6.4.3.3 step (f) says
its bytes *are* the user permissions and `/P` beside it is the copy a file can be edited to
change.

### PDFDocEncoding is refused rather than guessed

§7.6.4.3.2 step (a) wants a revision-4 password "converted to PDFDocEncoding", and that
encoding is Annex D Table D.2 — a glyph name per code, which `pdf-font` holds and `pdf-syntax`
sits below. What *is* derivable from the table without the glyph names is where the encoding
and Unicode agree: every code from 0x20 to 0x7E and from 0xA1 to 0xFF is its own code point,
and every code that is not — 0x18 to 0x1F, 0x80 to 0x9F, and 0xA0, which is EURO SIGN rather
than a no-break space — encodes a character outside those ranges.

So a password made of characters in the agreeing ranges converts exactly and one that is not
is refused by name. No corpus document needs more, and moving Annex D below `pdf-syntax` for
one rarely-trodden path is a change worth more argument than the path is worth.

## Consequences

**Nineteen of the corpus's twenty-six encrypted documents open with the default user password
and the other seven with theirs.** Eight documents draw with nothing reported at all, two more
reach page one and report something narrow, and eight say they need a password. The corpus
gate's `Content` row was 10 and is 1; its `Operator` row was 12 and is 9. Both fell for the
same reason: nine of those `Content` reports were a `/Contents` refusing to inflate because it
was ciphertext, and three of the `Operator` reports were the same ciphertext lexing as
operator names — `issue15893_reduced.pdf` announced an operator called `)` and two more of
byte soup.

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 808 | **816** |
| corpus documents reporting something | 147 | **137** |
| documents with no reachable page one | 19 | **11** |
| documents needing a password | — | **8** |
| documents encrypted beyond this reader | — | **2** |
| pages we call complete, in the oracle | 1603 | **1611** |
| of those, agreeing with the reference consensus | 742 | **747** |
| of those, contradicted | 102 | **102** |
| ledger subclauses nobody has read | 561 | **524** |
| `§` citations the checker verified | 891 | **1000** |
| tests | 451 | **473** |

**Eight pages joined the judged set and five agree outright, with no new contradiction** —
the first session in five where the contradicted count did not move at all.

**Interpretation costs nothing measurable**: 1.9398 G instructions to 1.9340 G by callgrind on
`examples/callgrind_interpret`, baseline measured on this machine at the previous commit. That
is −0.29%, which is a refactor of the object-loading path rather than a win worth claiming;
the honest statement is that an unencrypted document pays nothing.

**An encrypted document pays what the standard designed it to pay.** Opening a revision 4 or
earlier document costs 74 to 183 µs against 275 µs for an unencrypted one — Algorithm 2 is
fifty MD5s. Revision 6 costs **1.5 to 2.9 ms**, because §7.6.4.3.4's Algorithm 2.B runs at
least sixty-four rounds of AES over sixty-four repetitions of the password, and its own NOTE 2
says why: "The reason for multiple rounds is to defeat the possibility of running all paths in
parallel." That lands squarely on the time-to-first-page path and is not a defect to optimise;
it is the clause working.

### What it taught

- **A rule whose common case is the identity is a rule nobody tests.** §7.6.4.3.2 step (a)
  says to append "the first 32 − n bytes of the padding string"; the first implementation
  overlaid the password onto the padding string in place. For the empty password — which
  §7.6.4.1 makes every reader try first — the two are the same 32 bytes, so **all nineteen
  password-less documents opened correctly and every document with a password was refused**.
  The unit test written beside the code asserted the wrong thing, because it was written from
  the code rather than from the clause.
- **Two references agreeing can still be two answers to the wrong question.** Trap 9's third
  shape again: `mupdf` and `ghostscript` both refuse the attachment-only files, and §7.6.6
  says the refusal belongs to the stream rather than to the document.
- **A bucket that means "we failed" must not also mean "you have not told us the password".**
  The corpus gate's `unopenable` ratchet was right to fire and wrong to be widened; splitting
  it kept an invariant worth having.
- **The corpus cannot exercise §7.6.2's signature exception, and that is now a measured fact
  rather than a suspicion.** Eight corpus documents carry a signature dictionary and
  twenty-six carry an `/Encrypt`, and **the two sets are disjoint** — so the one rule in this
  clause that had to be *derived* (Table 255 makes `/Contents` a hexadecimal string, so the
  exception is about the dictionary) is defended by a unit test on the predicate and by
  nothing else. Third consecutive session to find a load-bearing rule no real file reaches.
