# ADR 0063 — What a page says, as against what it shows

Status: accepted, 2026-07-31.

## Context

`CLAUDE.md` puts "tagged PDF as far as accessibility needs it" in scope and names AccessKit as
this project's accessibility layer, and §14.9 is the clause that defines *how far*: it is the
one place in clause 14 that says which parts of §14.8's sixty rows a screen reader actually
needs. Before this session the whole family was `silent` except one entry — §14.9.4's
`/ActualText`, which landed in the fifty-fifth session because it changes what a page *extracts*
and the text-extraction gate could see it.

The rest of the family had been sized in its own ledger row: 95 of the 974 corpus documents
write a `/Lang`, 87 a `/StructTreeRoot`. The machinery each entry needs already existed —
§14.7.5.4's parent tree since the fifty-sixth session, the inline property list since the
fifty-fifth — so what was owed was the clause rather than a component. That is the shape the
handover called "one lookup away from where `/ActualText` already is", and it was right.

## Decision

**Read all four of §14.9's text entries, in both of the places each may sit, and keep the
two questions they answer apart.**

### The two questions

§14.9.4's NOTE 2 draws the distinction the whole design rests on — the treatment of
`/ActualText` as a character replacement "is different from the treatment of Alt, which is
treated as a whole word or phrase substitution". So:

| entry | clause | substitutes | for whom |
|---|---|---|---|
| `/ActualText` | §14.9.4 | characters | a person copying the page |
| `/Alt` | §14.9.3 | a whole phrase | a text-to-speech engine |
| `/E` | §14.9.5 | a whole phrase | a text-to-speech engine |
| `/Lang` | §14.9.2 | nothing; it labels | a text-to-speech engine |

Copying a ligature should give `fi`; copying a photograph should give nothing, and *speaking*
it should give its description. Two consumers, two answers, and a design that produced one
string would have to be wrong for one of them.

Hence: `/ActualText` is applied to `Interpretation::text` where it already was, and the other
three are recorded as **spans over that same string** — `Interpretation::described`, one entry
per marked-content sequence that states any of them. `Interpretation::speech()` combines the two
on demand and returns runs of text each carrying the language in force.

### Spans rather than a second string

The alternative was a second accumulator filled beside `text` as the glyphs are placed. Spans
win on three counts and one of them is measured:

- **An untagged page pays nothing.** No second buffer, no allocation per sequence: a `BDC`
  already reads its property list for `/ActualText`, and the three new entries are read from the
  same dictionary in the same pass.
- **The consumer decides when to pay.** Nothing that draws a page needs the spoken form, so
  building it during `interpret` would be work for a caller that does not exist yet. `speech()`
  is a method.
- **The rules are stated over ranges.** §14.9.3's "if each of two (or more) elements in a
  sequence have an Alt entry … they shall be treated as if a word break is present between them"
  is a statement about adjacency, and adjacency is what a range has and a concatenated string
  has thrown away.

Measured with `callgrind_interpret` over the specification's own page 101: **2 099.5 M
instructions against 2 099.8 M for the same page before the change** — no cost, and the two
numbers are from two builds made in the same sitting for exactly the reason the habit gives.

### Per entry, not per dictionary

Each of the four is looked for on the sequence's own property list first and on its structure
element second, **independently**. §14.9.3's own example is what forces that:

```
/Span <</Lang (en-us) /Alt (six-point star)>> BDC (A) Tj EMC
```

A file may state the language on the element and the description on the property list, and a
fallback that chose one dictionary and read all four out of it would lose whichever the file
split off. The element is still resolved at most once, and only where something is missing and
an `/MCID` names one: `structure.rs` records that following every entry of this page costs 96 M
instructions, so a lookup per entry would pay that four times.

### `/Lang` is the one that is inherited

§14.9.2.3 states the hierarchy and one clause of it is not a nesting rule:

> A structure element's language specification. If a structure element does not have a Lang
> entry, the element shall inherit its language from any parent element that has one.

So `structure::language` walks `/P` upward. `/P` is a reference a document controls and nothing
in §14.7 forbids a cycle, so the walk is bounded by `MAX_ANCESTRY`; reaching the bound answers
"no language stated", which is the answer an untagged document gives and is not a refusal —
a language is not a mark on the page, and declining to speak would be worse than speaking in the
default.

Everything else in §14.9.2.3 is ordinary nesting, **including the sentence that looks like an
exception**: "the structure element's language specification shall take precedence" over
surrounding unstructured content of a different language. The clause's own EXAMPLE 3 shows what
that means — a `/Span <</Lang (es-MX)>>` containing a `/P <</MCID 0>>` whose element states
`en-US` — and the structured content is the *inner* statement, so innermost-wins gives the
clause's answer without a special case. Both of the clause's examples are tests.

### One thing the standard does not decide

A sequence stating both `/Alt` and `/E` has said two phrase substitutions for one span, and
neither clause ranks them. **`/Alt` wins, and that is recorded as a choice**: §14.9.3 describes
the *item* while §14.9.5 expands the *text* the item contains, so where an item is described the
description subsumes what its text spells out. No corpus document states both — no corpus
document states an `/E` at all.

## Consequences

**Measured over the corpus**, page one of each of the 953 documents that reach one: 89 state a
document `/Lang`, and 35 carry §14.9 spans — 13 `/Alt`, 702 `/Lang`, and **no `/E`**. The `/Alt`
entries are exactly what §14.9.3 says the entry is for: `bug1937438_from_word.pdf` describes a
summation as `6 sum from n equals 1 to infinity of 1 over n squared , equals pi squared`, and
`bug1708040.pdf` describes an image that reads back as nothing at all as `A logo of a fox and a
globe`.

**No pixel moves and neither gate's numbers change**, which is §14.1's opening sentence rather
than a disappointment: the features of clause 14 "do not affect the final appearance of a
document". Both gates were run to confirm it — 858 documents draw with nothing reported, 821
pages agree with the reference consensus, 76 contradicted.

**The ledger's §14.9 family goes from nine `silent` rows to two `implemented`, four `partial` and
its two standing non-debts.** `silent` falls 193 → 188. Three rows stay `partial` and each names
what for: Table 122's `/Lang` on a CIDFont descriptor (§14.9.2.2, which is substitution quality
rather than accessibility), an annotation's `/Contents` as an alternate description (§14.9.3),
and §14.9.4's no-word-break rule between consecutive replacement texts.

**What is still owed is a consumer.** Nothing hands these runs to AccessKit, and until something
does, the strongest thing that can be said is that the data is read and tested. That is the
honest limit and it is written on the §14.9 row.

**And one stale ledger note was found by writing this one.** §14.9.4's row said its
structure-element half was owed — "the same entry on a *structure element*, which needs §14.7's
tree" — for four sessions after the fifty-sixth session built that tree and wired the fallback.
It is the ledger's own recorded failure mode seen from the other side: a note describes what
somebody found, and nobody revisits it when the thing it describes lands.
