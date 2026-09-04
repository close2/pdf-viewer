# 0900 — A modal verb as a discriminator, and what 214 rows said back

Session 930. Status: **accepted**. The instrument ADR 0896 asked for, built, calibrated against
that session's own four rows, and run over the whole `partial` population.

## Context

ADR 0896 named a shape no sweep in `doc/todo/01` can print: **a `partial` row whose stated debt
is a permission.** `ledger.toml` defines `partial` as "some [normative requirements] are
[executed]; the note says which are not", so a row `partial` for an entry the standard offers with
*may* or *can* says a requirement is unexecuted where the standard states none. Session 928 found
four instances in one band of five rows and could not say whether that was a band's accident or a
property of the ledger. The reason no sweep reaches it is structural rather than an omission:
**every sweep in `doc/todo/01` reads a row that owes something and asks whether the owed thing
exists in the tree**, and this is a row that owes nothing and says so. The discriminator is on
neither side of that comparison. It is the clause's own modal verb.

ADR 0896 offered a grep over the note — a debt sentence containing *may*, *can* or *is permitted*.
That reads the row's own prose, and `doc/ledger-and-claims.md`'s standing rule is that **a row's
note is not evidence.**

## Decision

`cargo run --release -p conformance --bin permitted`, the twenty-fourth sweep and the eighteenth
to be a program. It reads the note only to find out **what the row points at**, and takes every
verb off the standard.

### Two halves, because neither finds what the other does

- **The sentence half.** Every quotation of a note ([`quote::quoted_spans`]) is located in
  `doc/md/` and the standard's own sentence holding it is recovered
  ([`prose::Conversion::sentence_holding`], new here); the verb governing that sentence — `shall`
  / `should` / a permission / none — is read off it word by word, never by substring, because
  *cannot* holds *can* and *shallow* holds *shall*. A row whose strongest quoted verb is not a
  requirement is a finding.
- **The entry half.** A row's debt is very often an **entry** rather than a sentence, named as a
  key and attributed to a table with nothing quoted anywhere near it. So every `(table, key)` a
  note attributes — [`tables::attributions_in`]'s rule, the ninth sweep's, so that a key merely
  sharing a sentence with a table number is not a claim about it — is looked up in the table's own
  row (`entries::descriptions_in`, new here), and a row **every one of whose named entries the
  standard states as optional** is a finding.

Beside every hit the sweep prints ADR 0897's column: **how many `shall` sentences the clause's own
prose states outside its tables and NOTEs**, which is that ADR's suggested instrument. It is also
the tie-break within a rank, fewest first, because it separates the two shapes session 928 found
before the reading starts — a hit over a clause stating none is a status with nothing under it
(§14.9.2.2), and a hit over a clause stating several is a row that has read the wrong half of it
(§14.7.4.2).

### What the calibration cost the design, which is the part worth keeping

Trap 13, run against session 928's four rows restored to the ledger as they stood.

The entry half's rank began as *stated optional **and** described with no `shall`*, and under it
**three of the four are silent**. Table 122's `/Lang` reads "( Optional; PDF 1.5 ) A name
specifying the language of the font, which may be used … **The value shall be a Language-Tag as
defined in BCP 47.**" That `shall` constrains the **value**; a reader declining an optional entry
never reaches it, and §14.9.2.2's row implements the grammar it names and was `partial` for the
entry all the same. So the rank asks the table's own word and nothing else, and the verb governing
the description is *printed* beside it rather than gating it — because an optional entry whose
description does put a `shall` on a **processor** is a real debt, and that is a distinction a
reader makes and a program does not.

Calibrated at the loosened rule, with all four planted:

| row | where it lands | |
|---|---|---|
| §14.9.2.2 | rank 1, the entry half | Table 122's `/Lang` |
| §14.9.2 | rank 1, the entry half | the same entry one level up |
| §12.11.5 | rank 3, the sentence half | its only located quotation is a statement of fact |
| §14.7.4.2 | **not flagged** | and rightly: it stayed `partial` for a real `shall` |

Three of the four, and the fourth silent because it is not the defect. Restored, all four are gone
from the report.

## Consequences

- **The finding does not generalise the way a first reading of ADR 0896 suggests, and the numbers
  are what say so.** Over the population as this round found it — 214 `partial` rows, before its
  own two moves — **105 quote a requirement of the standard**; 109 do
  not, of which 49 name only entries the standard states as optional. That is a **reading list of
  half the population** and not a defect count — which is `doc/todo/01`'s own answer for every
  sweep whose ratio comes out this way (ADR 0249), and the reason this one exits zero.
- **A flag is a reading list and never a verdict**, and this round is its own witness in both
  directions. §8.11.1 is flagged and correct — its note names §8.11.4.5's "shall be reapplied"
  beside the `/Configs` permission, and a three-word quotation is below the sweep's own minimum.
  §12.7.7 is **not** flagged and was wrong, because it quotes the four `shall`s it *implements*
  while its debt is a *can* the sweep never sees.
- **The noise shape that bounds it, stated as a fact rather than as a caveat**: a note quotes the
  standard for the half of the clause it executes at least as often as for the half it owes, and
  nothing mechanical separates those two without deciding what an English sentence means. That is
  the same wall `--bin owed` hit (ADR 0397), reached from the other side.
- The reading it produced, and where each row turned, is ADR 0901.
