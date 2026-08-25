# ADR 0611 — A claim written outside its sweep's syntax

Status: accepted, 2026-08-25. Session the seven-hundred-and-twenty-fifth, beside ADR 0610 and found
by the same reading. Corrects two table numbers in `doc/errata-read.md` and one prose pointer in
`crates/pdf-model/tests/oracle.rs`; marks one quotation in ADR 0496; adds the shape to
`doc/todo/01`'s catalogue for the ninth sweep. **No status moves, no pixel moves, no report.**
Extends ADRs 0372, 0380 and 0475.

## 1. The three instances, all in one clause family

Reading §10.7.4 ~ §10.7.5 turned up three false or dead claims about the *same* family, each
invisible to the instrument built for exactly its kind of claim:

| the claim | the instrument for it | why it could not see it |
|---|---|---|
| `doc/errata-read.md`: "Table 58's `/FL` loses the 0-to-100 range" | ninth sweep, `--bin tables` | Table 58 is the path construction operators, whose columns are *Operands* and *Operator*. The sweep classes a table that states no entries as **keyless** and counts it rather than printing it — deliberately, so that a flags table or a displaced column does not print every round. So a wrong number is invisible precisely when it lands on a table with no entries |
| `doc/errata-read.md`, three rows down: "moved here from Table 58" | the same | no key stands beside the number, so it is not an *attribution* at all. The sweep's whole discriminator is that a key counts only where the sentence attributes it — which is what keeps it from printing 545 hits — and a number written on its own is below that floor |
| `oracle.rs`: "The handover's list of departures carries the argument" | eighth sweep, `--bin pointers` | a pointer written as prose rather than as a path. `doc/HANDOVER.md` has held no list of departures since it became an index; the list is `doc/todo/_scan-conversion.md` and §10.7.4's ledger row. Nothing resolves an English noun phrase |

The first two are the same wrong number as the one §10.7.5's own ledger row records having carried
"for the whole of its life" until the three-hundred-and-eighty-ninth session, when the ninth sweep
found it *there* — because there it was written as `Table 58's /SA`, and **Table 57 states entries,
so that citation was an absence rather than a keyless count**. One number, two documents, one sweep,
and the sweep printed it in the document where the wrong table happened to have entries.

## 2. Calibrated against the defect, per trap 13

One instrument over three states of the same table cell in `doc/errata-read.md`:

The cell attributes `/FL` possessively to one number, and only the number changes between rungs:

| the number in the cell | the sweep does |
|---|---|
| 58, the path construction operators | prints nothing; the citation lands in the keyless count |
| 166, a table that does state entries | prints the citation as an absence, **and names Table 57 as the one that states the key** |
| 57, the graphics state parameter dictionary | agrees |

So the sweep would have found this the moment the wrong number were any of the 305 tables that state
entries, and the correction it offers is the right one. The blindness is exactly one class wide.

## 3. The rule, which is more general than the three

**Every sweep has a syntax, and a claim written outside it is invisible to the sweep built for it.**
That is not a defect in any of these programs — each of the three exclusions above is *why* its
sweep is usable at all, and `doc/todo/01` prices two of them explicitly. What follows is a rule for
writing rather than for building:

> Write a claim in the form its sweep reads. A table number gets its key beside it; a pointer gets
> its path; a debt gets its identifier.

The alternative — widening a sweep until it reads English — is what ADR 0249's ratio argument
forbids, and the five-hundred-and-thirty-seventh session already measured the cost of the widest
version of the ninth sweep at 545 hits to nothing.

## 4. Not built

**No sweep is changed and none is added.** Making `--bin tables` print keyless attributions would
turn the same dozen conversion artefacts into a permanent reading list, which is the shape the
module comment says would get it switched off. What is added is a line in `doc/todo/01`'s catalogue
saying that the keyless count is a *hiding place* as well as a noise filter, so that a round reading
a table citation knows to check the number by hand when the cited table states operators rather than
entries.
