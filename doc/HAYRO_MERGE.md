# Can the two codebases be merged?

Status: **evaluation** — written for the project owner, before any contact with hayro's
maintainer. Round 569. ADR 0404 records the decision.
Read by: whoever is about to propose something to `LaurenzV`, or whoever wants to reopen this.

The question this file answers is not "is the collaboration working" — it is, and #1340 settles
it. The question is **whether the two codebases can become one, and what that would mean**.
Everything below is counted rather than asserted; every number in it was produced by a command
named in §10, over `tmp/hayro` at `1dc833f7` and over this tree.

---

## 0. The recommendation, first

**No merge of the syntax, interpreter, font or rendering layers, in either direction.** Not
because the collaboration is bad — because **the thing that makes it valuable is the
independence**, and a merge spends exactly that.

**What to do instead is not "nothing".** Three moves, in order of value, all of which fit in one
message:

1. **Offer the reduced-resolution pull request.** It is written, rebased, measured and waiting
   (`doc/HAYRO_PR_REDUCED_RESOLUTION.md`). It is the fourth of the four parts of our own un-pin
   condition.
2. **Ask for release cadence — or co-maintainer rights — on the three codec crates.** This is the
   only friction the evidence actually shows, and it is not review latency. `hayro-jbig2` 0.3.0
   has four unreleased commits behind it; `hayro-jpeg2000` 0.4.0 lags `main` by three fixes *and*
   carries the `lab.ra`/`lab.rb` typo in **both** published versions. Their bottleneck is
   publishing, not fixing. A merge would fix this at enormous cost; a release trigger fixes it for
   free.
3. **Propose exchanging specification *readings* rather than code.** We have 875 ledger rows and
   747 clause citations in source; `doc/HAYRO_ISSUES.md` bucket 1 is already seventeen of *their*
   issues answered against ISO 32000-2 with the clause quoted, and it can be handed over today.
   This is the one form of sharing that makes both implementations more correct **without** making
   either less independent — it strengthens principle 5 instead of eroding it.

The draft message is §9.

**What would change my mind** is §8, and the first item on it is short: hayro's maintainer has
stated that *"my main priority is definitely speed as opposed to 100% correctness"* (issue #60,
quoted in `doc/HAYRO_ISSUES_FOR_QUORRA.md`). `CLAUDE.md`'s first principle is the opposite
sentence. Two projects may hold different priorities and still help each other every week. One
codebase cannot hold both.

---

## 1. The crate map

Line counts are *production* lines — `#[cfg(test)]` modules inside `src/` excluded, which is why
`hayro-syntax` reads 12 550 here and 15 253 to `wc -l`.

### Their crates, and what a merged world would do with each

| theirs | prod lines | our counterpart | ours | outcome |
|---|---|---|---|---|
| `hayro-syntax` | 12 550 | `pdf-syntax` | 10 398 | **stays separate — and cannot be adopted at all.** §3 and §5. |
| `hayro-interpret` | 26 399 | `pdf-model` | 68 493 | **stays separate.** Ours is 2.6× the size because it is a different job: clause 12 and 14 as well as 8 and 9. |
| `hayro-interpret/src/font/`, `hayro-cmap` | 3 398 + 2 241 (a further 13 791 are generated tables) | `pdf-font` | 12 495 | **stays separate**, and the sharing that already exists (`skrifa`) is at the right level. §4.2 item 4. |
| `hayro-postscript` | 986 | `pdf-font/cmap.rs` | — | separate. Not a §7.10.5 evaluator: it "only implements a very small subset", and "[u]nsupported is anything else, including dictionaries, procedures". |
| `hayro` (the renderer) | 1 757 | `pdf-render` + three backends | 24 435 | **stays separate.** One `vello_cpu` backend against a neutral display list with five device decisions stated once so that backends cannot differ. |
| `hayro-svg` | 1 684 | — | — | no counterpart and none wanted. |
| `hayro-write` | 711 | `pdf-syntax/write.rs` | — | **separate and opposite.** `hayro-write` builds new pages and XObjects with `pdf-writer`. That is authoring, which is on `CLAUDE.md`'s closed exclusion list; ours only appends §7.5.6 incremental updates. |
| `hayro-jpeg2000` | 9 534 | — | — | **already ours, as a dependency.** Keep it exactly that way. |
| `hayro-jbig2` | 8 755 | — | — | same. |
| `hayro-ccitt` | 1 057 | — | — | same. |
| `hayro-tests` (357 PDFs) | 3 820 | corpus + oracle (974 PDFs) | — | **the one place a merge is cheap and worth doing.** §7 position C. |
| `hayro-demo`, `hayro-bench`, `hayro-fuzz` | 1 398 | examples, benches, `fuzz/` | — | separate. |

