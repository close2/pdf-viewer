# 758 — The gate that was the table's own inverse

Sixteenth merge round of the block. Four branches, **no conflicts**, and a batch in which every round
found that something this project had *checked* was checked against itself.

## The sequence, whole, on a quiet machine (load 1.56)

Both workers built first; §5's binaries installed from the directory `cargo metadata` names. `fmt` ·
`clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 · `nextest`
**2676 passed, 18 skipped** · conformance 192 + 5 + 1 + 1 · `cargo deny` all four ok · corpus **974
documents, 67 incomplete** · oracle **1945 pages — 983 agrees, 61 contradicted, 836 ambiguous, 3 our
geometry, 2 reference geometry, 42 not comparable, 18 no render** · `render-quorra` **933 agree, 22
differ** · both censuses · `fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. Ledger **445
implemented, 223 partial, 0 unreviewed**.

## 755 — the fourth step, and a gate that could not fail

750 left the errata rule owing a fourth step: **the ranking excluded `implemented` rows and the true
head was inside them.** Corrected, and the correction justified itself at once — the head over every
row is **§D.3 with fifteen annotations**, more than double the live head's seven, and it is
`implemented`.

**Its argument for how to rank them is the durable part.** A live row's count ranks a debt the ledger
already declares; a settled row's count ranks a **claim**, and `CLAUDE.md` says a claim decays. But
ordering settled rows above live ones outright would discard the count, which is the whole
instrument — so one ranking, tie to the row that asserts more. Step 2 gains a **writing rule** rather
than a third grep, since a bare-number search collides with another project's issue list.

**The finding is that the annex's only gate was the table's own inverse.**
`every_text_string_survives_the_round_trip` searches the array `text_string` indexes, **so a code
transcribed with the wrong character round-trips perfectly** — 232 mappings with no assertion beyond
seven spot-checked codes. Issue #461 names the exact mistake it would have missed. Planted: all ten
of the module's tests stay green. There is now a test of all 256 rows against `doc/md/`, decode and
encode as separate statements — **every row agrees, so the defect was the gate rather than the
table.**

**A new blindness, and a circular one.** *An erratum can correct a column of the standard that no
reader reads*, so `check` and `applied` are both silent by construction and what the erratum is
evidence of is the weakness of the row's gate. And the three glyph carets' contents are single
`PDFDocEncoding` bytes, **so `emit` renders the correction through the table it corrects** — reading
those errata with that tool is circular, and the annex's Unicode column is what closes it.

## 756 — the border, and three documents that agreed with each other

**§12.5.4 has no width-1 case.** The only place the subclause names 1 is Table 168's default `/W` —
how *wide* a border is, not where it goes — and a stroke straddles its path, so a border whose ink is
entirely inside `/Rect` is inset by half its width **at every width**.

**We are right, and at width 10 it is plain rather than arguable**: on a synthetic `/Border [0 0 10]`
ours covers exactly the rectangle while `poppler` covers **five units beyond it on all four sides,
exactly w/2**. At width 1 that becomes a snap-to-grid effect where which sides show it depends on
where the edges fall — two of four on one document, **none** on another whose `/Rect` is fractional.
Rounding *on top of* the placement, not a second placement. Over both corpora, on comparisons whose
border colour is the page's own, **`poppler` reaches further outside `/Rect` than we do on three
quarters of them and we reach further on none.** Two instrument corrections were paid for: *equality
not nearness*, after a nearness test called every black glyph a border and **accused us on all 31**.

**And the round's best finding contradicts its own brief: rendering the neighbouring page found a
defect of ours that three of this project's documents denied.** `Border::inset`'s comment, the oracle
group's note and §12.5.4's ledger row all said an oversized border "fills the rectangle solid". It
does not — a `/Border [0 0 112]` on a 150 × 20 `/Rect` drew a **38 × 20 block in the middle**, and a
width past *both* dimensions drew **nothing at all**, because the stroke of a degenerate inset path
loses the two sides that degenerated.

That page's ink goes 29.65 → **67.21** and it enters the *we are alone* list at second — **not a
regression, and the round says why**: `mupdf` and `ghostscript` are nearest by drawing **no border at
all**, so obeying the clause moves us away from a shared gap. The ink sweep agrees, its negative tail
unchanged in names and order.

## 754 — one arrangement in three hosts, and a test that is exact

