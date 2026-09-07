# 0922 — When veraPDF decides, and when the clause does

Session 940. Status: **accepted**, on the project owner's word, 2026-09-07. A refinement of
`CLAUDE.md` principle 5 for the one case principle 5 already treats specially: a requirement the
standard does not decide.

## The rule, in the owner's words

> If something is ambiguous, we trust veraPDF.

and, immediately after:

> It's only when it's ambiguous! Not when it's in conflict, then the spec wins, unless I say
> otherwise.

## Why this is not a departure from principle 5

Principle 5 forbids using another implementation as a *source of truth*, and forbids
curve-fitting to one. It also already says what to do where the standard genuinely defines
nothing: "say so plainly, make a deliberate choice, and document it *as a choice*."

This rule fills in that sentence. It does not say *what is true*; it says *which choice to make
where the standard has left one open*, and it picks the choice that costs a user least — because
a file this crate passes and veraPDF fails is useless to a depositor whose archive runs veraPDF.
That is an interoperability argument, and it is a different kind of argument from a truth claim.
Recorded as a choice, it stays visible and revisitable; `CLAUDE.md` warns that a claim of silence
in the standard **decays**, and each of these is exactly such a claim.

## The discipline, which is the order of the two tests

**The clause is read first.** A disagreement with veraPDF is never, by itself, evidence that the
clause is ambiguous — that inference is the thing principle 5 exists to prevent, because it turns
every difference into a licence to copy. So a disagreement is adjudicated into one of three
rulings, and only the third reaches veraPDF:

| ruling | when | what happens |
|---|---|---|
| `SpecAgainstUs` | the clause decides, against this crate | work owed; the predicate is fixed |
| `SpecAgainstTheCorpus` | the clause decides, for this crate | ours stands, and the disagreement is *recorded* rather than tolerated in silence |
| `AmbiguousSoTheirs` | the clause genuinely does not decide | veraPDF's reading is adopted, with a sentence saying **what** is undecided |

The owner's closing clause — "unless I say otherwise" — is the fourth path and is theirs alone: a
conflict where the standard decides against veraPDF, and the owner rules that this crate follows
veraPDF anyway. It is a per-case decision, not a policy, and it belongs in a `doc/questions/A*`
answer rather than in code.

## The mechanism

`crates/pdf-archive/tests/corpus.rs` carries `ADJUDICATED`: one entry per disagreement that has
been read, with its `Ruling` and the sentence justifying it. The sweep subtracts them into a
`settled` column, so **the disagreements a reader sees are the ones nobody has read yet**. A
sweep that re-reported every settled case would bury the new ones, which is the failure this
column exists to prevent.

## The first four adjudications, and what they found

All four are `SpecAgainstTheCorpus`, and they are one finding seen four times.

ISO 19005-2 §6.2.11.3.1 and ISO 19005-4 §6.2.10.3.1 require, where a Type 0 font's encoding is
not one of the identity CMaps, that the CIDFont's `Supplement` be **greater than or equal to**
the CMap's. The corpus files' own outlines state the case each is built from, and two of them are
inverted against the clause in each part:

- `…-t01-pass-a` (part 4) and `…-t01-pass-d` (part 2) state that the CIDFont's supplement is
  **less** than the CMap's — which the clause forbids — and are named passes.
- `…-t01-fail-c` in both parts states that it is **greater**, which the clause permits in as many
  words, and is named a failure.

**The direction is not a boundary case, and the standard says why.** ISO 19005-2 §6.2.11.3.1's
NOTE explains the purpose: the requirement ensures the font has glyphs for all the CIDs the CMap
can reference. That makes the supplement a *floor* on the font, and a font with more CIDs than
the CMap can name satisfies it. Both halves of the corpus have the comparison the other way
round.

### What veraPDF says about those files, checked rather than inferred

The owner asked for the other side of the disagreement to be looked up rather than assumed, and
it is more than a corpus slip. veraPDF's **validation profile itself is inverted**, in both the
rule it runs and the sentence it quotes:

| | ISO 19005-2 §6.2.11.3.1 and ISO 19005-4 §6.2.10.3.1 | veraPDF rule 6.2.11.3.1-1 and 6.2.10.3.1-1 |
|---|---|---|
| the sentence | the CIDFont's `Supplement` shall be **greater than or equal to** the CMap's | "shall be **less than or equal to** the Supplement key in the CIDSystemInfo dictionary of the CMap" |
| the test | — | `CIDFontSupplement <= CMapSupplement` |

The rest of veraPDF's description is the standard's own wording, word for word, including the
`Identity-H`/`Identity-V` exemption and the Registry/Ordering equality — **only the comparison
is reversed**. That is the shape of a transcription error rather than a reading: it propagated
from the description into the test expression and from there into the corpus files, which is why
all four witnesses point the same way in both parts.

Three checks were made for a reason to prefer their reading, and none of them found one:

- **No ISO corrigendum.** ISO 19005-1 has two technical corrigenda; no corrigendum to
  ISO 19005-2 touching this clause was found.
- **The PDF Association's own errata for ISO 19005-4:2020** record a single correction under
  clause 6, and it is to **Table 2, the PDF/A identification schema** — not to §6.2.10.3.1.
  (Worth a look for its own sake: Table 2 is where `metadata.rs` had to make a judgement about
  the `pdfa:conformance` prefix.)
- **No veraPDF issue** reporting the inversion was found, so it appears unreported rather than
  known and defended.

**PDF Association TechNote 0010**, which clarifies parts 1 to 3 and would be the one document
that could overturn this, returned HTTP 403 and was not read. That is the single loose end, and
it is named here rather than left as an assumption.

So this crate keeps its reading, and the four files are recorded. That the same inversion appears
in both parts is what makes it a property of the corpus rather than a slip in one file — and it
is worth reporting upstream, which is a courtesy this project can afford now that it owns the
text.

## Consequences

- **False positives are zero again** across the six targets, with four settled adjudications
  standing behind that number rather than a tolerance.
- The first use of the corpus produced a finding about the corpus. That is the relationship
  principle 5 predicts — evidence about a reading, in both directions — and it is the argument
  for keeping the harness a *report* rather than a gate.