### The shape that falls out of the table

Of hayro's **65 674** production lines:

- **19 346 (29%) are already in our shipped binary** as `hayro-jbig2`, `hayro-jpeg2000` and
  `hayro-ccitt`, reached from exactly one place (`pdf_sandbox::decode`) inside a confined worker.
- **2 395 (4%) are out of our scope** — `hayro-svg` and `hayro-write`.
- **43 933 (67%) duplicate work this tree has already done**, to a bar §3 counts.

And of our **184 565** production lines, the crates with *no hayro counterpart at all* —
`pdf-spec`, `pdf-sandbox`, `raster-compare`, `test-scenes`, `viewer-core`, `viewer-confined`,
`viewer-host`, `viewer-ui`, `viewer-gtk`, `viewer-qt`, `viewer-ffi`, `viewer-accessibility`,
`tools/pdfref`, `tools/conformance`, `tools/spec-errata`, `tools/pdf-retrieve`,
`tools/safedocs`, `tools/hayro-compare` — come to **68 744 lines, 37% of the tree**. A merge does
not touch one of them.

**The overlap is smaller than the word "merge" suggests, and it is concentrated in precisely the
layers where a merge is most expensive.**

---

## 2. Direction

Three shapes were considered. The answer is the third, and the useful part of the answer is that
**it has already happened**.

- **We absorb hayro.** Licence-clean: they are `Apache-2.0 OR MIT`, we are Apache-2.0, and taking
  the Apache-2.0 arm is free. (This read "we are MIT, and taking the MIT arm is free" until the
  eight-hundred-and-eighty-seventh session relicensed this tree; the conclusion is unchanged and
  the arm taken is the other one.) But absorbing 43 933 lines with 0 clause citations and 3 957 lint sites (§3) is not
  a merge, it is a rewrite of somebody else's code — and it buys nothing we do not already have,
  since ours is the larger and more complete implementation everywhere the two overlap.
- **hayro absorbs us.** Licence-costly: our Apache-2.0 code would need dual-licensing to keep
  their `Apache-2.0 OR MIT` offer intact, which is the owner's decision alone. (Theirs is nearly a single-party decision — `git shortlog` since
  2026-02-01 gives 353 of 373 commits to the maintainer, the other 20 split across eleven people.)
  It is also a handover of direction to a project whose stated first
  priority is speed over correctness. And it ends this project.
- **A shared lower half is extracted, and both sit on it.** This is right, and **the extraction
  already exists**: `hayro-jbig2`, `hayro-jpeg2000` and `hayro-ccitt` are separate crates, we ship
  all three, and the boundary is not arbitrary. §4 says why it is the *only* boundary available.

**Recommended direction: no absorption; deepen the extracted lower half that already exists.**

---

## 3. The cost of our own bars, counted

Method: `cargo clippy --workspace --lib --no-deps` over `tmp/hayro` at `1dc833f7`, with this
tree's `[workspace.lints]` set reproduced on the command line — `clippy::pedantic`,
`unwrap_used`, `expect_used`, `panic`, `arithmetic_side_effects`, `missing_errors_doc`,
`missing_panics_doc`, `undocumented_unsafe_blocks`, `missing_debug_implementations`,
`unreachable_pub`, `unused_qualifications` — and `--force-warn missing_docs`, which is needed
because `hayro-jbig2` and `hayro-postscript` carry `#![allow(missing_docs)]` at crate level.

