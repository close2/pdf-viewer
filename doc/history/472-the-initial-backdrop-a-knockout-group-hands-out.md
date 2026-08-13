# 472 — The initial backdrop a knockout group hands out

**Finding.** `doc/todo/23`'s open knockout item named three corpus documents and **two of them were
not that item**. §11.4.6's initial backdrop and §11.4.4's immediate one are the same wherever the
knockout rule can change no pixel, and NOTE 6 hands a non-isolated group nested in a knockout group
that group's initial backdrop — so `knockout_blend_multiply.pdf` is §11.4.4's group wearing `/K`
and `knockout_inner_backdrop.pdf` is §11.4.5's wearing `/I false`. The first contradicted all three
references at mean 23.96 of 255 and now agrees with them; the second was drawn right and reported
wrong.

**Date.** 2026-08-13.
**ADR.** [0307](../adr/0307-the-initial-backdrop-a-knockout-group-hands-out.md).
**Touched.** `crates/pdf-model/src/content.rs` (`Interpreter::transparent_initial_backdrop`,
`Interpreter::for_page`, `run_transparency_group`, `note_group_departures`, `note_group_structure`,
`build_soft_mask`, the tiling group), `crates/pdf-model/tests/transparency_groups.rs` (two tests
added, one amended, one fixture helper), `crates/pdf-model/tests/oracle.rs`
(`AMBIGUOUS_KNOCKOUT_GROUP`'s prose), `doc/conformance/ledger.toml` (§11.4.4, §11.4.6, §11.6.7),
`doc/oracle-and-corpus.md` §3, `doc/todo/23-transparency-departures.md`, `doc/todo/README.md`,
`doc/adr/0307-*`, this file.

## Why these two and not the second colour space

The round was pointed at `doc/todo/23` with two open items and told to take whichever the clause
reading makes finishable with a discriminating test. The other one — a group inside the page that
introduces its own blending space — needs `Command::Group` to carry a space and a second command
list, three backends to resolve the pair, and is 8 web documents and 0 corpus ones. This one turned
out to need no construction at all, which is not what its own file said, and the way that was found
is worth the sentence: **the three named witnesses were opened and read before anything was
priced.** Both are 900- and 1146-byte hand-written pdf.js fixtures whose whole content is four
objects; five minutes of reading them against §11.4.6 is what said two of the three were mislabelled.

## The picture, and the three references

`knockout_blend_multiply.pdf` is a yellow page, one non-isolated knockout group, one element: a
cyan rectangle under `/BM /Multiply`. §11.3.5.2's Multiply is the componentwise product, so the
clause asks for `(1,1,0) × (0,1,1)` — green. One element has nothing to knock out, so §11.4.6's
initial backdrop *is* the page, and this tree gave it §11.4.5's transparent one instead, against
which Multiply returns the source: cyan.

The oracle before and after is the independent half of that, and the direction of inference is
principle 5's:

```text
before  knockout_blend_multiply.pdf page 1: CONTRADICTED (incomplete)
        — poppler and mupdf and ghostscript agree, we differ:
          ours at worst mean 23.96 worst tile 63.75 differing 9.55%; bound mean 1.00
after   (absent — the page agrees)
```

The clause reading came first and the three references agreeing afterwards is evidence that it was
read right, not the reason for it.

`knockout_inner_backdrop.pdf` moved no pixel. It is `/K true /I true` outside, `/K false /I false`
inside, and NOTE 6 says the inner group's initial backdrop is the outer group's — transparent — so
this tree's isolated construction *is* the clause there. What it did was report a departure it does
not have, which took the page out of the oracle's judged set. Trap 11 with nothing to fix but the
condition.

## Gates

`fmt`, `clippy --workspace --all-targets` (silent), `nextest --workspace` **1697 passed, 11
skipped**, doctests, the sandbox worker build, the corpus gate, `pdfref-hayro`, the oracle, both
text gates, dates, xmp, jpeg2000, the quorra corpus gate (**918 agree, 37 differ, 1 refused, 18 not
comparable**) and `conformance`.

Before and after, taken by stashing the round's `crates/` and re-running:

| | corpus incomplete | oracle agrees (of them complete) | contradicted | ambiguous |
|---|---:|---|---:|---:|
| before | 67 | 905 (859) | 68 | 786 (754) |
| after | **65** | **906 (861)** | **67** | 786 (754) |

Two documents leave the incomplete list; one leaves the contradicted set for the agreeing one; the
other was already agreeing and joins the *complete* pages the oracle judges. The reference cache hit
99.8%, which is what says the corpus and the renderers did not move underneath the comparison.

The quorra gate's second coverage lane was **not** owed: `doc/todo/02` §2 asks for it from a round
that takes a quorra release or changes the zoom path, and this round did neither.

## The ink sweep, and one thing it says

`doc/todo/00` step 7, owed because this round changes pixels. Our ink minus the lightest reference's
over **all 786 ambiguous pages**, both halves taken with one instrument (`-alpha off -colorspace
Gray`), before with the round stashed and after with it applied: **byte-identical, all 786 lines**.
Twenty at or past −1, sixteen of them documents this tree calls incomplete, and the four complete
ones are the four standing diagnoses — `issue16038.pdf` −5.734 (`AMBIGUOUS_TILING_CELL_CLIP`),
`issue12295.pdf` −2.823 (`AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY`), `issue14297.pdf` −1.145 and
`issue7821.pdf` −1.000 (`AMBIGUOUS_GRADIENT_QUANTISATION`). The alarm holds again.

**The identity is a claim rather than a shrug**, and it is the same one the 405th recorded: the page
this round moved was *contradicted* before and *agrees* after, and a page crossing those two states
is invisible to a sweep whose population is the ambiguous bucket. What the identity does say is that
no ambiguous page's ink changed — which is a real statement here, because `blendmode.pdf` is in that
bucket at −0.535 and is sixteen blend modes over a backdrop.

**One caution for whoever compares these numbers with the file's own.** `doc/todo/00` records
`issue12295.pdf` at −2.956 in the 444th and −1.709 in the 405th; this run reads −2.823. Two
instruments are in circulation in that file — `-alpha off -channel R -colorspace Gray` and `-alpha
off -colorspace Gray` — and a cross-round comparison that does not say which was used is comparing
two things. The before/after pair above used one, which is the comparison the step is for.

## Two things the next round should know

1. **`interpret_into` was one line under `clippy::pedantic`'s hundred**, and one struct field pushed
   it over. `Interpreter::for_page` is the split, and it is a better shape than the function it came
   out of — but the next round that adds a field to `Interpreter` should know the constructor is
   where it goes, and that the lint has been at the boundary once.
2. **The ledger is a TOML basic string and a `"` inside a note breaks the file, not a test.** Two
   quotations went in with raw quotes and `cargo test -p conformance` failed with
   `TomlError { line: 3095, expected: "nothing after the value" }`, which reads like a checker
   complaint and is a parse error. `\"` in the note; the checker's own quotation sweep
   (`doc/todo/01`) is a separate thing entirely.
