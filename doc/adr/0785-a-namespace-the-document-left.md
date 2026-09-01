# 0785 — A namespace the document left, and the blank page behind the recovered dictionary

Session 861. Status: **accepted**.

## Context

Two questions were handed on, and they are unrelated except that both are about saying out loud
what a *file* did rather than what this program could not do.

ADR 0784 recovered a page from a damaged dictionary's readable prefix. One of its three witnesses,
`GHOSTSCRIPT-701034-0.pdf`, then drew on its parent's rectangle and showed **nothing**, with its
content stream reported damaged after 310 decoded bytes. Either ADR 0343's prefix rule was not
reached from the recovered-dictionary route — a wiring defect — or the 310 bytes genuinely mark
nothing. Nobody had looked.

And §14.8.6's ledger rows stood `partial` on one sentence apiece: nothing reported the tagged
document whose elements end in no standard structure namespace. `Namespace::is_standard` and
`Tree::standard_role` already did the deciding; only the report was missing.

## 1. The blank page: the 310 bytes mark nothing, and the wiring is sound

The route connects, and unconditionally, because the recovery hands back an ordinary `Dictionary`:
`Pages::get` puts the prefix entries in as the page's dictionary, `ContentReader::for_page` reads
`/Contents` out of them like any other page's, and `Window::push` raises `ContentIssue::Damaged`
from the decode. Both reports fire on the same page and
`doc/checks/fixed-documents.toml` has pinned both since the round before this one.

What the 310 bytes are:

```text
q Q q 0 0 292 319 re W n /Cs1 cs 1 1 1 sc 149.7279 41.72791 m 156.7574 34.69848
156.7574 23.30152 149.7279 16.27209 c 142.6985 9.242624 131.3015 9.242624
124.2721 1.27209 c 1497.2426 23.30152 197.2426 34.69848 124.2721 41.72791
c 131.3015 48.75738 142.6985 48.75738 1492 319 re W n /Cs1 cs 1 1 1 sc 149.7279 1
```

Two clip operations, a colour space, a colour, a path built out of `m` and `c`, and a run of
operands the damage cuts off mid-number. **The only path-painting operator in the whole prefix is
`n`**, which §8.5.3.1 defines as ending "the path object without filling or stroking it … a
path-painting no-op, used primarily for the side effect of changing the current clipping path". So
the producer's own bytes, as far as they go, place no mark on the page — and the colour they would
have painted with, `1 1 1` in the page's `/Cs1`, is white.

The interpreter agrees to the command: `examples/open_one` reports **0 commands** and both
sentences. So the page is blank because the file is blank up to its damage, and the honest thing to
do about it is to say so where somebody will read it. `doc/checks/fixed-documents.toml`'s `why` for
that row now carries the operator list and the clause; the band was already `0.0 .. 1.0`, which was
the seeded value and is therefore already a pin on blankness.

**The general form is worth more than the file.** A recovery that produces a blank page has two
possible causes that look identical from outside — the recovery lost the marks, or there were none
— and the way to tell them apart is not a metric but the operators. Trap 1's sentence one step
further in: *the metrics lie, look at the page* has a companion, which is that a blank page is not
a diagnosis either. Look at the operators.

## 2. The eleven that declare `/Type /Page` nowhere

ADR 0784's census split the standing-count population by cause and left its largest cause unread,
because the answer for it was already right: no object in the file declares itself a page, so
ADR 0782's standing `/Count` and a refusal out loud is what a reader owes. Eleven documents. Read
against §7.5 and §7.7.3 they are **five defects**, and the account is in
`doc/todo/03-more-corpora.md` §34 with a file apiece.

What is worth recording here is the instrument and the shape:

- **The census prints the account rather than a document asserting it.** A byte scan for
  `/Type /Page` can only ask about an object's own declaration, and all eleven have stopped making
  one. The question that separates them is what the page tree's own `/Kids` names — the file's
  statement about an object, made where the object's damage cannot reach it (§7.7.3.2: "[t]he
  children shall only be page objects or other page tree nodes"). `standing_count_census` now
  follows every `/Kids` array of every object that parses and prints, per named object that does
  not resolve, how far it reads and which entries were whole before the damage.
- **Two of the eleven have no bytes to recover from**: the tree names objects the file does not
  contain, with cross-reference offsets running past the end of the file. **One has no object
  header that lexes.** **Two have a prefix of zero entries.** Those five are beyond additive
  recovery, each for a stated reason.
- **Five have a route the standard supplies twice, and it is a door ADR 0784 deliberately did not
  open.** `/Kids` says the child is a page object or a page tree node; §7.7.3.4 says which entries
  a node may legitimately carry beyond Table 30's four, because `Resources`, `MediaBox`, `CropBox`
  and `Rotate` are inheritable — so a prefix stating `/Contents` or `/Annots` was written by a
  producer describing a page. ADR 0784's consumer requires the prefix to declare `/Type /Page`
  *itself*, and rightly, because it finds its candidates by scanning the whole file; this one would
  be handed its candidate by the tree. It is not taken here: it needs the recovery to run from the
  tree rather than from the scan, which `Pages::new` does not do.
- **One discriminates nothing**, and taking it would mean deciding it is a page on the strength of
  it not looking like a node — the substitutive direction trap 5's test forbids.

## 3. §14.8.6.2's requirement on the file, reported

> In a tagged PDF, all structure elements shall be in at least one of the standard structure
> namespaces or in a namespace identified in 14.8.6.3 , ' Other namespaces '.

