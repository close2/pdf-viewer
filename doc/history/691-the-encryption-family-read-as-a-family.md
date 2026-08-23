# 691 — The encryption family read as a family, and two carets nobody had read

The blame list's top sixteen held four of §7.6's rows, so the round read the *family* instead of
the band — and **three of its four findings are a disagreement between two rows of that family**, a
shape no ordering by age can produce. Two errata went with them, both a bare `Caret` under §7.6.6,
one of which licenses what this reader has always done at `/V` 5.

Date: 2026-08-23.
ADR: [0538](../adr/0538-the-encryption-familys-counts-and-two-carets-nobody-had-read.md).

Touched: `crates/pdf-model/examples/encryption_census.rs` (new),
`crates/pdf-syntax/src/crypt.rs` (two doc comments),
`doc/conformance/ledger.toml` (§7.6, §7.6.4, §7.6.4.1, §7.6.4.2, §7.6.4.4.2, §7.6.6),
`doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`, the ADR and this file.

## The band, re-derived

`git blame --line-porcelain doc/conformance/ledger.toml`, each `partial` or `reported` row's own
`note = ` line, ranked by where its commit falls in `git log --reverse`. This base has **916
commits** and **242** `partial`-or-`reported` rows with a blamed note, and the six-hundred-and-
seventy-first's prediction came out exactly for the fifth band running: §7.6.4 and §7.6.4.4 at ranks
1–2, §11.3.4 at 3, the cluster of nine at 4–12, and §14.6, §14.6.1, §7.6 and §7.7 at 13–16.

**Four of those sixteen are §7.6's**, which is the observation the round is built on. `doc/todo/01`
has two rules for choosing within a band — ADR 0455's (prefer a reason that is a claim about this
codebase) and ADR 0460's (prefer a claim about the standard that names where it looked) — and both
rank a row against something outside the family. Neither can find the fifth failure shape, which is
*defined* as a row disagreeing with its siblings. Reading §7.6, §7.6.4, §7.6.4.4 and §7.6.6 beside
each other found it three times in one sitting.

## The four defects

- **§7.6's arithmetic accounted for every encrypted document as opening.** "19 of the corpus's 26
  encrypted documents open with the default user password and the other 7 with theirs" — 19 + 7 =
  26, and two of them are refused by name, which §7.6.4.2's own row says and `corpus.rs`'s
  `MAX_UNREADABLE_ENCRYPTION = 2` ratchets. The answer stood in two other places in this tree while
  this row denied it.
- **§7.6.4.2's four figures were 26, 19, 4 and 6.** Measured over `doc/pdf.js`: 25, 23, 3 and 6.
  Only the last survives.
- **§7.6.4 was `partial` on a reason its own child records as not a debt.** The note said "revision
  5 is refused by name" and nothing else about what is owed; §7.6.4.2 is `implemented` carrying that
  exact refusal, because Table 21 states no algorithm for `/R` 5. The row now names the three
  `partial` rows below it, which is what the status is actually resting on.
- **§7.6.4.4.2 said "the one whose known password is the owner's".** Three of the eight
  password-protected documents authenticate as the owner, and `encryption.rs::an_empty_password_may
  _be_the_owner_password` has asserted one of them since it was written.

## The two errata

Both are a **`Caret` with no `StrikeOut`**, which `spec-errata check` cannot see by construction —
its discriminator is a quotation matching *struck* text, and an erratum that only adds has struck
nothing. `emit` found both.

**Issue #74** inserts "or 5" into §7.6.6's "the value of the V entry shall be 4 to use crypt
filters". Under the 2020 text every AES-256 file's crypt filters are non-conforming, because
Table 25 gives `AESV3` no home but a `/CF` entry and Table 20 requires `/V` 5 for it — eleven of
`doc/pdf.js`'s twenty-five encrypted documents. `crypt::crypt_filters` has read `/CF` at `/V` 4 or
greater since it was written, on nobody's authority; it has the clause's now.

**Issue #184** appends "for public-key security handlers, and 16 for the standard security handler"
to Table 25's `AESV2` `/Length` sentence and "and 32…" to its `AESV3` one, so the byte reading the
table already stated higher up is now stated in the two sentences that had seemed to deny it.
`crypt::key_length` had disambiguated by the table's *range* instead — under 40 is bytes, 40 or over
is bits — which agrees with the erratum on every value either sentence can take. No arithmetic
moved; the comment calling the entry "famously ambiguous" did.

