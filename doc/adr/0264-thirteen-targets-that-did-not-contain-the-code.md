# ADR 0264 — Thirteen targets that did not contain the code

Date: 2026-08-10 (session 428)
Status: accepted

## Context

The four-hundred-and-twenty-fifth session found this project's **first crasher**: a 696-byte
document whose §7.10.4 stitching function names its own object, which overflowed the stack of
`target/pdf-retrieve` — a *shipped* binary — and of anything else that interprets a page in
process. It was found by **downloading a corpus**, not by fuzzing, and ADR 0261 recorded the
consequence in one sentence:

> Thirteen fuzz targets and not one of them could have found it.

`CLAUDE.md` puts fuzzing among the non-negotiables — "[f]uzzing from the first parser commit.
Every crasher found becomes a permanent regression test" — so a crasher a corpus found first is a
question about the instrument. This round asked it: **why did thirteen targets miss a 696-byte
file, and what should change?**

## Finding 1 — the tooling was there, and the round before this one said it was not

ADR 0261 and `doc/todo/03` both state that the gap was left open "because `cargo-fuzz` is not
installed on this machine". **It is installed**, at `/home/AI/.cargo/bin/cargo-fuzz`, version
0.13.2, dated **26 July** — a fortnight before session 425 — with the `nightly-x86_64-unknown-linux-gnu`
toolchain it needs beside it. `doc/HANDOVER.md`'s own environment list has said "**`cargo-fuzz`
needs `+nightly`** explicitly" for many sessions, which is a sentence nobody could have written
about a tool that was absent.