`viewer-gtk` and `viewer-qt` rasterised on the toolkit's main thread, so a slow page took the window
with it and 749's interrupt could not reach them. **`viewer_host::drawing` is one arrangement shared
by both** — a queue, one job in flight, and a `render-cpu` thread the first page to need one spawns.
**No message, the fifteenth time since session 607.**

Two design points that were forced rather than chosen. **A queue, not one slot**, because a tier-1
host is asked per page and *owes an answer for every one* — `Viewer::schedule` skips a page whose
outstanding request already matches, so a dropped request is a page that never draws again. And **the
wake-up is a pull**, because Qt's C++ owns the `Host` and Rust never calls a Qt object; GTK *could* be
woken through a descriptor and deliberately is not, since the point is one arrangement in three hosts.

**The policy reached further than briefed, and in a sharper form.** `viewer-ui`'s rule is a judgement
about pixels; a tier-1 host holds a **token**, and `Viewer::rendered` drops an answer whose token is
not the one outstanding — so *is this answer still wanted* is **decidable**. Abandoning is therefore
**provably free of trap 20**: it only ever discards work the core would have discarded. `Finished::outcome`
is an `Option<Rendered>` so the call site must *state* trap 20 rather than remember it.

Screen evidence: with a 1000-fill fixture drawing, three zoom-ins 700 ms apart — GTK drew 4.842 s and
**abandoned the next two after 737 and 699 ms**, Qt after 702.6 and 702.7. Handoff **2.16 ms (GTK) /
2.13 ms (Qt)**, two toolkits agreeing to 0.03 ms; `POLL` measured rather than picked at 6.9 ms median
page-turn gap against 15.0 ms at one refresh.

**And it refused to average over a measurement it could not take**: the launch A/B drifted an order
of magnitude *in both arms* at load 25–40, so the structural cost is recorded and **a quiet-machine
launch A/B is owed** — said in the ADR and the todo file rather than smoothed.

## 757 — the composition that decayed faster than its total

`doc/todo/42` §1 has said since ADR 0180 that `Document::open` costs 76.6 M "of which inflating the
two cross-reference streams is 18 M and nothing can remove it". **Session 752 re-took that sentence's
total four rounds ago and left its composition alone** — and re-run, the ranking is **inverted**:
`xref::read_section` is **37.0%** of the open against the predictor's 15.2% and zlib's 7.6%. *The part
the sentence called irreducible is under a quarter.*

A temporary `#[inline(never)]` attributed it exactly: **`entry_location` was 18.4 M per open — a
quarter of `Document::open` — to read three integers out of a seven-byte record**, 164 instructions
apiece, re-deriving Table 18's three field offsets for every one of 101 318 entries. §7.5.8.2 says
`/W` describes the *stream*, so the offsets resolve once.

| | before | after | |
|---|---:|---:|---|
| `callgrind_open`, ten opens | 678 200 421 | 607 023 715 | **−10.49%** |
| `callgrind_interpret`, p101 ×50 | 1 285 546 279 | 1 278 428 629 | −0.55% |
| `callgrind_rasterise`, p101 | 5 431 961 793 | 5 420 432 898 | −0.21% |

`read_section` drops 71 176 710 against a total delta of 71 176 706 — **the attribution is the whole
change**, every other row unchanged. Byte-identity over 200 000 generated cases with three plants
catching 4 440 / 833 / 2 367 first.

**The rule it earned is better than the fix**: *a composition decays faster than the total it adds up
to*, because a total is what a later round re-takes and a breakdown is not. And **re-deriving the
baselines was load-bearing** — quoting the previous round's figure would have turned −11.5 M into
−29.9 M.

It also **reported its own mistake rather than burying it**: `PDFREF_CACHE` left unset, so three gate
lines rebuilt 360 MB of references beside the shared copy. No verdict affected; the exact fix is
recorded.

## What the batch has in common

| what was checked | what it was checked against |
|---|---|
| Table D.3's 232 mappings | the table's own inverse |
| an oversized border | three of this project's own documents |
| a hot function's cost | a total that had been re-taken while its parts were not |
| a slow draw in a native host | nothing — the window went with it |

## Owed

- **The quiet-machine launch A/B** for the native hosts' drawing thread (754), stated rather than averaged.
- **The errata rule's fifth use**, now ranking over every row.
- **Orca on all three binaries, by a person.**
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
