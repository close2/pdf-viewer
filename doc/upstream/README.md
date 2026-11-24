# Drafts for other projects' trackers

Each file here is one or more issues as they would be filed with another project — a validator, a
codec, a font library — written so that a person can file them as they stand. ISO 19005 is cited
by clause and paraphrased, never quoted (`doc/third-party-data.md`); ISO 32000-2 is quoted from
`doc/md/`.

## The template

Every section that is one issue carries these lines, in this order, above its argument:

```markdown
## <n>. <component>: <what it does, in one line>

**Filed**: <URL, or "not filed">
**Errata checked**: <date> — <what doc/errata/ holds for the clause: the site page's changes under
it, the pdf-issues numbers that name it and their state, or "none">
**Profile / component**: <file, rule, version>
**What the clause says**: <the clause, quoted from doc/md/ or paraphrased for ISO 19005>
**What it does instead**: <the observed behaviour, with the smallest witness>
```

**The `Errata checked` line is not optional** (ADR 3591, `doc/habits/judging-against-other-
implementations.md`). It records that the clause was searched in `doc/errata/` before the
difference was called the other project's: veraPDF-library #1668 was filed against a validator
that had followed an agreed erratum (pdf-issues #77, ADR 3582). Where the search finds an agreed
correction the draft is not filed and the departure is this tree's; where it finds a parked or open
issue, the draft names it, since the other project may be reading the same proposal.