What made the claim survivable is that `~/.cargo/bin` is not on this shell's `PATH`, so `which
cargo-fuzz` answers nothing and `cargo fuzz` in a bare shell answers nothing. That is a two-second
check that reports a *false negative*, and both the ADR and the todo were written from it. The
corrections are in `doc/verify.md`, `doc/todo/03` and `doc/HANDOVER.md`; the lesson is
`doc/habits.md`'s shape and is worth stating plainly: **`which` answers a question about `PATH`,
not a question about the disk.**

## Finding 2 — eleven of the thirteen binaries do not contain the crashing function

The coverage question has a sharper instrument than an argument about what a target "reaches",
and it is the linker. Each fuzz target is its own binary and links only what it calls, so
`nm` answers whether a target *could* have executed a given function under any input at all:

```
$ nm <target> | grep -c parse_stitching
```

Over all fourteen binaries after this round, the answer is **2**: `variable_text` and the new
`page`. The same question asked of `pdf_model::interpret` gives the same two — and it corrects
ADR 0261, which said `confined_wire` reached the interpreter as well. It does not: that target
fuzzes the confined viewer's *wire decoders*, and the worker process that does the interpreting
is on the other side of a pipe and is not what its bytes are. So:

> **Twelve of the thirteen fuzz binaries do not contain `pdf_model::interpret` at all**, and the
> thirteenth calls it on a page that has no `/Resources`.

The second measurement is libFuzzer's own. Every target was run at `-runs=0` over its own corpus,
which replays it and prints two numbers: how many instrumented edges the binary *has* and how many
that corpus *covers*.

| target | edges in the binary | corpus files | edges covered | |
|---|---:|---:|---:|---|
| lexer | 970 | 3746 | 367 | `pdf_syntax::Lexer` |
| cms | 1596 | 711 | 450 | X.690 and RFC 5652 |
| sfnt | 1862 | 940 | 464 | §9.6.3's glyph-table repairs |
| fragment | 2017 | 2020 | 515 | Annex O |
| object | 2539 | 2344 | 679 | §7.3's grammar |
| x509 | 2935 | 374 | 389 | RFC 5280 and PKCS#1 |
| cmap | 3724 | 3368 | 629 | §9.7's `CMap` |
| xmp | 6503 | 6074 | 2240 | §14.3.2 |
| confined_wire | 18057 | 7988 | 6275 | the confined transport |
| crypt | 22775 | 4066 | 1257 | §7.6, in a fixed document |
| document | 23024 | 7730 | 3010 | §7.5, whole file |
| forms_data | 33912 | 1292 | 469 | §12.7.8, whole file |
| **variable_text** | **238744** | 5813 | **6483** | §12.7.4.3, in a fixed page |
| **page** (this round) | **237171** | 1882 | **28535** | clauses 8, 9 and 11, whole file |

Every row is that target's corpus as this round found it, before anything was seeded or run —
`document`'s and `page`'s both move below.

The two rows that matter are the last two, because they are the only pair whose *universes* are
the same: both link the whole of `pdf-model`, so a difference between 6483 and 28 535 is a
difference in what the **input shape** can express and in nothing else. `variable_text` puts the
fuzzer's bytes into a widget's `/DA` and `/V` inside a page it holds fixed — one Type 1 font, an
empty content stream, no resource dictionary — so 97.3% of the code it links is unreachable by
construction. It has 5813 corpus files and many rounds of evolution behind it; the new target has
1882 seeds and one day.

**Three of the thirteen could have produced the crasher's bytes** — `document`, `forms_data` and
`crypt` hand a whole file to `Document::open` — and **none of them could have executed it**.
`document` stops at `pdf-syntax` and its binary has no `pdf-model` in it; `forms_data` reads
§12.7.8's fields and imports them; `crypt` wraps its bytes in an encryption dictionary. The gap
was never that the fuzzers could not write the file. It was that nothing downstream of
`Document::open` was on the other end of one.

## Decision — a fourteenth target, over a whole document, through `interpret`

`fuzz/fuzz_targets/page.rs`. The fuzzer's bytes are a **whole document and nothing wraps them**;
the target opens it, takes page one and interprets it. Three properties, none of which is "the
page is drawn correctly":

- **No panic, no abort, no unbounded recursion.** Every descent the interpreter makes is a cycle
  a *valid* file may state — a form XObject inside a form XObject, a pattern whose content paints
  a pattern, a stitching function whose subfunction is itself — because §7.3.10 makes a reference
  something a reader follows and nothing in ISO 32000-2 forbids an object naming itself.
- **Interpretation terminates**, which is libFuzzer's timeout rather than an assertion.
- **Interpretation is a function of the bytes.** `CLAUDE.md` rests the whole cross-backend
  comparison on this — "`interpret` remains a pure function of what the file says" — and nothing
  in the tree checked it under an input nobody wrote on purpose. The target interprets twice and
  compares `DisplayList::geometry_digest`, the text and the glyph count.

Two limits are in the target with their measurements beside them. Inputs past **256 KiB** are
refused, and inputs past **16 KiB** are interpreted once rather than twice: a 236 KB corpus
document costs 0.8 s in a release binary and **15 s** in this one, where the sanitiser, the debug
assertions and two passes multiply, so charging every large seed twice would halve an exec rate
that is 10–30/s to begin with. Short inputs are where a fuzzer spends its runs and where the
crasher lived, so the purity property is checked exactly where it is cheap.

**It is not a rasteriser.** The display list is where the budgets live and where every
document-controlled decision is made; putting pixels behind it would cost an order of magnitude
of exec rate to fuzz a backend whose input is no longer the document.

## The target was checked against the crasher it was written for, historically

An argument that a new target "would have found it" is worth what a measurement is worth, so it
was measured. A detached worktree at **5fbf72a** — the commit before session 425's fix — took a
copy of `page.rs` cut back to what compiled there (`geometry_digest` arrived in session 426), and
the crasher was regenerated from `hostile_functions.rs`'s own construction. **That construction
emits 696 bytes, not 720** — asked of the test itself with one temporary `eprintln!` rather than
recalled — so ADR 0261's figure, and every document that copied it, is 24 bytes over. The shape is
what matters and the shape is unchanged:

```
==1937164==ERROR: AddressSanitizer: stack-overflow
    #2043 in parse_stitching /home/AI/wt-425/crates/pdf-model/src/function.rs:363:14
    #2044 in <pdf_model::function::Function>::parse /home/AI/wt-425/crates/pdf-model/src/function.rs:133:18
SUMMARY: AddressSanitizer: stack-overflow
```

One input, no mutation, 2044 frames. On today's tree the same bytes are interpreted in **4 ms**
and the refusal names its bound.

## Decision — the corpus is seeded from the documents already on disk

`fuzz/seed_page.py`, which is the fifth seeding recipe in this tree and follows `sfnt`'s lesson
(ADR 0175) one layer up: an unseeded target that never forms a valid structure tests nothing, and
a document is a much taller structure than a table directory. libFuzzer will not invent a header,
a page tree, a content stream and a resource dictionary that agree with each other in any number
of runs this machine has time for.

The seeds are what sessions 422–427 already fetched — **1944 SafeDocs documents**, `doc/corpora`'s
**108**, and the pdf.js submodule's **974** — filtered to the target's own 256 KiB ceiling, named
by SHA-256 so a re-run adds only what is new. **1882 files in the corpus, and 1142 documents past
the ceiling were skipped.** What they buy is printed rather than assumed, because a corpus that states
no shading seeds nothing about §8.7.4.5:

```
    887  an embedded font        234  /SMask
    681  image XObject           100  /Function
    545  /Annots                  62  /Pattern
    532  /Group                   58  /Shading
    332  form XObject             36  /OCProperties