**3 957 distinct warning sites.** Per crate, with the categories that matter:

| crate | total | arithmetic | unwrap/expect | missing docs | casts | other |
|---|---|---|---|---|---|---|
| `hayro-syntax` | **1 199** | 235 | 43 | 145 | 254 | 522 |
| `hayro-jpeg2000` | 776 | 319 | 18 | 0 | 182 | 257 |
| `hayro-interpret` | 686 | 115 | 10 | 0 | 292 | 268 |
| `hayro-jbig2` | 604 | 277 | 3 | 0 | 200 | 124 |
| `hayro-ccitt` | 235 | 21 | 1 | 0 | 7 | 206 |
| `hayro` | 178 | 43 | 7 | 0 | 93 | 35 |
| `hayro-svg` | 88 | 9 | 10 | 0 | 49 | 20 |
| `hayro-cmap` | 73 | 26 | 8 | 0 | 12 | 27 |
| `hayro-postscript` | 64 | 28 | 1 | 2 | 8 | 25 |
| `hayro-write` | 41 | 0 | 6 | 0 | 21 | 14 |
| **total** | **3 957** | **1 073** | **107** | **154** | **1 118** | **1 504** |

For calibration, the same command over three of ours — `pdf-syntax`, `pdf-render`, `pdf-sandbox`
— prints **0**. Our lints are workspace-level and CI treats warnings as errors, so that is what
"clean" means here rather than a claim about it.

Four readings of the table, each of which is the *cost* rather than the number:

- **1 073 `arithmetic_side_effects` and 1 118 cast warnings** are not style. `CLAUDE.md` names
  them: "[a]rithmetic on untrusted input is a correctness and DoS concern, not a style one".
  Every one of those sites is a question about what a hostile document can do, and answering
  2 191 of them *is* the merge rather than a tidy-up before it.
- **154 missing-doc sites**, 145 of them in `hayro-syntax`, plus two crate-level
  `#![allow(missing_docs)]` opt-outs. `#![warn(missing_docs)]` is enforced here.
- **107 `unwrap`/`expect` sites** in production code, and separately **33** `panic!` /
  `unreachable!` / `todo!` / `unimplemented!` sites by grep. Our rule is no `unwrap()` outside
  tests and provably-infallible cases, *with a comment naming why it cannot fail*. Each of the
  140 is an argument to be written, not a line to be changed.
- **The number nobody would guess: 0.** Across all 65 674 production lines of hayro there is not
  one occurrence of `ISO 32000`, `32000-2` or `§`. This tree has **747** in `crates/` and **875**
  ledger rows resting on them. §6.

**None of this is a criticism of hayro.** Their workspace lint set is the Linebender canonical
one, they run it clean, and they comment `rust.unsafe_code = "forbid"` out with the note "This one
may vary depending on the project." They are meeting their own bar. The number above is the
distance between two bars, and it is the merge cost because our bars are non-negotiable and
theirs are not ours to change.

---

## 4. The oracle consequence

The brief called this possibly deciding. It is close — but the shape is not the one the brief
assumed, and the correction matters.

### 4.1 A merge costs us no *vote*, because hayro already has none

`tools/pdfref/src/reference.rs` returns, for `Reference::Hayro`:

> `Independence::Shared("shares skrifa, flate2, zune-jpeg, hayro-jbig2 and hayro-jpeg2000 with us")`

and `Reference::voting()` filters to `Independent`. The module header says "**It never votes.**"
The project has already refused to let hayro reach a verdict by the back door as well:
`Tolerance::widened_to` measured hayro-against-poppler as a candidate fourth spread and declined
to use it, because widening our bound on that evidence