Every other sentence of §14.8.6 is addressed to a reader and is executed — which map applies to
which element (§14.8.6.2), the default namespace for an element that states none (§14.8.6.1), and
what a name in a foreign namespace *means* (`Tree::standard_role`). This one is addressed to
whoever wrote the file, and §7.3.7's row had already established that such a `shall` is answered by
a report rather than left to a validator.

**Where the report goes, and why it is not a page's.** `viewer_core::notes::about` — a
`Event::Reported` with no page, said once when the document opens, beside §12.11's requirements and
§7.5's rebuilt table. The violation costs no mark on any page, so an `Unsupported` would take a
page out of the oracle's diagnosed set to say something that is not about it (trap 11's arithmetic,
ADR 0152's).

**The condition is the clause's, in two parts.** §14.8.1 is what makes a document tagged — "[a]
tagged PDF document shall contain a mark information dictionary … with a value of true for the
Marked entry" — and a document with a structure tree that does not claim to be tagged is outside
the sentence, which says *in a tagged PDF*. Then `Tree::namespaces_outside_the_standard` walks the
elements and asks, of the namespace each one *ends* in after §14.8.6.2's transitive role mapping,
whether it is one of §14.8.6.1's two or §14.8.6.3's one. A namespace dictionary stating no `/NS` of
its own is counted apart and worded apart: Table 356 makes the entry required, so such an element
is in a namespace the document does not name, which is a different thing to tell a person.

### The cheap gate, and the blind spot it leaves

The elements have to be walked to answer the clause, and the walk is **151 ms** on the largest
tagged document this tree holds — `MAX_ELEMENTS`' own note has the measurement — while
`notes::about` runs on the launch path. Paying that on every tagged document open would be a
principle-2 regression of the plainest kind.

The clause is what makes it unnecessary: "[i]f the structure element is in an explicit namespace,
then that namespace shall be identified in the structure tree root dictionary's Namespaces array
entry". So the root's `/Namespaces` array is the set of namespaces a conforming file's elements can
be in, and a root declaring none outside the permitted set has no element to find. That test is a
handful of dictionary lookups, and it is what every tagged document pays.

**What it leaves is stated rather than hidden**: an element whose `/NS` names a namespace the root
does not list is not seen. Such a file breaks the sentence above as well as the one being answered,
and closing it costs the walk on every tagged document — so the gate is a reading of the clause
with a measured price beside it, not an optimisation nobody argued.

### Calibration

Trap 13, both directions, and the negative half is four fixtures rather than one, because the
clause states four ways to satisfy it:

- the planted violation — a tagged document whose element names
  `http://example.invalid/tagset` — **is named, with the namespace and the count**;
- the same file that does not claim §14.8.1's `/Marked true` says nothing, because the sentence
  says *in a tagged PDF*;
- the same file whose namespace carries `/RoleMapNS << /Widget /Div >>` says nothing, which is the
  third bullet — "role mapped into the namespace, either directly or transitively";
- an element in §14.8.6.3's MathML says nothing;
- an element with no `/NS` at all says nothing, which is the second bullet.

**And a floor under the exemption**, which is trap 11's ninth instance: an exemption that costs
nothing to widen has none. `doc/pdf.js/test/pdfs/bug1937438_af_from_latex.pdf` declares four
namespaces of its own — two LaTeX tagsets and a `data:` URI — so the gate **opens** on it and the
whole tree is walked; the answer is empty, because each of those namespaces carries the
`/RoleMapNS` the clause asks for. Without a case like that the gate could be inverted and every
other test would still pass.

### The population

**No witness**, in the pdf.js corpus, the four `doc/corpora/` submodules, this project's fixtures,
or the 65 944-document `CC-MAIN-2021-31` crawl. `examples/absence_audit` carries the claim, which
is where this project keeps the "no corpus document does X" measurements, and it is measured with
the reader that decides the report rather than with a grep for `/Namespaces` — which over-reports
twice, since a declared namespace may be a standard one and may have no element in it. Four
documents in the tree declare a `/Namespaces` array at all and three of those declare a foreign
namespace; all three role map completely.

That is what a file-addressed conformance report should look like: a condition derived from the
clause, calibrated against the defect, and costing nothing because no file in reach breaks the
rule.

### One caller that was not asking

Writing the row found a reader of §14.8.4's vocabulary that did not go through
`Tree::standard_role`: `viewer_core::accessibility::nodes` derived its `kind` from
`StandardType::read` of the *name*, so a foreign namespace's `Table` would have had Table 384's
`/Summary` read off it and a foreign `TH` its `/Short` — the exact homonym `standard_role` exists
to refuse, in the crate that publishes the tree to a screen reader. It now asks `standard_role`,
and `header_scope` takes the standard type rather than re-deriving it from the name. No corpus
document is a witness, which is why it survived the whole life of `standard_role`.

## Consequences

- §14.8.6, §14.8.6.2 and §14.8.6.3 are `implemented`. §14.8.6.3's remaining `shall` — that MathML
  be enclosed under a `Formula` element — is addressed to the producer of the tagging and is
  covered by `CLAUDE.md`'s closed authoring exclusion; the row names it.
- `viewer_core::notes` is six clauses rather than five.
- The eleven documents have an account, and the two doors they hand on are argued rather than
  left as a cause line in a census.
- `standing_count_census` answers a question it could not: what the page tree names, as against
  what an object declares about itself.