## The census, and the population a row's word for it hid

`crates/pdf-model/examples/encryption_census.rs` is the command `doc/todo/01`'s counted-claim rule
asks for. Two things about it are decisions:

It asks `pdf_model::restriction::withheld` what Table 22 withholds rather than reading the flag word
itself, because bit 9 is a grant from revision 3 and a reserved bit below it — a census with its own
reading of that would measure itself. That is why it is in `pdf-model` and not in `pdf-syntax`.

And its population is an argument. Asking it about `doc/corpora` rather than `doc/pdf.js` found
§7.6.4.1's boundary: its "eight corpus documents" are eight of `doc/pdf.js`, which is all the corpus
gate walks, and `format-corpus/pdfCabinetOfHorrors/encryption_openpassword.pdf` is a ninth
password-protected file no gate opens. Nothing is owed for it — a document that will not
authenticate is what the prompt is *for* — but a round re-deriving the eight over a wider corpus
would have found nine and had no way to know why.

## Gates and sweeps

The machine was heavily loaded when the round began — `load average: 49.70` on 24 cores — so the
gates were run after it fell to about 18, per `doc/todo/02` §2's rule that a gate spawning a
reference measures two programs and a loaded machine is a silent third. `PDFREF_CACHE` pointed at
the shared warm cache.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker,
corpus, `pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000,
quorra and `fixed_documents` all green; `cargo test -p conformance` green after the final edit.
The corpus gate's own output confirms the census line for line: eight locked, two refused.
§5's binaries rebuilt and installed.

Twelve sweeps run before the edit, after it, and **a third time on the tree with the ADR and this
file in it** — the six-hundred-and-seventy-first's lesson, that an ADR, a history file and
`doc/todo/01` are `SOURCE_ROOTS` too. Three levels are worth recording as *catches* rather than as
levels, and the third was only visible on the third run:

- `--bin counts` went from 4 places counting one family twice to **5**, on this round's own draft
  sentence — "the three that are owed are the three `partial` rows below" put a second cardinal for
  §7.6.4's family in a note whose first sentence counts twenty. Both numbers were true and the pair
  was still worth not writing; the sentence names the three rows instead and the level is back at 4.
- `--bin owed` gained 26 terms and **no** phantom key, because the citation added is
  `examples/encryption_census`, whose leading segment `encryption` is a word this tree's sources
  name in quantity. ADR 0493's shape costs a round a phantom only when the invented noun is
  invented.
- `--bin tables`' contradicted denials went 6 → **7** on the ADR's own summary table, and the
  sentence was "because Table 21 states no algorithm for `/R` 5". Table 21 *does* state `/R`; what
  the sentence denies is an **algorithm**, and the sweep reads the nearest table number beside the
  nearest key. The tell is that the identical sentence in this file does not trip it — a markdown
  table row is one "sentence" to the extractor, so the ADR's cell put "Table 21" and "`/R`" and
  "states no" inside one span where the prose does not. **That is a noise shape charged to prose
  written as a table**, which is worth knowing before the next round writes a findings grid. The
  ADR says "the standard states no algorithm for revision 5" now, which is §7.6.4.2's own existing
  wording, and the level is back at 6.

Everything else moved by exactly what the new prose contains and nothing landed in a defect
bucket. Final levels on the committed tree, after → before: `counts` 7037 ← 7005 sentences with
**four places counting one family twice both times**, `quotations` 1844 ← 1838 ledger spans with
verbatim 1409 ← 1407 and **diverging unchanged at 2**, and 5527 ← 5505 document spans; `tables` 5983
← 5961 sentences and 2261 ← 2250 key citations with absent unchanged at 100; `pointers` 7361 ← 7333
with absent unchanged at 123 and undefined symbols at 13; `owed` 3617 ← 3591 terms with 182 unnamed
over 114 rows unchanged; `overtaken` 507 ← 506 decision records with 40 overtaken unchanged;
`entries`, `overstated`, `unread`, `blockers`, `capabilities` and `inapplicable` all unmoved.
`spec-errata check` and `applied` print no hit at any line this round wrote.
