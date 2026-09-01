# 861 — A namespace the document left, and the blank page behind the recovered dictionary

2026-09-01. Argued in [ADR 0785](../adr/0785-a-namespace-the-document-left.md).

Touched: `crates/pdf-model/src/structure.rs`, `crates/viewer-core/src/notes.rs`,
`crates/viewer-core/src/accessibility.rs`, `crates/pdf-model/examples/standing_count_census.rs`,
`crates/pdf-model/examples/absence_audit.rs`, `doc/conformance/ledger.toml`,
`doc/checks/fixed-documents.toml`, `doc/todo/03-more-corpora.md`,
`doc/adr/0785-a-namespace-the-document-left.md`.

## 1. The blank page was blank

ADR 0784's witness `GHOSTSCRIPT-701034-0.pdf` draws on its parent's rectangle and shows nothing.
The two candidate explanations were a wiring defect — the recovered-dictionary route not reaching
ADR 0343's content prefix — and the 310 bytes genuinely marking nothing. **It is the second.**

The route is sound and unconditional: the recovery hands back an ordinary `Dictionary`, so
`ContentReader::for_page` reads its `/Contents` like any other page's and `Window::push` raises
`ContentIssue::Damaged` from the decode. Both sentences fire on the one page and
`doc/checks/fixed-documents.toml` has pinned both since the round before this one —
`examples/open_one` prints `0 commands` and the two reports side by side.

The 310 decoded bytes are `q Q q`, two `re W n` clips, `/Cs1 cs 1 1 1 sc`, a path built from `m`
and `c`, and operands cut off mid-number. **The only path-painting operator in the whole prefix is
`n`**, which §8.5.3.1 makes "a path-painting no-op"; the colour it would have painted with is
white. The `why` line in `fixed-documents.toml` now carries that, because a blank page from a
recovery has two indistinguishable-looking causes and only the operators tell them apart.

## 2. The eleven that declare `/Type /Page` nowhere are five defects

`doc/todo/03` §34 has the file-by-file account and `examples/standing_count_census` now prints it
rather than a document asserting it: the census follows every `/Kids` array of every object that
parses and reports, for each named object that does not resolve, how far it reads and which entries
were whole before the damage. That is the question a byte scan for `/Type /Page` cannot ask, and it
is §7.7.3.2's own sentence — "[t]he children shall only be page objects or other page tree nodes".

- Two have **no bytes to recover from**: the tree names objects the file does not contain, with
  cross-reference offsets past the end of the file.
- One has **no object header that lexes** (`3 0 obj5<`), which is `PDFBOX-4339-0.pdf`'s defect
  again.
- Two have a **prefix of zero entries**; one of those two has its `/Type /Page` four bytes past the
  damage, which is the second door below.
- **Five have a route the standard supplies twice** — `/Kids` says page-or-node, §7.7.3.4 says
  which entries a node may legitimately carry, so a prefix stating `/Contents` or `/Annots` was
  written by a producer describing a page.
- One **discriminates nothing** and taking it would be substitutive.

Neither door is opened here. Both are argued in `doc/todo/03` §34 so the next round takes an
argument rather than a hunch.

## 3. §14.8.6.2's requirement on the file is reported

`Tree::namespaces_outside_the_standard` decides it and `viewer_core::notes::about` says it, once,
when the document opens — a `Event::Reported` with no page, because the violation costs no mark and
a page report would take a page out of the oracle's diagnosed set to say something not about it.

**The cost, measured rather than asserted.** The elements have to be walked, and on the largest
tagged document in this tree — ISO 32000-2's own 1023 pages, 129 389 elements — that walk is
**153.7 ms**, which cannot go on the launch path. §14.8.6.2 is what makes it unnecessary: an
explicit namespace "shall be identified in the structure tree root dictionary's Namespaces array
entry", so the root's array is the population and a root declaring nothing outside the permitted set
has no element to find. On that same document the whole check costs **180.9 µs** and does not enter
the walk. The blind spot it leaves — an element naming a namespace the root does not list — is
named in the code, in the ADR and on the row.

Calibrated both ways (trap 13): the planted violation is named with its namespace and count, and
each of the clause's four ways out — not tagged, role mapped, MathML, no `/NS` — leaves it unsaid.
The floor under the cheap gate (trap 11's ninth instance) is a real document,
`bug1937438_af_from_latex.pdf`, whose four declared namespaces open the gate and whose role maps
then satisfy the clause.

Population: **no witness** in the pdf.js corpus, the four `doc/corpora/` submodules, this project's
fixtures, or the 65 944-document `CC-MAIN-2021-31` crawl — `examples/absence_audit` carries the
claim.

Writing the row found one caller that was not asking: `viewer_core::accessibility::nodes` derived
its standard type from `StandardType::read` of the *name*, so a foreign namespace's `Table` would
have had Table 384's `/Summary` read off it. It asks `Tree::standard_role` now.

**Ledger**: §14.8.6, §14.8.6.2 and §14.8.6.3 all move to `implemented`. §14.8.6.1 already was.

## Gates

The full §2 sequence, twice — once before the cycle fixture and once after it — green both times,
with the second run the one that counts. §5's binaries rebuilt and installed from the final tree.
Sweeps run: `owed`, `overstated`, `capabilities`, `undenominated`, `pointers`, `quotations`; no hit
belongs to this round.
