# The conformance ledger, and where a false claim can hide

Status: **standing** — `tools/state.sh` prints the counts; the failure modes live here.
Read by: whoever is about to write or correct a ledger row, or to believe one.

`doc/PLAN.md` §5a owns the ledger's format, statuses and checker; `doc/todo/01-ledger-partial-rows.md`
owns the sweeps — fifteen now, six of them committed programs — and what each run found — the eleventh is new in the
four-hundred-and-thirteenth and is the first that reads a row's *quotation marks* (ADR 0249), and
the twelfth lives in `tools/spec-errata` and asks the same spans whether an erratum struck one
(ADR 0254). The counts below were read off `cargo run -p conformance --bin ledger` in the
four-hundred-and-forty-second, which read the `partial` rows nobody had re-read — ordered by when
each row's *note* was last written, which `git blame` answers and no sweep here had asked — and
moved four to `implemented`. The four-hundred-and-thirty-seventh's fourteenth sweep — a `partial`
row whose note names nothing owed — had moved three on its first run (ADR 0273). What is here is the part that is about *claims* —
the seven shapes a row goes wrong in, and the four times this project's own ledger has been wrong.
It was `doc/HANDOVER.md`'s "The ledger" and its section 2 until the three-hundred-and-ninety-fifth
moved it.

§14.7.3 and §14.9.1 moved to `implemented` in the three-hundred-and-seventy-sixth, both because the
consumer they were waiting for arrived and, in §14.7.3's case, because the *query* that had read
past its role map stopped doing so (ADR 0214).

All **823** subclauses of the eight technical clauses have been read against this code, since the
fifty-sixth session — **and, since the three-hundred-and-sixtieth, the 52 numbers of the standard's
eight normative annexes**, which no instrument in this project could previously name: `ClauseNumber`
was a list of integers, so `§K.2` was a malformed citation and Annex O could not have a row. ADR
0206. Counts come from `cargo run -p conformance --bin ledger`, which prints them
— **not** from arithmetic in this file, which has been wrong about them twice.

| status | rows | |
|---|---|---|
| `implemented` | 410 | every normative requirement in the clause is executed |
| `partial` | 244 | some are; the note says which are not |
| **`silent`** | **0** | not implemented, and nothing says so — **Annex O's five were the last, and they were built in the three-hundred-and-sixty-ninth** |
| `inapplicable` | 82 | a press, a layout engine, a production workflow — **and read at last**. §10.4.2.3 left in the three-hundred-and-eightieth, where a conversion this row called "[not] on any route to a pixel" turned out to be §11.5.3's own (ADR 0217), and **§10.4.2.4 left in the three-hundred-and-eighty-third on the same reasoning one clause over**: it said the two functions "cannot change a pixel" while §11.6.6 converts an RGB colour into a `DeviceCMYK` mask group by exactly this route, where every term the clause generates provably cancels (ADR 0220). Session 380 corrected two §10.4.2 rows and asked whether the neighbours had the same disease; one did |
| `out-of-scope` | 113 | principle 5's closed exclusions, which the row names |
| `reported` | 18 | not implemented, detected and named at runtime — **nine of §12.8.3's moved to `partial` in the three-hundred-and-seventy-seventh**, when the digest question turned out not to need the trust store the whole clause had been refused for (ADR 0215) |
| `writer-side` | 8 | addresses a PDF *generator* |

