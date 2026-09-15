# 1101 — A referenced page is an input a host supplies, matched by identifier and never by path

Session 1087. Status: **accepted**.

ISO 32000-2 §8.10.4.1 writes a `shall` to each of two classes of processor:

> PDF processors that do not recognise the Ref entry shall simply display or print the proxy as an
> ordinary form XObject. Those PDF processors that do implement reference XObjects shall use the
> proxy in place of the imported content if the latter is unavailable.

This tree executed the first completely and recorded that as conforming, which it is. Session 1084
asked whether the row was "a `may` mistaken for a debt" and answered no: a reachable target page is
content a producer specified, and `CLAUDE.md`'s "[e]very PDF that exists renders as its producer
specified" counts that as owed however §6.3.2.1 would score it. This ADR builds the second class
and decides the shape, so that a later round does not have to re-argue it.

## 1. The target document is an input, not a fetch

`CLAUDE.md` principle 3 gives the renderer no filesystem, and the reason bites harder here than
anywhere else it has been applied: Table 95's `/F` is a *file specification the document writes*, so
a reader that opened what it named would let a file choose what this machine reads. That is the
exact hazard the sandbox exists for, arriving through the content stream rather than through an
action.

So the target documents are supplied, on ADR 1076's shape, and the surfaces are its surfaces:

- **`pdf_model::reference::Supply`** holds the files a host read, opened once each, under the name
  the host knows them by, beside the sentence saying where they came from. `Supply::NONE` is the
  default and is what every caller that says nothing gets — `interpret_with_fonts` *is*
  `interpret_importing` with it, so the two cannot diverge.
- **`viewer_core::Command::References(ReferenceFiles)`** carries the bytes, on `Command::Trust`'s
  rules. The direction is the one that matters: the party that reads a file off a disk is outside
  the confinement, the party that **parses a PDF** is inside it, which is where every other document
  this program opens is parsed.
- **`viewer_host::reference_files`** is the one place a host answers *which files, if any*, beside
  `trust_anchors` and the seven decisions before it. `--reference-files <dir>` names a directory.
- **`viewer-confined`** carries them across the confinement as command kind 29.
- **`quorra_reference_files`** is the C ABI's. It takes no struct by value, so `QUORRA_ABI_VERSION`
  does not move.

**Nothing changes for a host that supplies none**, which is every host by default: every reference
`XObject` draws §8.10.4.1's proxy and reports nothing, which is what the clause writes for a
processor in that position and what this tree has always done.

## 2. The rule that may not be re-litigated

**The match is §14.4's identifier. Nothing reads a path, and no later round may make one read.**

This is not a caution invented here — §14.4 states the match itself, in a sentence whose subject is
a reference:

> If the first identifier in the reference matches the first identifier in the referenced file's ID
> entry, and the last identifier in the reference matches the last identifier in the referenced
> file's ID entry, it is very likely that the correct and unchanged PDF file has been found. If only
> the first identifier matches, a different version of the correct PDF file has been found.

So the permanent string decides *which file*; a difference in the changing string is Table 95's own
warning that the file "has changed since the reference was created", which is **drawn and said**
rather than refused. A `/Ref` with no `/ID` imports nothing and says so where files were supplied,
because the only other thing the dictionary offers is a path, and matching on one would hand a
document the choice of which of the host's files is opened for it.

Three of the five outcomes draw the proxy *and report*; the fourth — no host supplied anything —
draws it and says nothing, because the clause states that alternative and a report there would take
every page holding a proxy out of the oracle's comparison for a requirement the standard says is met
(trap 11).

## 3. An imported page is drawn with its own document's memos, or it is drawn wrong

§8.10.4.1 says what is drawn and where:

> When the imported content replaces the proxy, it shall be transformed according to the proxy
> object's transformation matrix and clipped to the boundaries of its bounding box, as specified by
> the Matrix and BBox entries in the proxy's form dictionary.

— which is §8.10.1's steps b) and c) over another document's page instead of this form's own stream,
with the proxy's `/Group` applying to the imported page as the clause's last sentence requires.
§8.10.4.3's first consideration puts that page's printable, unhidden, visible annotation appearances
inside the same box; its second is a `may` and is taken, so the imported page's logical structure is
ignored.

**And every memo the interpreter holds is swapped with the document.** `Interpreter::fonts`,
`icc_spaces`, `shadings`, `resource_tables`, `image_masks`, `image_rasters`, `stream_structures` are
keyed by an `ObjectId`, and `structure`, `output_intent`, `optional_content`, `page_resources`,
`view`, `delegated`, `page`, `ledger` and `stream` are derived from one document — two files hand out
the same object numbers, so a memo carried across the swap answers a question about the target
document with a fact about the containing one, in silence. `ImportedFrame` is that swap, its
destructure is exhaustive on purpose, and `Interpreter::across` became an `Option` for the same
sentence: a `FontCache` empties itself when another document's bytes arrive, so a page that imported
would clear the cache the containing document shares with every other page of itself.

## 4. What holds it, and why no corpus document could

`examples/absence_audit` asks §8.10.4.1's own condition over every PDF on this disk and finds none —
the population is empty, and the second half of the question is empty by construction with it: no
document states a `/Ref`, so none states a `/Ref` `/ID`, so no target file named by one can be here
to be matched. `tests/reference_xobjects.rs` is the fixture pair that forces (trap 8): one containing
document and one target, differing between tests in what is supplied and in nothing else, with four
colours so that every assertion excludes a *named* wrong answer rather than merely wanting one.

Three planted defects, each caught by exactly the tests it should be: the proxy's clip not carried to
the imported page's annotations, the annotation clip not set at all, and `resource_tables` not
swapped — the last drawing the target page in a shading its own file states and never paints, put
there to be the thing a leak would find.

## 5. What this does not decide

The imported page is drawn into the containing page's compositing. §11.4.7 says something else about
a page used this way — "it shall be treated as a transparency group using the page Group attributes
dictionary and is composited with its backdrop in the usual way" — and that sentence is about the
*target page's* own `/Group`, not the proxy's, so §8.10.4.1's last sentence is honoured here and
§11.4.7's is not. §8.10.4's ledger row carries it as the residue, named. Nothing here decides *whose* files a reader should
supply, which is ADR 1039's refusal repeated: building the input is not choosing a value for it.
