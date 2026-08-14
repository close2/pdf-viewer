# A cold document-wide search, and what is still in it

Status: **raised by the project owner on 2026-08-09**, on reading session 414's report:
*"Is the search implemented single threaded? 6.19s doesn't sound that fast. Can we easily improve
this? … Any improvement must be reasonable. Improving the search speed is a goal, but not a
requirement if the cost is too high (for instance in code quality or possibly also memory usage)."*
**Half answered in the four-hundred-and-twentieth session** (ADR 0256): the *repeated* search is
750× cheaper and the window's is **0.021 s** against about five seconds, and the *cold* search was
unchanged. **The cold one moved in the four-hundred-and-eighty-second** (ADR 0317), by memoising
§7.4's filter chain — a third of the instructions off a hundred-page sweep — and **again in the
four-hundred-and-ninety-fifth** (ADR 0330), by taking §7.7.3.2's page tree walk off the copies it
was making. What is left of it is a dictionary lookup's own allocation, below.
Priority: 47 — performance, measured, and explicitly **not** a requirement: the owner has priced
the trade in advance and code quality and memory both outrank the seconds
Corpus: every document; the cost scales with the *document*, not with the needle
Code: `crates/viewer-core/src/viewer.rs` (`find_step`, `readback`),
`crates/viewer-core/src/readback.rs`, `crates/pdf-model/src/page.rs` (`Pages::get`, `Node`),
`crates/pdf-syntax/src/document.rs` (`get_key_of`, `Dictionary::get`),
`crates/pdf-model/src/content.rs`. `crates/viewer-core/examples/find_cost` is the instrument, and
the invocation that splits a whole document is
`find_cost <file> <needle> 0 split 100000` under callgrind.

## Where the time goes, measured

`find_cost` drives the pump with no host at all, so the number is the search's. ISO 32000-2, 1023
pages, `--profile gates`, medians of three:

| one cold sweep, split | | |
|---|---|---|
| §7.7.3.2's page tree, `Pages::get` once per page | **1.12 s** | **19.6%** |
| the content stream, `interpret_with` | **4.59 s** | **80.4%** |
| building the index, `Pages::new` × 1023 | 2.8 ms | 0.05% |

**Both of the first two rows have moved since** — ADR 0317 took a third off the second and ADR 0330
took 71% off the first — and the wall clock they are in is a busy machine's. The split that decides
anything now is in instructions and is in those two ADRs; this table is kept because it is what
pointed at both.

And end to end, medians of seven, against the same binary with the cache's budget set to 0:

| | before | after |
|---|---|---|
| first (cold) sweep | 5.51 s | 5.61 s — **unchanged**, inside spreads of 0.16 and 0.32 s |
| repeated sweep | 5.45 s | **7.27 ms** |
| in the window (`Xvfb`, lavapipe), first / repeated | ≈4.9 s / ≈4.9 s | **4.79 s / 0.021 s** |

## What was done, and what it settles

**Candidate 3, the readback cache, is built** — `crates/viewer-core/src/readback.rs`, 4 MiB per open
document, least-recently-used, observable through `Viewer::readback_cache` and
`pdf-viewer --trace=search`. ADR 0256 has the argument, the invalidation rule and the cost; the
short of it is that ADR 0250 declined it against a bound nobody had stated, and the owner stated
one.

**Candidate 1, "interpret less", is answered rather than tried**, and the measurement is why: a
text-only narrowing of the walk could remove at most part of the 80.4%, and the cache removes
*all* of both halves for every page after the first. What it cannot do is help a *first* search
— and a second extraction path would be the wrong way to buy that, for the reason this file has
always given: it can diverge from what the page draws, and then a search finds words the reader
cannot see. **What did help a first search was inside the 80.4% rather than beside it**, and it is
the section below: the same font program inflated once a page.

