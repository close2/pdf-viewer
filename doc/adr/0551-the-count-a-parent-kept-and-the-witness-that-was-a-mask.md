# ADR 0551 — The count a parent kept, the witness that was a mask, and the caret that named a resource dictionary

Status: accepted, 2026-08-23. Session the six-hundred-and-ninety-seventh, a clause round under
`doc/todo/01`, taking the `partial` rows of one family rather than a band across families — the
method ADR 0538 argued for one block earlier. Amends §11.4, §11.4.1, §11.6 and §11.6.6 in the
ledger; adds one paragraph to `crates/pdf-model/src/content/transparency.rs`'s `named_press`; adds
one section to `doc/errata-read.md`. Extends ADRs 0237, 0262, 0276, 0327 and 0492; changes nothing
any of them decided, moves no status, and moves no pixel.

## 1. The family, and why it was the one to read

The blame ordering over `doc/conformance/ledger.toml`'s `note =` lines puts §11.3.4 at rank 2 and
§11.3.7, §11.4.1 and §12.5 in the cluster at ranks 3–10. Three of the top ten are clause 11's, and
clause 11 has three families with several `partial` rows apiece — §11.3 five, §11.4 seven, §11.6
five. §11.4 was taken because it is the one whose rows *cross-refer by name*: §11.4's aggregate
counts §11.4.4's reports, §11.4.1 defers the colour-space question to §11.6.6, §11.4.5 answers it,
§11.4.7 and §11.6.6 share a construction and a population. A family whose rows quote each other's
figures is a family where a stale figure has somewhere to disagree with itself, and all three
findings below are exactly that.

The corollary is worth stating because it is the second time in three rounds: **a row that is wrong
about its siblings cannot be found by any ordering that reads one row at a time.** ADR 0538 said it
of §7.6; this is the replication, in an unrelated clause, with a different kind of claim.

## 2. What was wrong

| row | shape | was | is |
|---|---|---|---|
| **§11.4.1** | a `partial` reason its own siblings deny | "a *painted* group that introduces one is reported rather than composited in it" | false since ADR 0327: an isolated group whose `/CS` names four components this tree can sample composites in them, as §11.4.7's pair one scope down. §11.4.5's row and §11.6.6's have both said so for two hundred sessions |
| **§11.4** | a parent's count of a child's population | "6 documents reported §11.4.4 … and 3 do now" | the child's own row says one, and the corpus gate says none. Over `doc/pdf.js` **no document reports a transparency-group departure of any kind** |
| **§11.6.6** | a count and a named witness, both stale, in the paragraph that *narrowed* the row | "1 corpus document … `bug1721218_reduced.pdf` being the corpus's", and its "inner gray-`ICCBased` groups are the corpus witness, so that document keeps a smaller report" | the corpus half is 0. That document reports nothing, and every one of its eight groups whose `/CS` is the one-component `ICCBased` space is the `/G` of an `/SMask` dictionary of subtype `/Luminosity` — §11.5.3's population, not this clause's |
| **§11.6** | the same count, written twice | "reported for 1 corpus document and 8 of 65 944 web ones (§11.6.6)" | defers to §11.6.6 for the populations. ADR 0101's shape, and the copy was the one that could not be corrected by fixing the original |

### The third of those is the one worth arguing about

§11.6.6's row contains, four paragraphs apart, the correction and the relapse. The
four-hundred-and-fortieth session measured that **most of what was reported for this clause was not
this clause at all** — `build_soft_mask` clears the space in force for a mask group's content,
because §11.5.3 reduces such a group to one luminosity, and the flag recording a *change* of space
was outside that scope (ADR 0276). Fifty sessions later the four-hundred-and-ninety-second narrowed
the row for ADR 0327's construction and named `bug1721218_reduced.pdf`'s "inner gray-`ICCBased`
groups" as what the document still had left. They are mask groups. Every one of the eight is the
`/G` of an `/SMask` `/Luminosity` dictionary, which is precisely the population the paragraph above
had removed.

**So a corrected row is not a safe row**, and that is the general lesson rather than the instance: a
round narrowing a row reads the sentence it is replacing, and a round that also read the paragraphs
*around* it would have had the answer in the same note. Nothing in `doc/todo/01`'s eighteen sweeps
can see this — the contradiction is between two paragraphs of one note, and every instrument here
compares a row with the tree, a row with its children, or a row with the standard.