> would forgive whatever the two of us get wrong together — which is the circularity
> `Reference::independence` exists to prevent, **and the same circularity whether it reaches the
> verdict through a vote or through a bound.**

So no verdict in this tree moves if hayro disappears tomorrow. Anyone arguing that a merge costs
"an independent oracle reading" in the voting sense is wrong, and this tree wrote down why more
than a hundred rounds ago.

### 4.2 What it *does* cost, in descending order of value

1. **The stream of clause questions — the biggest loss, and the least obvious.** ADR 0392 read all
   167 of hayro's issues. Seventeen were questions about a clause put to this tree; **one found a
   real defect** (`/Rows` is not a row count — Table 11 makes `/EndOfBlock` override it, and it
   defaults to true), and eight became tests naming the issue they guard. That stream exists
   because somebody else's users hit somebody else's implementation of the same clauses. **It is
   generated by the difference.** Merge the interpreters and it stops, permanently.
2. **The Type 3 shape.** Both trees independently concluded that a Type 3 glyph name says nothing
   about the character. Both were wrong, and §9.6.4 step b) with §9.6.5.3 settles it in two
   sentences neither had read. `doc/HAYRO_ISSUES.md` calls that "the strongest single argument for
   principle 5 this round found". **It is visible only while there are two readers.** One merged
   reader makes that mistake once and nothing in the world contradicts it.
3. **The fourth artefact panel.** `oracle.rs` renders hayro *after* the verdict, only on
   non-agreeing pages — "[a] fourth render, for the eye rather than for the vote" — into the
   side-by-side strip `doc/HANDOVER.md` calls the fastest diagnostic in the tree.
4. **The hinting-boundary measurement.** `PairKind::AcrossTheHintingBoundary`: over 786 ambiguous
   pages, `ours + hayro` is the closest of the ten renderer pairs on **651**, and on 612 of the
   670 text ones; median 1.94 of 255 against 5.39 for the closest voting pair. That is currently
   the only measurement of the font half of trap 9, **and it works precisely because hayro shares
   `skrifa` with us and shares nothing else**. Merge the font stack and it measures a thing
   against itself.
5. **The speed baseline.** `hayro-speed` is where "6.9× faster on the complete pages and 14× over
   every page" comes from. A merged codebase cannot be its own baseline, and this is the only
   like-for-like one available — same language, same safety rules.

### 4.3 Is the loss confined to the layers merged?

**Yes, and that containment is the whole argument.** Items 3, 4 and 5 are lost only if the
rendered picture stops being independently produced — that is, only if the interpreter, the font
stack or the renderer merges. Items 1 and 2 are lost by merging any layer where readings
*differ*, which is those same layers.

**Merging the codecs costs none of the five, and that is not luck.** A JBIG2 or JPEG 2000 decoder
has an independent reference implementation — `opj_decompress` for ISO/IEC 15444, `jbig2dec` for
T.88 — so correctness there is checkable without a second PDF reader at all. `pdf-model`'s
`tests/jpeg2000.rs` already does this: all 30 corpus codestreams decoded through
`hayro-jpeg2000` and through the reference software and compared **exactly**, with a
`DIFFERS_FROM_THE_REFERENCE_SOFTWARE` list of 13 held by name in both directions. That is shared
code judged against a truth neither project wrote.

**The content-stream interpreter has no such reference, and that asymmetry is the boundary.**
Above the line where an independent reference exists, a second implementation *is* the
instrument. Below it, the instrument is the reference software and a second implementation is
merely a second cost. The extracted lower half of §2 sits exactly on that line.

### 4.4 Does anything recover it?

Only a fifth independent reading, and there is none in prospect: pdfium and Acrobat are the
candidates, neither is pure Rust, and neither is a fair speed baseline. **The loss at the
interpreter level is not recoverable.**

---

## 5. Architecture fit, constraint by constraint

Each row is a hard constraint in `CLAUDE.md`, not a preference.