**Candidate 2, parallelism, is measured and declined for now** — the four-hundred-and-twenty-fourth
session, ADR 0260. The blocker this file named was not one: **N documents in N threads needs nothing
from `pdf-syntax`**, and it reads 1023 pages in **1.18 s against 6.11**. `Document` is `Sync` now
(0.021% of a page interpretation's instructions, nothing measurable in wall clock) and one
`&Document` on 24 threads does it in **1.61 s** — but the peak resident memory is **625 MB shared or
966 MB per-thread against 225**, and the owner's stated bar is that 1 GB is definitely too much. So
what is open is `doc/todo/49` item 3, the API that hands a pool in, and it is a memory argument
rather than a lock. `pdf-model/examples/parallel_sweep` is the instrument.

**Candidate 4, skipping pages by scanning bytes, is still unsound** for the reason it always was.

## And a second one, found by counting: nothing memoised a decoded stream — now something does

**`Document::decoded_stream_data` ran 12 717 times over one sweep of ISO 32000-2 and 11 975 times
over the *second* sweep of the same document** (ADR 0260's counter build). That is §7.4's filter
chain re-run — a flate inflate per content stream, per font program, per image — on a document
nothing has changed.

**Taken in the four-hundred-and-eighty-second session, and it is the first thing on this file that
makes a *cold* search faster.** The two things the round owed were the two this file asked for. What
it is worth, measured before anything was built: **24.6% of a cold sweep is inside
`decoded_stream_data` and 23.4% of it is decoding something already decoded** — 830 MB of
re-inflation against 46 MB of first decodes, three font programs accounting for 3.2 s of the 3.9.
The budget is 4 MiB per open document, derived from the owner's band less what the readback already
spends and from a least-recently-used replay that says 4 MiB is 0.3 points short of unbounded.
ADR 0317 has the census, the derivation, the liveness invariant that makes an address a legitimate
key, and the callgrind A/B: **4 933 481 135 → 3 133 405 696 instructions over a hundred-page sweep,
−36.5%**, with the readback byte-identical.

**Wall clock was not the instrument and the reason is worth keeping.** The machine carried a load
average of 20 to 30 that session, and seven interleaved samples an arm gave medians of 9.75 s
against 6.73 s with ranges of 9 s and 8 s — the right direction and no evidence at all. A
measurement that has to survive a busy machine counts instructions.

## The page tree, taken — and the index this file proposed was not the answer

**A fifth of a cold search was walking §7.7.3.2's tree from the root, once per page**, and this
file said the way out was an index the walk fills as it goes. It owed the round two measurements
first, and the second of them changed the answer: *measure `find_leaf` before believing the walk
is all descent.*

It was not descent. **Two fifths of the walk was §7.7.3.4's inheritance being applied to every
node the walk stepped over and thrown away on the next line** — overlaying a page copies its whole
`/Resources` — and the rest was `Document::get` handing back a deep copy of each neighbour so that
Table 30's `/Count` could be read off it. ISO 32000-2's root has 9 children and one of them holds
998 of the 1023 pages, so finding page *n* copied about *n* page dictionaries: quadratic in the
page count, which is why the 1.10 ms on the thousandth page was a cost that grows rather than the
flat one it looked like.

**Taken in the four-hundred-and-ninety-fifth session** (ADR 0330), by holding a node as a *name*
until something needs the node: `Document::get_key_of` reads one entry out of an indirect object
without copying the rest, and `page::Node` is what the four walks carry. Nothing is copied until
the page asked for has been found. Callgrind, two binaries from one tree, one cold sweep of ISO
32000-2's 1023 pages: **42 884 714 194 → 37 676 397 310 instructions, −12.1%**, of which
`Pages::get` is **7 324 564 135 → 2 123 154 570, −71.0%** — 17.1% of the sweep down to 5.6%.
Interpretation moved 0.02%, which is the check that nothing else did. The readback is
byte-identical.

**It costs no memory at all**, which is the part this file should keep: the index of leaves and
the cache of resolved `Page`s were both answers to a question about the descent, and the descent
was never what it cost. `Pages` is still built per lookup and no tree is walked eagerly, so
`CLAUDE.md`'s startup rule is untouched rather than argued with.

## What is left: a dictionary lookup allocates, everywhere

Found by the same measurement and deliberately not taken with it. `Dictionary::get` is
`self.0.get(&Name::new(key.as_bytes().to_vec()))` — a heap allocation and a copy of the key on
**every dictionary lookup in the program** — and it is 1 529 118 804 instructions, **4.1% of a
cold sweep**, after the change above took it down from 2 503 477 461.

`Dictionary` is a `BTreeMap<Name, Object>` and `Name` is an `Arc<[u8]>`, so probing it with a
borrowed `&[u8]` needs `impl Borrow<[u8]> for Name`; `Name`'s ordering *is* its bytes' ordering,
so the map's invariant survives it and the change is a few lines. What it needs is its own A/B,
because it is the hottest path in the tree and it belongs to no clause: this is the whole reason
it was not folded into ADR 0330's measurement.

## What a round taking this still owes

- **Measure `find_step` alone**, not a host's loop. `find_cost` is the instrument now and prints the
  split itself with a fourth argument.
- **The gate is unchanged**: `tests/text_extraction.rs` and the 14 specification PDFs against
  `pdftotext`'s words, both exactly — the gate prints its own two figures and they are ratchets. A search that returns different results is a defect.
- **Do not undo the pump.** Rules 3 and 4 are not negotiable.
- **State what it costs in memory and in lines.**

## What is explicitly not owed

A match count ("3 of 17"), which needs the whole document read before the first answer. It is now
*cheap on a second search and unchanged on a first*, which is a different shape from the one ADR
0250 declined — but no host has asked, and nothing in the vocabulary prevents one doing it.