```

Counted in the raw bytes, so every number is a lower bound — a construct inside an object stream
is invisible to a regular expression. `cargo fuzz cmin` takes the 1882 to **1535** files at the
same 28 535 edges.

## What the runs found

**`page`, 50 373 runs in 3411 s over six forks: 0 crashes, 0 out-of-memory, 1 timeout.** Coverage
over the run went **28 535 → 31 701** edges and the corpus 1535 → 4769 files, with libFuzzer
naming new functions as it went — `pdf_font::LoadedFont::load_simple`, `skrifa`'s
`GlyphHMetrics::advance_width`, `pdf_syntax::lexer::Token`'s formatter — none of which any target
had reached from a document before.

**The one timeout is the sanitiser's and not the product's, and it was checked rather than
assumed.** The unit is 62 467 bytes; on an idle machine it is **0.70 s in `target/pdf-retrieve`**
and **19.43 s in the fuzz binary**, a factor of 28 that is AddressSanitizer, the debug assertions
the fuzz profile keeps on, and — during the run — six forks sharing 24 cores. The five
`slow-unit-` artefacts read the same way: 0.03–0.89 s in release against 0.67–19.43 s under the
sanitiser. **No budget was touched**, and none needed to be.

So this round found no crasher, and the honest form of that is: **the regression test for the one
crasher this project has is now reachable by a fuzzer**, which is what was owed. `CLAUDE.md`'s
rule is satisfied by `hostile_functions.rs` and the historical run above is what connects the two.

**And an existing target was measured for what seeding is worth**, because "seed the corpus" has
been advice in this tree since ADR 0175 without a number attached to it anywhere but `sfnt`.
`document` takes a whole file exactly as `page` does and had **7730 corpus files** behind many
rounds of evolution, covering **3010** edges. Adding the same 1884 real documents took it to
**4351 — plus 44.6%** — for one `-runs=0` replay. A target whose input is a document and whose
corpus is not documents has been testing the recovery scanner.

**`document`, 50 000 runs after the reseed, in 515 s: 0 crashes, 0 timeouts**, 4351 → **4568**
edges and 9610 → 10 179 corpus files. Its ratio to `page` is the whole point of this ADR in one
line: 50 000 runs cost `document` 515 s and `page` 3411 s across six forks, and the second number
buys a body of code the first cannot execute.

## Two things fixed on the way, both because nothing was looking

- **`cargo fmt --all --check` does not see `fuzz/`**, because the fuzz crate is deliberately not a
  workspace member (its manifest says why: cargo-fuzz builds it with its own profile and
  sanitiser settings). Three targets were not rustfmt-clean under the project's own
  `rustfmt.toml`; they are now, and `doc/verify.md` carries the two commands that check it.
- **`cargo clippy --workspace --all-targets` does not see it either**, and `x509.rs` had a
  `redundant_slicing` warning. Fixed. `cargo clippy --all-targets` inside `fuzz/` is now silent
  and is written down beside the fuzz commands.

Neither is a licence to leave the fuzz crate out of the gate sequence: it costs a nightly
toolchain and a sanitiser build, and `doc/todo/02` §2 is 268 s for a reason. What it is, is a
reason the two commands are in `doc/verify.md` under the fuzzers rather than nowhere.

## Consequences

- **Fourteen fuzz targets.** `doc/todo/02` §2, `doc/verify.md` and `doc/HANDOVER.md` say fourteen
  and name what the new one covers; the "twelve" that three of them still said is corrected in
  the same pass.
- **`doc/todo/03`'s open item is closed** and its reason retracted: the tooling was present.
- The 50 GB corpus budget is unmoved at 5.4% and the promotion budget is unmoved at **0 MB** — a
  seed corpus is generated from files already on disk and nothing was committed.
- §7.10.4's ledger row keeps `implemented`; its three quotations were checked against
  `doc/md/ISO_32000-2_sponsored_EC3.md` line 3751 onward and against Errata Collection 3, and all
  three stand.