| our constraint | what hayro does | verdict |
|---|---|---|
| `#![forbid(unsafe_code)]` on every crate touching PDF bytes | `hayro-interpret`, `hayro` and all three codecs forbid it. **`hayro-syntax` does not**: `page.rs:529` is an unconditional `unsafe { core::mem::transmute(xref.deref()) }` to `&'static XRef`, and 13 further sites sit behind an `unsafe` feature that is on by default and which the crate "strongly recommend[s]" enabling. | **conflict at the parser**, compatible elsewhere |
| incremental parsing — 500 pages open no slower than 5 | `Pdf::new` → `CachedPages::new` → `resolve_pages` materialises a `Vec<Page>` for the **whole** tree at open, resolving `/MediaBox`, `/CropBox`, `/Rotate` and `/Resources` inheritance per page. | **conflict** |
| nothing eager on the launch path | the same call — `CLAUDE.md` names "no full page-tree walk" explicitly. | **conflict** |
| multi-process sandbox (seccomp-BPF + Landlock) | none. And `Device` is a `&mut self` callback trait rather than a display list, so an interpretation is not a *value* and cannot cross a pipe — which is exactly how `viewer-confined` works. | **conflict for the confined worker** |
| explicit memory and time budgets | `MAX_XREF_CHAIN_DEPTH = 256` and an LZW `MAX_ENTRIES = 4096`. Nothing else: no pixel budget, no decode budget, no time budget. Ours are `pdf_sandbox::MAX_PIXELS`, `MAX_SAMPLES`, the lockdown address ceiling, and a typed `LimitExceeded`. | **conflict** |
| GPU-first through quorra | `hayro` targets `vello_cpu`. A Vello-Scene backend is an open ask (#821), and the maintainer's stated reservation is correctness: *"vello doesn't support everything that is needed for correct rendering AFAIK (for example masks)"*. | **direction conflict**, not a code one — a `Device` impl could feed a quorra scene |
| `interpret` a pure function of the bytes and the view state | not stated, not tested; `cache.rs` sits beside the interpreter. Adaptable, but unverified — and the oracle's whole comparison rests on this being true. | **adaptable, unverified** |
| every layer reports what it could not handle, as a typed value | `log::warn!`/`error!` behind a `logging` feature. `Device` has ten methods and **not one reports a refusal**. `Page::new` does `dict.get::<Rect>(MEDIA_BOX)…unwrap_or(A4)`, so a page with no `/MediaBox` silently becomes A4 — where ADR 0389 makes that a report, because otherwise "a guessed sheet look[s] like a measured one". | **conflict, and the worst one** |

That last row is the one that would break something quietly. Our oracle *removes a page from the
judged set* when the interpreter reports (trap 11, ADR 0152's arithmetic). If interpretation stops
producing a typed report, the oracle keeps producing verdicts while no longer knowing which pages
it is entitled to judge — the silent-failure shape this project spends its rounds removing.

**Two of the eight are adaptable; six are conflicts, and three of the six are load-bearing
architecture rather than a fix.** The `&'static` transmute in `hayro-syntax` exists *to support*
the eager page cache, so the unsafe row and the incremental-parsing row are one defect seen twice:
neither can be taken without redesigning the other.

---

## 6. What the conformance ledger becomes

`doc/conformance/ledger.toml` holds **875 rows**. Each is a claim about *this tree's* code with a
clause behind it, and `tools/conformance` verifies the quotations against `doc/md/`. What makes a
row mean anything is that somebody read the clause beside the code.

Adopted code brings **0 citations across 65 674 lines**.

Retrofitting is not mechanical: a citation is a claim, and writing one honestly means reading the
clause. `CLAUDE.md`'s own retrofit policy is "clause-family by clause-family as work reaches it,
**never as a separate marathon**". Adopting 43 933 uncited lines would *manufacture* precisely the
marathon that policy exists to prevent, and would run it over code nobody here wrote.

The alternative — and this is what a merged world would actually force — is a **provenance axis**
on every row: "this claim is about code this project read" against "about code adopted and not yet
read". That is a strictly worse instrument than the one we have, because a row's status would stop
meaning one thing, and `silent` — the status `doc/HANDOVER.md` calls the one worth hunting —
would stop being comparable across rows.

**The ledger is not merge-compatible, and it is the project's main instrument for the coverage
question.** That is an independent reason for the same conclusion §4 reaches about the robustness
question, which is why the two together are decisive rather than merely additive.

---

## 7. The intermediate positions, priced

| position | costs | buys | verdict |
|---|---|---|---|
| **A. Shared codecs — where we already are** | already paid: a fork pin with a four-part un-pin condition, a 13-entry known-defect list, and "a spec question we hit becomes an issue report" (ADR 0014) | 19 346 lines of the most error-prone code in the format, already validated against corpora larger than ours, judged against reference software neither project wrote | **keep** |
| **A+. Release cadence, or co-maintainer rights, on the three codec crates** | a review obligation on behalf of somebody else's users | removes the *only* friction the evidence shows. `hayro-jbig2` 0.3.0 has four unreleased commits; `hayro-jpeg2000` 0.4.0 lags `main` by three fixes and carries `lab.ra`/`lab.rb` in both published versions — going back to crates.io today would *regain* a bug | **do this** |
| **B. Shared font stack above `skrifa`** | destroys the hinting-boundary measurement (§4.2 item 4) and imports the encoding logic where readings differ most — §9.6.5.2, §9.6.5.4, §9.10.2 | little: their font code is smaller than ours and does less | **no** |
| **C. Shared corpus and test infrastructure** | close to nothing — no code enters either binary | their 357 PDFs against our 974, with different failure modes; and we can offer the JPEG 2000 rig (30 codestreams against `opj_decompress`, compared exactly), which is a correctness oracle for a codec that *has* an independent reference | **yes, cheaply** |
| **D. Shared specification *readings*, published as documents** | the writing, which we pay anyway | the only sharing that raises correctness on both sides **while leaving both independent** — it strengthens principle 5 rather than eroding it. `doc/HAYRO_ISSUES.md` bucket 1 is seventeen of their issues already answered against the standard, and it can be handed over today | **yes — and it is the interesting proposal** |
| **E. Shared interpreter / full merge** | 3 957 lint sites, 43 933 uncited lines, the ledger's meaning (§6), and oracle items 1–5 (§4.2) | one implementation instead of two | **no** |

---

## 8. What would change my mind

- **A stated change of priority.** If hayro's first priority became correctness rather than
  speed, the direction conflict in §0 would go, and B and E would be worth re-pricing. Nothing
  else on this list matters as much.
- **`hayro-syntax` dropping its unconditional `unsafe` and gaining lazy page access.** Those are
  one change, not two (§5). It would make the syntax layer adoptable *in principle* — at which
  point the oracle argument becomes the binding reason rather than the bars, and the answer is
  still no, but for one reason instead of six.
- **A typed unsupported channel on `Device`.** If an interpretation became a value that names what
  it could not do, the worst row of §5 would close and position C would grow considerably: their
  suite could then be run under our gates.
- **A fifth independent reading joining the oracle.** It would reduce the cost of items 3–5 in
  §4.2. It would not touch items 1–2, which are about two readers of a clause rather than two
  rasters.
- **The reverse, which would settle it the other way:** if A+ is declined and the codec crates go
  on shipping fixes without releases, the pressure for a deeper arrangement is real, and this file
  should be reopened — with **vendoring**, not merging, as the next position to price.

---

## 9. The message

Short, concrete, proposing three specific things and no merge. Every fact in it is from this file,
and nothing in it commits the owner to work beyond the pull request that already exists.

---

Hi Laurenz,

Thanks for #1340 — and for rewriting it rather than just taking it. Passing `irreversible: bool`
and returning `f32` is a better shape than the quantisation style we passed, because it puts the
lossless case in the type instead of in a match.

I maintain a PDF viewer in Rust that ships `hayro-jbig2`, `hayro-jpeg2000` and `hayro-ccitt`, and
uses `hayro` itself as a fourth reading when our renders disagree with poppler, mupdf and
ghostscript. Three things I'd like to propose, none of which is a request for your time on our
behalf.

**1. One more JPEG 2000 pull request, ready to send.** `build_decompositions` sizes the
coefficient buffer from the full-resolution component tile even when a reduced resolution level
was asked for — a single 3.4 GB allocation for one file in our corpus, whatever raster you want
back. Sizing it from the highest level that will actually be decoded takes peak address space
from 3336 MB to 115 MB at 788×1051; all 183 of your own test assets stay byte-identical and the
Annex B.4 example still passes. Rebased on `main`. Say the word and I'll open it.

**2. Releases for the codec crates.** This is the only thing that actually costs us anything
today. `hayro-jbig2` 0.3.0 has four commits behind it, including the zero-width-bitmap overflow
fix; `hayro-jpeg2000` 0.4.0 lags `main` by three fixes and still has the `lab.ra`/`lab.rb` typo
that `c2df2014` fixed, so we pin a fork rather than go back to crates.io and regain a bug. If
publishing is the bottleneck rather than reviewing, I'm happy to take a co-maintainer role on
those three crates and do release work — or just agree a trigger, e.g. cut a release whenever a
correctness fix lands. Whatever suits you.

**3. Something you might find more useful than patches.** We derive every expected value from ISO
32000-2 with the clause quoted, and keep a conformance ledger of 875 rows against it. Reading your
tracker recently, seventeen open and closed issues turned out to be questions about a clause
rather than about hayro — so we answered each against the standard, and one of them found a real
bug **in our code** (#1337: `/Rows` doesn't bound the decode, because Table 11 makes
`/EndOfBlock` override it and it defaults to true — we were refusing images we should have
drawn). I'd be glad to send you those seventeen readings, with clause numbers, and to keep doing
that as your tracker grows. Also happy to share our JPEG 2000 conformance rig: 30 codestreams
from the pdf.js corpus decoded through `hayro-jpeg2000` and through the ISO/IEC 15444-5 reference
software and compared exactly, which is how we measured #1340.

To be explicit about what I'm *not* proposing: merging the two implementations. Your interpreter
disagreeing with ours is the most useful signal either of us gets — we independently reached the
same wrong conclusion about Type 3 glyph names and `/ToUnicode`, and the only reason anyone
noticed is that there were two of us. I'd rather keep that and trade readings.

Christian

---

## 10. How to reproduce every number here

```sh
# the lint cost, from the root of tmp/hayro at 1dc833f7
cargo clippy --workspace --lib --no-deps --message-format=short -- \
    -W clippy::pedantic -W clippy::unwrap_used -W clippy::expect_used -W clippy::panic \
    -W clippy::arithmetic_side_effects -W clippy::missing_errors_doc \
    -W clippy::missing_panics_doc -W clippy::undocumented_unsafe_blocks \
    -W missing_debug_implementations -W unreachable_pub -W unused_qualifications \
    --force-warn missing_docs

# the same lints over three of ours, which print nothing
cargo clippy -p pdf-syntax -p pdf-render -p pdf-sandbox --lib --no-deps

# citations and rows
grep -rn "ISO 32000" --include=*.rs crates | wc -l                          # 747
grep -rn "ISO 32000\|32000-2\|§" --include=*.rs tmp/hayro/hayro*/src | wc -l  # 0
grep -c "^\[\[" doc/conformance/ledger.toml                                 # 875

# production line counts, excluding `#[cfg(test)]` modules inside src/
#   tools/hayro-bars.py in this round's scratchpad; the split rule is
#   "a #[cfg(test)] attribute followed by `mod` opens a region that closes
#    when brace depth returns to where it started".
```