**`silent` is zero, and Annex O's five were the last of them.** The three-hundred-and-sixtieth
session gave the ledger the standard's **normative annexes** — D, E, F, I, K, L, O and Q, 52 rows —
and the reason they had none is that the instrument could not spell their numbers: `ClauseNumber`
was a `Vec<u16>` and its own test asserted `"A.1".parse()` fails, so a citation to `§K.2` was
malformed, a quotation from Annex Q was uncheckable and a row for Annex O was unwritable. **Annex O
is eleven `shall`s on "the PDF processor"** — `page`, `nameddest`, `zoom`, `view`, `highlight`,
`search` and five more — saying what a document shows when it is opened through a URI. **A document
cannot contain a fragment identifier**, so the corpus and the oracle were blind to this by
construction: coverage found what robustness cannot see. **Built nine sessions later**:
`pdf_model::fragment` reads all eleven, `viewer_core::Open::apply_fragment` carries out seven, four
are reported by name with a different blocker each, and `pdf-viewer doc.pdf#page=5` is the first
caller. Three things in the annex's own text came out of it — it prints `(28h)` for the AMPERSAND
its own Table D.2 gives 0x26, it never states the `=` that joins a parameter to its arguments, and
its coordinate rule is true only when the *units* are default user space's and the *origin* is the
page's top-left corner. `doc/todo/39`, ADRs 0206 and 0209. **Annex I.2 came out of the same read and was paid the round after**: the file's version
number was located to fix the byte offsets and its digits thrown away, against a `should` that says
to warn the reader. `Document::version` reads §7.5.2's header and Table 29's `/Version` and ranks
them the way §7.7.2 does, and `notes::about` says a file is newer than the 2.0 this program
implements. No corpus document reaches it — 354 of the 974 state 1.7 and nine state 2.0 — so it is a
requirement the corpus could never have ranked (ADR 0207).

**Before Annex O arrived it was zero, and before that it had been one for exactly one round.** §10.5's transfer function:
`issue6931_reduced.pdf` states an `/ExtGState` `/TR` whose type-0 tables map 2/255 to 0.992, its
image's every sample *is* 2, and the page's own text says *The color should be red*. Ours and
`mupdf` showed a black square where three references showed a red heart. It came off
`doc/todo/00`'s **step 7** at +17.26, which is the instrument built for a page nobody is far
from. **What took the round was the argument**: implementing it crossed `CLAUDE.md`'s own scope
sentence, and the standard settled it — ISO 32000-2 never uses the phrase *marking device*,
§10.1's list of rendering steps makes halftoning conditional on the device and the transfer
function not, and §10.6.1 keeps the transfer for a device that needs no halftone. The project
owner split the scope line rather than dropping it, and the clause is implemented (ADR 0204).
There is no requirement in the standard — the eight technical clauses or the
eight normative annexes — that this program
fails without saying so. That is a narrow claim: `partial` and `reported` are 262 rows between
them and each names what it owes.

**A seventh way was found in the three-hundred-and-fifty-ninth, in the population no sweep had
ever read: the `inapplicable` rows.** Every sweep in `doc/todo/01` walks the rows that *owe*
something, which is the property that let five wrong reasons sit undisturbed. §14.11.3's printer's
marks and §14.11.6.2's trap networks were `inapplicable` because "a screen is not a printer" —
while `PrinterMark` and `TrapNet` have been in `annotation.rs`'s `STANDARD_SUBTYPES` from the
start, and **§12.5.6.20 and §12.5.6.21 said so in their own notes**. Both clauses settle it in one
sentence each uses verbatim: the flags "shall be set and **all others clear**", which leaves
`NoView` clear. §14.12.4 said Table 409 was unread while `document_part.rs` reads it, and its own
parent row said the opposite. **Shape 7 is two rows about one mechanism, disagreeing** — cousins
rather than parent and child, which is why the arithmetic sweep cannot see them — and the tell is
that one row gives a *capability* reason where the other names *code*. ADR 0205.

**And a sixth was found in the two-hundred-and-sixteenth, by a sweep that is one `grep`:** a
sentence a session *retired* in one row, still standing in the other row that describes the same
mechanism. §11.6.4.3 recorded in the two-hundred-and-first that the graphics state's soft mask is
applied and had been since the eighteenth session; §8.9.6.1 went on saying it was "reported rather
than applied on 28 corpus documents" for fourteen more. **A correction is a string, and the string
is greppable** — `doc/todo/01`'s fourth sweep.

