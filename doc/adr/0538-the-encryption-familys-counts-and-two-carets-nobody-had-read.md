# ADR 0538 — The encryption family's counts, and two carets nobody had read

Status: accepted, 2026-08-23. Session the six-hundred-and-ninety-first, a clause round under
`doc/todo/01`, taking the `partial` rows of one family off the blame list rather than a band across
families. Amends §7.6, §7.6.4, §7.6.4.1, §7.6.4.2, §7.6.4.4.2 and §7.6.6 in the ledger; adds
`crates/pdf-model/examples/encryption_census.rs`; corrects two doc comments in
`crates/pdf-syntax/src/crypt.rs`; adds one section to `doc/errata-read.md`. Extends ADRs 0031, 0212
and 0426; changes nothing any of them decided, and moves no status.

## 1. Why one family rather than a band

The blame ordering put §7.6.4 and §7.6.4.4 at ranks 1 and 2, §7.6.6 at 7 and §7.6 at 15 — four rows
of one clause family in the top sixteen, which is what happens when two sessions write a family's
notes in two sittings and nobody comes back. Reading them together is what made three of this
round's four findings visible at all: **every one of them is a disagreement between two rows of the
same family**, and a band that takes one row from each of sixteen families cannot see that shape by
construction.

That is not an argument against the band. It is an argument for the *choice within* it, and it
sharpens ADR 0455's rule one turn: rank by blame, prefer a reason that is a claim about this
codebase — and then, where the band's top is several rows of one family, read the family.

## 2. What was wrong

| row | shape | was | is |
|---|---|---|---|
| **§7.6** | a count with no command, hiding a refusal | "19 of the corpus's 26 encrypted documents open with the default user password and the other 7 with theirs" | 19 + 7 = 26 accounts for every encrypted document as *opening*, and two are refused by name — a fact this family's own §7.6.4.2 row states and `corpus.rs`'s `MAX_UNREADABLE_ENCRYPTION` ratchets. Measured: 25 state an `/Encrypt`, 15 open on the default, 8 on the passwords `encryption.rs` records, 2 are refused |
| **§7.6.4.2** | the same, four figures deep | "26 documents carry an /Encrypt, 19 open, 4 of those as the owner … and 6 withhold one of the two operations" | 25, 23, **3** and 6 — only the last survives |
| **§7.6.4** | 620's second shape — the *status*'s stated reason is not a debt | `partial`, on "revision 5 is refused by name" | §7.6.4.2 is `implemented` carrying that same refusal, because the standard states no algorithm for revision 5 at all. A refusal on the standard's own account is not something owed; what is owed is the three `partial` rows below, which the note now names |
| **§7.6.4.4.2** | 1, understated | "`print_protection.pdf`, **the one** whose known password is the owner's" | three of the eight authenticate as the owner, and `encryption.rs::an_empty_password_may_be_the_owner_password` has asserted one of them — `pr6531_2.pdf` on the *empty* password — since it was written |
| **§7.6.6** | 17th sweep's — no erratum recorded | — | Issue #74 amends the sentence that would make this reader's own behaviour non-conforming, and Issue #184 settles an ambiguity a source comment still asserts. §3 |

The status of every row is unchanged. `partial` was right in each case; what was wrong was three
counts, one superlative and one stated reason.

## 3. The two carets, and the rule they are the fifth instance of

Both are a **`Caret` with no `StrikeOut`**, which `spec-errata check` cannot see: its whole
discriminator is a quotation matching text an erratum struck, and an erratum that only *adds* has
struck nothing. `emit` is the only instrument that reads one, which is why `doc/todo/01`'s standing
instruction — run `emit` on the document before writing, rather than `check` afterwards alone —
paid again here.

**Issue #74 licenses `/V` 5.** §7.6.6's first bullet as printed ends "the value of the V entry
shall be 4 to use crypt filters"; the caret inserts "or 5" after the 4. This matters because
Table 25 gives `AESV3` no home other than a `/CF` entry and Table 20 requires `/V` 5 for it, so
under the 2020 text every AES-256 file's crypt filters are non-conforming — eleven of
`doc/pdf.js`'s twenty-five encrypted documents. `crypt::crypt_filters` has read `/CF` at `/V` 4 or
greater since it was written, on no stated authority at all; it now has the clause's.

**Issue #184 retires an ambiguity a comment still claimed.** Table 25's `/Length` row already said
"The standard security handler expresses the Length entry in bytes (e.g., 32 means a length of 256
bits) and public-key security handlers express it as is", and then closed with two unit-less
sentences — 128 for `AESV2`, 256 for `AESV3` — that read as denying it. Two carets append "for
public-key security handlers, and 16 for the standard security handler" and "…and 32…". So the byte
reading is now stated twice. `crypt::key_length` had resolved the conflict by Table 25's own
*range* — under 40 can only be bytes, 40 or over can only be bits — which agrees with the erratum
on every value either sentence can take, so no arithmetic moves. What moves is the comment above
it, which called the entry "famously ambiguous": that is a claim about the specification, and
`CLAUDE.md` says a claim about the specification decays.

## 4. `examples/encryption_census`, and why the rule it obeys wants a *shared* reading

`doc/todo/01`'s rule since the six-hundred-and-forty-first is that a note stating a count over the
corpus names the command that produces it. Three of these rows stated one and none did; the two
that could be checked against each other disagreed, and the third disagreed with a gate's ratchet.
The census is that command, and two things about its construction are the decision rather than the
code:

- **It asks `pdf_model::restriction::withheld` what Table 22 withholds**, rather than reading the
  flag word itself. Table 22's positions mean different things at different revisions — bit 9 is a
  grant from revision 3 and a reserved bit that "must be 1" below it — and a census that
  reimplemented that would be measuring its own second reading of the table rather than the tree's.
  That is why it lives in `pdf-model` beside the other censuses and not in `pdf-syntax` beside the
  clause.
- **Its population is an argument, not a constant.** It takes paths, so `doc/pdf.js` and
  `doc/corpora` can be asked separately — and asking the second is what found the row's own
  boundary: §7.6.4.1's "eight corpus documents" are eight of `doc/pdf.js`, which is all the corpus
  gate walks, and `doc/corpora/format-corpus/pdfCabinetOfHorrors/encryption_openpassword.pdf` is a
  ninth password-protected document no gate opens. Nothing is owed for it. What the row now says is
  which population its eight is a count of, which is ADR 0493's "an instrument has a population
  too" arriving from the other direction: there the census was narrower than the sentence, here the
  sentence was narrower than the word "corpus".

## 5. What this round does not claim

No status moved and no defect in the *code* was found. The four algorithms, the crypt filter
resolution and the permission reading all did what their rows said; every finding is in the prose
around them. That is the ordinary outcome for a family this well worked, and it is worth writing
down because the previous eight bands each moved something — a round that finds only prose defects
in a family has still made the next reading of it cheaper, and the alternative reading of a clean
code result is that nobody looked.