## 3. What the measurements were

Three commands, each printed rather than recalled.

- `cargo test --profile gates -p pdf-model --test corpus -- --ignored` — the incomplete list, which
  is where the report count comes from. Not one of its entries is an `Unsupported::TransparencyGroup`.
- `cargo run --release -p pdf-model --example open_one -- doc/pdf.js/test/pdfs/bug1721218_reduced.pdf`
  — `unsupported []`, one top-level command.
- `cargo run --release -p pdf-model --example group_space_census -- doc/pdf.js/test/pdfs/*.pdf` —
  964 documents opened, **7 page groups state a `/CS` that is not a three-component RGB one** (which
  is §11.4.7's own figure, unchanged) and **1 painted group introduces one**, which is
  `bug1721218_reduced.pdf`'s `/DeviceCMYK` outer group and composites in ink.

**A count of zero reports needed a second question, and it is trap 13's.** A report that fires on
nothing is indistinguishable from a report that has stopped firing, so the discriminator is whether
anything still exercises it: `crates/pdf-model/tests/transparency_groups.rs` holds each of this
family's departures on a synthetic fixture and asserts both directions — the report present where
the clause's condition holds, absent where it does not. Those pass. Zero is therefore a fact about
`doc/pdf.js` and not about the reports, and §11.4's row now says so in those words, because the next
round to read it will ask the same question.

The web figure — 8 of 65 944 — is the four-hundred-and-fortieth's and is **not** re-derived here.
Re-deriving it means interpreting page one of the whole crawl, which is not a minutes-long run, and
the honest thing is to leave it attributed rather than to restate it as current.

## 4. The errata, read for §11.4 and §11.6 before anything was written

`cargo run --release -p spec-errata -- emit doc/*.pdf` files three annotations across the two pages
§11.6.6 spans, and all three are a **`Caret` with no `StrikeOut`** — the shape `check` cannot see by
construction, which `doc/errata-read.md` states as a rule and which has now paid on six consecutive
rounds.

**Issue #134, `Review/Completed`, is the finding, and it is #74's shape from ADR 0538.** It inserts
`of the transparency group XObject` into Table 145's `/CS` row, after "the ColorSpace subdictionary
of the current resource dictionary". The word doing the work is *current*: a group's `/CS` is read
at the `Do`, where the resource dictionary in force is the **parent's**, since the group's content
stream has not begun — so the published sentence admits two readings and they select different
`/DefaultCMYK` entries whenever a form states one its page does not. This tree reads the group's
own, because `content/xobject.rs` resolves `form_resources` from the form's `/Resources` and hands
that to `press_for_entry` and `named_press`. It had no stated authority for that and now has the
clause's. `named_press`'s comment says so, and §11.6.6's row records it.

**Issue #619's two carets are not §11.6.6's**, and that is a property of the instrument worth
having. `Landing::section` files an annotation by the *page* the outline puts in a clause, so
everything on page 436 is filed under §11.6.6 — whose heading is at the bottom of that page, above
nothing but its own first paragraph. The two carets are at the top, on Table 143's `/ID` and `/OPI`
rows, which are §11.6.5.2's. ADR 0492 read this family's errata in the six-hundred-and-sixty-sixth
session and recorded them as marking "entries of §11.6.6"; counting in the same unit that ADR used
— a change rather than an annotation — it found four across clause 11 and there are five, Issue
#134 being the one that was not seen. The arithmetic that
separates them is a minute's work and is written down in `doc/errata-read.md`: a `/Rect` measures
from the bottom of an 841.92-tall page and `mutool draw -F stext` measures from the top.

## 5. What this decides

Nothing about the code. Four rows say what is true; one function's comment cites the clause it has
been obeying; one document records two errata and one filing shape. The status of every row
involved is unchanged, and §11.4.1 stays `partial` for a remainder that is real and is enumerated
one row over.

## 6. What it leaves

§11.6.6's three remaining reported shapes — a three-component or one-component space inside a
four-component parent, four components no profile backs, a stated black generation — are unwitnessed
on `doc/pdf.js` now that its one candidate turned out to be a mask group. That is a corpus fact and
not a reason to demote them: `CLAUDE.md`'s two denominators say a count over a corpus cannot rank a
requirement no document exercises. What it does change is where a witness would have to come from,
and the answer is the crawl rather than `doc/pdf.js`.
