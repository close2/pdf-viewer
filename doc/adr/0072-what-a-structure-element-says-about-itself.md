# ADR 0072 — What a structure element says about itself

Status: accepted, 2026-07-31.

## Context

The seventy-eighth session read §14.7.2's structure tree downwards. What it left were the two
things hanging off an element that are not its children: §14.7.6's **attributes** and PDF 2.0's
**namespaces**, eleven `silent` rows between them.

Both look optional from a distance and are not.

- **Attributes cannot be skipped by a reader that reads the tree at all.** §14.7.6.3's revision
  numbers are stored *inside* the same array as the attribute objects — "a single or a pair of
  array elements, the first or only element shall contain the attribute object itself and the
  second (when present) shall contain the integer revision number" — so a reader that does not
  know about revisions takes an integer for an attribute object. The mechanism is deprecated in
  PDF 2.0 and reading it is what makes the array parseable.
- **Namespaces change which role map applies.** §14.7.3 says a document's own type names are
  mapped by the structure tree root's `/RoleMap`; §14.8.6.2 says that is only true of an element
  that states no namespace, and that one which does is mapped by *that namespace's* `/RoleMapNS`.
  `Tree::role` had the first rule and would have answered the root's mapping for an element the
  clause says the root does not govern.

## Decision

**Read both, and put the precedence rules where the clause states them.**

`Tree::attributes` returns every attribute object attached to an element in increasing
precedence order — §14.7.6.2's classes first, then the element's own `/A` — because the clause
states two rules that compose into exactly that order: later in `/A` wins over earlier, and `/A`
wins over `/C`. `Tree::attribute` then takes the last match, so neither rule is written twice.

`Tree::role` carries a name **and** a namespace through its walk, follows a `/RoleMapNS` entry in
both of Table 356's forms — a name, or a name paired with a target namespace dictionary — and
stays bounded, because "applied transitively" is an invitation to a cycle.

§14.7.6.4's user properties are read as they are written: the value stays the PDF object the file
states, with `/F`'s formatted form beside it rather than replacing it. The clause is explicit that
a processor "need not display values of other types; however, they should not treat other values
as errors", which is a decision for whatever displays them and not for the reader.

## The other half of this session: six rows that claimed too little

While updating the family, six rows in it were found **understating** what the code does. §14.7
still said "none of it is read" two sessions after the tree was built; §14.7.5, §14.7.5.1,
§14.7.5.2 and §14.7.5.3 were `silent` beside a `structure::Child` that implements all three
content-item forms; §14.7.1's mechanism had been complete since the parent tree landed.

This is the failure mode the handover already names — "a stale row can understate as well as
overstate, and only the overstatements have a gate" — met a second time, and it is worth the
sentence: the ledger's numbers are a *lower* bound on what exists unless somebody reads the
family. Eighteen rows left `silent` this session and only eleven of them were written today.

## And a gate for the ledger's own prose

The ledger is 823 notes about ISO 32000-2, and **nothing checked a word of them**: the citation
gate reads Rust sources and the ledger is TOML. Adding `citation::scan_prose` and running it over
the notes found, on its first run:

- three wrong table numbers, each ISO 32000-1's number for a table ISO 32000-2 renumbered — the
  namespace dictionary as Table 358 (it is 356), the attribute object dictionary as Table 363 (it
  is 360), an object reference's `/Obj` as Table 362 (it is 358);
- two clause numbers that name nothing: a `§11.2.2` for a sentence that is in §11.2 itself, and a
  `§12.7.3.2` for §12.7.4.2's field names.

All five read as correct writing, which is the same shape as the `§9.3.6 Table 106` the thirteenth
session found in the code. The notes now name 832 clauses and 203 distinct tables, all of which
exist.

## Consequences

- `silent` falls 171 → 153, the largest single-session fall since the ledger reached zero
  `unreviewed` rows — and eleven of those eighteen are new code, seven are corrections.
- Nothing renders differently. §14.1 says the clause's features "do not affect the final
  appearance of a document", and the three gates confirm it: no page moved.
- What clause 14 now owes is unchanged in kind and smaller in size: a **consumer**, and §14.8's
  vocabulary — what a `/Table` or a `/BackgroundColor` *means*, as against which element states
  one.
