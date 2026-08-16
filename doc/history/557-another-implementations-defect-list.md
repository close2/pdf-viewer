# 557 — Another implementation's defect list, read once and turned into tests

2026-08-16. A research round the project owner asked for: read all 167 issues on `LaurenzV/hayro`,
open and closed, decide what each is to this tree, and write up the ones that would interest the
quorra developers. ADR 0392 has the argument and the decisions.

## What came out

Four buckets, every issue in exactly one: **17** questions about a clause to put to this tree,
**36** about the three codec crates this tree links, **37** for quorra, **77** not relevant.

Two documents. [`doc/HAYRO_ISSUES_FOR_QUORRA.md`](../HAYRO_ISSUES_FOR_QUORRA.md) is the deliverable,
written to be handed over, and it says at the top what it is not: a defect list, a claim about
quorra, or a claim that hayro is right. [`doc/HAYRO_ISSUES.md`](../HAYRO_ISSUES.md) is the record of
the other three buckets, so that nobody reads the tracker twice.

## The turn the round took

The instruction said to fix a bucket-1 finding where it was small and clause-clear and otherwise to
write it down. Mid-round the project owner said something better: **make the checks into tests.**

That changed what the round produced. Sixteen of the seventeen bucket-1 issues turned out to be
things this tree already gets right, and a round that answers those in prose has written sixteen
sentences with a half-life — the exact shape `doc/todo/01`'s sweeps exist to find later. Eight
tests went in instead, each naming in its doc comment the issue it guards against and the clause
it rests on. The eight are listed in ADR 0392; the sharpest are a bowtie that must not be taken for
a rectangle (§8.5.3.3, because the two fill rules disagree about exactly the crossing a rectangle
fast path erases), a `/Length` that must not slice a sixteen-byte digest at thirty-two (§7.6.3.2),
and a number longer than the fixed-format fast path (§7.3.3).

## The one defect

`/Rows` is not a row count. `decode_ccitt` handed the `DecodeParms` `/Rows` to the codec whenever it
was non-zero; Table 11 gives `/EndOfBlock` the last word — "overriding the Rows parameter" — and
only "[i]f false" does `/Rows` bind. Its default is true, so the ordinary decode is bounded by
`/Height`. `pdf_model::ccitt_rows` is the derivation, and it is a named function so that Table 11's
seven cases can be a unit test.

No corpus document exercises it: a scan for a CCITT `/Rows` disagreeing with `/Height` over 1249
files found none, and the corpus and oracle gates moved nothing. The finding came from reading
somebody else's bug report and then reading the clause, which is a third source of work beside the
two `CLAUDE.md` names.

The same reading corrected two quotations that had dropped a precondition: `pad_to_height`'s doc
comment and §7.4.6's ledger note both cited the `/EndOfBlock` row's "whichever occurs first" as the
governing rule, without its "If false".

## The codec audit, which is the part with numbers

- **`hayro-ccitt` 0.3.0** — newest published, no bug fix after it. Nothing owed.
- **`hayro-jbig2` 0.3.0** — newest published, and four commits have landed after it. Their #1261's
  fix is one. Measured rather than assumed: the regression file that fix added upstream was fetched
  and run through `pdf_sandbox::decode` on a debug build, and it comes back as a clean typed
  refusal. **This tree's version is older than the defect** — the overflow became reachable through
  a June fast-path rewrite that 0.3.0 also predates — so there is nothing to take.
- **`hayro-jpeg2000`** — the fork pin's terms changed twice over. Two of its three fixes are now on
  hayro's `main`, one of them this project's own PR #1340, merged the day before this round. The
  third has no pull request. And a fourth condition appeared: #1188's `lab.ra`/`lab.rb` typo is
  present in **both** published versions and fixed on `main`, so going back to crates.io today would
  regain it. `Cargo.toml`'s un-pin note now says all three parts. One consequence reaches the
  oracle: `pdfref-hayro` carries that typo today, through `hayro-syntax`'s 0.3.5.

## The finding that is about method

hayro's #1331 — a Type 3 font without `/ToUnicode` yields no character — is a position this tree
held for three hundred sessions and corrected in the three-hundred-and-twenty-sixth, on two
sentences of §9.6.4 and §9.6.5.3 that neither implementation had read. Two independent readers,
the same plausible argument, the same wrong answer. It is principle 5 made better than any
statement of it: agreement between implementations is not evidence, because the thing two readers
of a clause are most likely to share is the misreading.

## Left open

[`doc/todo/53`](../todo/53-what-hayros-tracker-asked.md), three residues, each with what would
change the answer: the `/EndOfBlock`-false short decode, which needs a second field on the sandbox
pipe; `5f`'s missing diagnostic, which cannot be separated from the deliberate `12pt` leniency
without inventing a rule; and a Type 1 program's unassigned codes claiming glyph 0, which is a
`read-fonts` API question first.
