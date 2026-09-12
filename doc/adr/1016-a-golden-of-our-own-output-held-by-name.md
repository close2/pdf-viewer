# 1016 — A golden of our own output, held by name, and the diff that is the review

Session 996. Status: **accepted**. Builds the instrument `doc/reviews/984-direction-and-boundaries.md`
Finding 4 ranked second and ADR 1005 §4 priced: a per-page digest of the CPU raster over the tracked
corpus, held by name in a committed file, failing the round that moves a page and regenerated in the
open by the round that means to.

Context: `crates/pdf-model/tests/raster_golden.rs` (new), `crates/pdf-model/tests/raster_golden.tsv`
(new), `crates/pdf-model/examples/raster_digest.rs` and `display_list_digest.rs` (doc comments),
`crates/viewer-core/tests/accessibility_census.rs` (the model, read), ADRs 0282, 0912, 0945, 0970,
0985, 1005; `doc/traps/instruments-and-reports.md` traps 1, 13, 25, 37, 39;
`doc/traps/pixels-and-rasterisers.md` trap 1. `doc/PLAN.md:276-279`, which promised this in the first
month.

## 1. What the tree could not see

The oracle's last recorded run (`doc/history/985`) judged 1,956 pages: 990 agree, 62 contradicted,
**835 ambiguous**, 47 not comparable, 17 no render. On an `ambiguous` page no two references agree
closely enough for anybody to be called wrong, and nothing then holds *our* pixels: the
`AMBIGUOUS_*` groups hold a diagnosis by name, and the only measurement is the ink sweep, whose alarm
is one level of 255 and which could not see a page drawn in the wrong place at the right weight for
hundreds of sessions (ADR 0945). `render-raster`'s corpus gate compares the third rasteriser against
the CPU one, so an interpreter change moves both sides together. Two examples — `raster_digest` and
`display_list_digest` — compute exactly the digest wanted, and `grep -n digest tools/state.sh` found
nothing: they were run by hand, in two arms, when a round remembered to.

So on 43% of the population an interpreter regression was invisible unless it moved ink. The review
called this "the instrument that was planned and does not exist", and it is the fourth in a row of
its kind — ADRs 0962, 0985, 0970 and 1006 were each an instrument that existed and measured the wrong
thing; this one measured nothing because it was never a gate.

## 2. What was built