**A sixth way was found in the two-hundred-and-ninety-eighth, and it is the inverse of the first:
the note was corrected and the *status* was not.** §7.6.3 was `partial` above a note opening "both
algorithms are implemented in both directions"; §9.10 was `partial` above one saying all three of
§9.10.2's methods had been implemented a hundred and forty-two sessions earlier; §14.3 was
`partial` four rounds after the last of its four children closed. No grep in `doc/todo/01` can see
this, because the half a sweep reads is the half that is right. **The instrument is arithmetic**:
print every row that owes more than all of its own children, which is twenty lines over
`ledger.toml` and nothing else, and is `doc/todo/01`'s sixth sweep.

**And a fifth way for a row to be wrong was found in the hundred-and-seventieth to
-seventy-fourth**: not overstating, not understating, but *stale about its neighbour*. §7.7.2
listed eighteen catalog entries as unread that were read, most of them by the session that built
their clause; §12.6.3 said "this crate has no events" for forty-one sessions after
`Command::Pointer` landed; §14.3.3 was `inapplicable` because "this one has no panel" for seven
after one was drawn. **A family's parent row is not maintained by the sessions that implement its
members, because the clauses do not cite each other**, and neither is a row whose blocker was a
capability rather than a clause.

**This file's arithmetic was wrong about the oracle too, and session 154 corrected it by reading
the output.** The row above said "11 not comparable", which is 1665 minus the other three buckets;
the gate prints **seven** buckets and the missing two are `our geometry` (0 complete) and
`reference geometry` (2). Nothing had changed — the number had been derived rather than read, which
is the thing the paragraph below says not to do.

**The ledger has been wrong four times and this file's arithmetic about it once.** A row that
names a rasteriser's behaviour has recorded that rasteriser (§8.4.3.2); a row written during a
review describes what the code *should* do (ADRs 0056, 0057, 0060). The defences are reading the
*family* rather than the row, and `FILE_ONLY_EVIDENCE_CEILING`, which is zero and asserted with
`==`.

**`writer-side`'s 7 rows were re-read in the hundred-and-thirty-seventh session** against the
amended definition — `CLAUDE.md` excludes *authoring*, not writing — and `ledger.toml`'s header
now carries that definition rather than "we do not create files". **That last sentence was false
from some regeneration after it was written until the five-hundred-and-tenth session**: the
header is *generated*, its vocabulary lives in `tools/conformance`'s `PREAMBLE` and `Status`
docs, nobody amended the generator, and the next `--bin ledger` run stamped the retired sentence
back over the corrected file. A correction to generated text is not a correction until it
reaches the template (ADR 0345). **Six stay and one moved**:
§7.2.2's "Representation" binds this tree now that it writes, and all three of its requirements
are met by construction. ADR 0122.

## What belongs to this file's own claims rather than to the ledger's

**The reading task itself is [todo 01](todo/01-ledger-partial-rows.md)** — the three sweeps, the
five shapes a stale note takes, and what the last run found. What belongs here is the part that
is about *this file's* claims rather than about the ledger's.

- **Keep `REVIEW_OWED` empty.** A clause the code cites and nobody has read is the cheapest debt
  this project can accrue, and the list fails the build the moment one appears.
- **`FILE_ONLY_EVIDENCE_CEILING` is zero, asserted with `==`.** 58 → 0 over four sessions of
  auditing (ADRs 0098, 0100, 0101, 0102), **every one of which found a false or unheld claim**.
  It does *not* say the right test was named: three of the four false claims it hid were caught
  by the oracle rather than by a row.
- **A gate cannot see a cache.** ADR 0115's defect drew wrong glyphs on two documents in silence
  for thirty-one sessions: no report, no contradicted page, and one of them sat on the text gate's
  "undiagnosed" list at 83%. **Where a lookup is memoised, ask what the key claims.** Every cache
  in the tree keys on object identity, checked one by one in session 128.
- **A silence is not a gap**, and the first move on one is neither a report nor a feature: work
  out what the clause asks *of this device*. §10.7.5's `/SA` was implemented in the half a display
  can state and recorded as a departure in the half it cannot; §11.7.4's overprinting was six rows
  a reading of Table 146 removed altogether.