`crates/pdf-model/tests/raster_golden.rs`, ignored by default and run under the gates profile like
every corpus gate. It walks `doc/pdf.js/test/pdfs` — the **tracked** corpus, a submodule pinned by
commit, 974 documents — and for each first page opens the document (with the corpus's eight known
passwords where §7.6.4.1's default refuses), interprets it, rasterises it through `render-cpu` at 72
dpi, and holds **three digests and an outcome** in `tests/raster_golden.tsv`, one line per page, keyed
`<file> p1` (ADR 0970's key shape):

```
issue1002.pdf p1	drawn	612x792	7f0d…	3a91…	e3b0…
issue19517.pdf p1	no-target	-	-	c480…	4f53…
```

The columns are the raster's extent, then sixteen hex digits of a SHA-256 over the raster's bytes,
over the display list's `Debug` rendering, and over the interpretation's reports. The outcome word is
held because a page that drew and is now refused is a change too.

**The check is ADR 0970's construction, in both directions:**

- a page on disk whose line **differs** fails, naming the page and which layer moved;
- a page on disk with **no line** is reported as `unheld` and passes — a name that is absent proves
  nothing, so a corpus that grew or a submodule at a newer commit passes and says what it could not
  hold;
- a line whose page is **not on disk** is reported as `left` and passes — a name that leaves is
  examined rather than deleted quietly (ADR 0282), and `update` removes it in the open.

The denominator is printed on the first line (trap 25): documents on disk, pages drawn, entries in
the file. A missing file is a failure with the command that writes it, because an instrument with an
empty population that passes is trap 25's shape exactly.

**The update path** is `PDFVIEWER_RASTER_GOLDEN=update`, which rewrites the file whole and sorted and
prints the same classification the check prints — every moved, joined and removed entry — so the
round can name them in its history file and commit the file with the change. An environment variable
rather than a flag because everything after `--` belongs to the test harness, which refuses a flag it
does not know; `tests/corpus.rs` already uses the same convention for `PDFVIEWER_CORPUS_TRACE`.

## 3. Three digests, because the diff has to say which layer moved

ADR 1005 §4 priced "a per-page digest of the CPU raster" alone and asked whether
`display_list_digest` should join it. It joined, as two more columns of the same file rather than a
second golden, and the reason is what a moved page needs first: a diagnosis.

| raster | list | reports | the gate says |
|---|---|---|---|
| moved | same | — | `raster only (a rasteriser change under an unchanged list)` |
| moved | moved | — | `raster and list (an interpreter change)` |
| same | moved | — | `list only (an interpreter change no pixel shows)` |
| same | same | moved | `reports only (a change in the diagnosis, trap 37)` |

The fourth row is ADR 0912's finding made a gate: session 936 changed how a real is read and the
whole of what moved was a *sentence* — a false accusation against `GHOSTSCRIPT-695872-0.pdf` that no
command and no pixel showed. A digest that cannot see what the program said is a digest of half the
artefact, and trap 37 says so. The marginal cost is one `Debug` string per page, which the example
already paid in a single pass; the whole gate is measured in §5.

**The hash is SHA-256 and not the examples' `DefaultHasher`**, which the standard library documents
as unspecified across releases. That is fine for two arms in one sitting and disqualifying for a file
read by a later toolchain; it is why the examples could not simply become the gate. Sixteen hex
digits are sixty-four bits, ample for *detecting a difference* — the only question asked — and the
extent column beside them makes a lone collision not a false pass. `sha2` was already in the crate's
graph for §12.8.3.

## 4. Calibrated both ways, and what each direction showed (trap 13)

**A name that leaves, a name that joins.** With `tracemonkey.pdf p1` deleted from the file and
`never-existed.pdf p1` added, the run exits 0 and prints:

```
held 973, moved 0, unheld 1, left 1
  unheld: tracemonkey.pdf p1 (on disk, not in the file — proves nothing)
  left: never-existed.pdf p1 (in the file, not on disk — examine it before it goes)
```

**One pixel.** In a scratch worktree with its own build directory — never this one, where three
neighbours build — `render-cpu`'s `rasterize` was made to invert the low bit of the first byte of
every raster it returns: one channel of one pixel, one level. The gate names **every drawn page and
nothing else** — 966 of 966 drawn, all classified `raster only`, and the eight pages with
no raster held — and exits 101. A digest sees one level of one pixel, which the ink sweep's alarm
(one level over the whole page) cannot.

**The pages it moved and not the others.** `examples/raster_digest`'s own calibration moved
`Medium::PAGE_ONLY`'s surround off white and recorded that 193 of 957 pages changed — the pages whose
extent is not a whole number of pixels, where `TargetSpec::for_page` rounds up and a sliver lies
outside the crop box. The same plant in the same scratch tree names **193 pages**, every one
`raster only`, and holds the rest: the instrument names the population a change moved, and not the
population it did not.

## 5. Determinism and cost

Two consecutive `update` runs produce **byte-identical** files (`cmp` silent, 974 entries, 966 drawn),
and the check run afterwards holds 974 of 974. The interpreter is a pure function of the document,
the CPU rasteriser is deterministic for a display list, and the pages run in parallel but each on one
thread; the one thing that can move a digest without a code change is the sandbox worker — a page
whose JBIG2, JPX or CCITT image it did not decode holds no command for it — so the gate refuses to
measure without it, like every other corpus gate (`require_the_sandbox`, ADR 0557).

Alone under `tools/bounded.sh`: **28 to 31 seconds over 974 documents, peak 7.1 to 7.3 GiB resident
over the process tree**, 24 rayon threads at nice 19. A digest is a count, not a clock: the seconds are
what the run costs a round, and nothing in the file depends on them.

## 6. What this is not, and the discipline it costs

**It is not a correctness claim.** `CLAUDE.md` principle 5 governs what this program is compared
against; this gate compares it with itself, and a digest that moves says *these pages draw differently
from the commit that last held them* and nothing about which was right. The round that moved them
says why.

**The discipline is the cost ADR 1005 §4 weighed**, and trap 39's shape is near: a signal that fires
on every pixel change is only a signal if a round reads which pages moved and says why. What makes it
a signal rather than a chore is that the gate prints the *classification* and the file's diff is the
*list* — a round that moves a hundred pages has to commit a hundred changed lines and name them in its
history file, which is what a pixel round already owes under trap 1 and did by hand. A round that
regenerates without reading the list has written "unchanged" without running anything, and the diff
shows it.

**The two examples stay**, for what the gate cannot do: two uncommitted builds, a scratch tree against
this one, or documents outside the tracked corpus. Their doc comments say so now.

## 7. Where the digests were taken

The committed `raster_golden.tsv` was written at the end of this round, after session 997's
uncommitted rendering sources (`crates/pdf-model/src/{image,content/image,content/transparency}.rs`,
`crates/pdf-render/src/shading.rs`) had been unchanged for five minutes; `doc/history/996` records
the hash of that state. It is byte for byte the file written at `1ea10797` clean: those edits move
no tracked first page in raster, list or report. Where the merge lands a change that does, the
merge regenerates, and the regeneration's printed list is that change's review.
