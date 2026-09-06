# 937: two merges, a launch-path floor nobody may derive, and a font matrix written to two decimal places

## What this round was

A merge round with an improvement lane, run on `main` in the main checkout. It carried
`round-933` and `round-932` onto `main`, ran `doc/todo/02` §2 whole on the merged result, closed
three worktrees, walked `batch5/pdfcpu`, and took the head item that walk produced. A weekly model
limit ended the round in the middle of it and it was resumed on the same tree; nothing on the
machine had failed and no merge was outstanding.

## The two merges

| | | |
|---|---|---|
| `2f077c36` | `Merge round-933` | the annotation catch-all swept across all twenty-eight subtypes, six false refusal sentences corrected, a live watermark departure out of silence, eight ledger rows read, and the modal-verb sweep's second blindness — a requirement stated in the indicative. Carries rounds 928, 930 and 931 through 933's branch |
| `613ce751` | `Merge round-932` | an inline image whose `/W` and `/H` are reals drew a full-bleed cover as a blank page; five call sites unified behind one reader; and a census that finally searches |

**One textual conflict, in `doc/todo/42-the-launch-path.md`.** Sessions 933 and 934 each wrote a
section about the same failure — all four `peak_mib` rows reading *below* their floors on a binary
neither round had touched — and neither had seen the other. Both were kept, in session order,
because they are two independent measurements of one figure and each carries numbers the other
does not; 934's ordinal ("the third round running either") became "fourth", which is what it is
once 933's paragraph sits in front of it.

**One conflict that was not textual and was the real one.** `doc/checks/launch-path.toml` merged
clean and silently took session 932's lowered `peak_mib` floors — 95, 100, 100 and 112 where
session 931 had derived 127, 131, 132 and 143. The merge did not take them. The two rounds
measured the same figure on the same binary and disagree: 932 read 99 to 116 with the machine
pressed and 19 GiB in swap, 934 read 161 to 182 on nine runs with 29 GiB free, and 934's ADR 0909
explains both — what falls is the resident set of a process that has brought the graphics device
up, and it falls under *memory pressure*. A floor lowered to admit those readings writes the loaded
machine into the claim, which sessions 931, 933 and 934 each declined to do. So the merge restored
931's figures, rewrote that file's paragraph to record **both** observations and the disagreement
rather than one round's conclusion, kept 932's numbers and its `Q32`, and derived nothing.

**The gate then said the restoration was right.** On the merged tree the launch line read
`peak_mib` 174, 178, 179 and 190 MiB — two of the four judged, both inside the restored bands, and
the other two declined for clock reasons on a machine two rounds were building on. Nine figures not
judged, seven outside, none failing; exit 0.

**And the owner has since answered `Q32`.** `A32` is *recommendation approved*, and that
recommendation is to take the floor off `peak_mib` altogether and put one on `open_peak_mib`, which
has no graphics device in it. So the figure this merge held open is settled by the owner rather than
by another measurement; `doc/todo/42` says so, and the round that lands `round-935` — whose branch
was cut before both the merge and the answer — applies it.

## The gates

The full `doc/todo/02` §2 sequence ran **twice**: once on the merged result before the improvement
lane, and once after it, because a change in `pdf-model` is under everything and a merge runs the
sequence anyway. Both green, every one of the twenty-nine lines at exit 0 both times.

| | first run, the merge | second run, with the change |
|---|---|---|
| `cargo nextest run --workspace` | 3329 passed | 3330 passed |
| `--test corpus` (doc/pdf.js) | 974 documents, 64 incomplete | 974 documents, 64 incomplete |
| `--test oracle` | 1945 pages, 1841 complete | 1945 pages, 1841 complete |
| `--test fixed_documents` | 76 rows, 0 absent | 77 rows, 0 absent |
| `render-quorra --test corpus` | 929 agree, 22 differ | 929 agree, 22 differ |
| `cargo test -p conformance` | 222 passed | 222 passed |

The new row is `pdfcpu-52-2.pdf p1 ink 4.513 reports 22`, and the second run's launch line read
`peak_mib` 172, 176, 175 and 188 MiB against the restored 127/131/132/143 floors — the third
independent confirmation this round that the figures session 932 read were the machine and not the
program.

**One thing this round got wrong and caught within seconds**: it started its own walking gate lines
while round 939 was inside `foreign_corpus`, which is two corpus walks on one machine. Stopped by
process group — never `pkill -f` — confirmed gone with a second `ps`, and restarted after a wait
loop on `/proc/PID/exe`. The check has to happen *immediately before* the launch and not five
minutes earlier, which is what a wait loop is for and what a glance is not.

## The worktrees

`r927`, `r928` and `r930` closed, checkout and build directory together — their branches were
ancestors of `main` after the merges and no running round had branched from them. `r931` was
already gone. `r932` and `r933` were left, because `round-936` and `round-935` branched from them.

## The improvement lane

`corpus-cache/tika-issue-tracker/batch5/pdfcpu`, 100 documents, the largest of the fifteen unwalked
trackers. `doc/todo/03` §50 has the survey line, the ranking and the finding; ADR 0914 has the
reading and ADR 0915 the instrument correction. In short:

- **The ranking's head was a held population** — `pdfcpu-90-0.pdf` at −1.259 levels, nine
  non-embedded `Arial` faces, `doc/todo/21`'s standing population for the third directory running.
- **The finding was four rows down and the measure had hidden it.** `pdfcpu-131-0.zip-0.pdf` scored
  0.000, *inside* the interval, on ours 0.000 against `pdftoppm`'s 154.735 and `mutool draw`'s
  0.000 — because `mutool` refuses that page and exits 0, and its blank sheet was the interval's
  floor. `doc/oracle-and-corpus.md` §3d already says a sheet of zero ink is not a page; nobody had
  said the rule binds the ranking's **endpoints**. ADR 0915. That page, opened, is poppler's own
  defect rather than ours.
- **What the corrected list left was three `NoninvertibleMatrix` pages, and they are one producer
  bug**: fifty-seven Type 3 fonts stating `/FontMatrix [0.00 0 0 -0.00 0 0]` — a 2048-unit glyph
  space written to two decimal places, §8.3.4 NOTE 3's own all-zero example reached by rounding.
  1737 marking commands collapsed to points, 966 of one page's 1358, and the page told its reader
  about a *matrix* rather than that its **text** is not drawn. ADR 0914 refuses the font on §9.2.4's
  `shall`, and the raster does not move: ink 1.58783, 1.36211 and 4.5126 before and after, to every
  digit.
- **`doc/todo/11` item 8 said its witness did not exist, and that was false.** Over the 89 286
  documents on this disk, page one alone, 465 985 such marks on 35 documents, four of them above
  39 000 apiece. Corrected there, and in §8.3.4's ledger row, with the part that still holds kept:
  not one of them carries a shading, which is the paint the inverse positions.
- **The population the fix reaches was measured twice and the two agree.** The census re-run over
  exactly those 35 documents afterwards: **3 gone, 32 left, 464 248 marks**, every other count
  unchanged to the mark and none of the four large ones a font matrix — so item 8 keeps its
  witness and this is not a question closed by removing what asks it. And a byte scan of every PDF
  on the disk for the *cause* — a `/FontMatrix` whose `a·d − b·c` is zero, over the raw file and
  every `FlateDecode` stream — finds five documents, of which two are the scan's own false
  positives: a Ghostscript manual that *prints* `/FontMatrix [0.001 0 0 0.001 0 0]` as page text
  between `Tj` operators, and a file whose fourth number carries a corrupt byte on a page that has
  no content stream at all. Three of 89 286, one producer.
- **And the scan itself needed the memory rule.** An uncapped first attempt, six workers
  decompressing whole streams, reached `tools/bounded.sh`'s 12 GiB tree ceiling in 319 s and was
  killed by it — which is the wrapper doing exactly its job. Re-run at four workers with a 16 MiB
  cap on any one decompressed stream it costs 527 s and 0.59 GiB.

## What arrived while this round was stopped

The owner answered **twenty-eight** questions; the `A` files were untracked on the machine and are
committed here as their own commit, with `doc/.gitignore` (they bought two parts of ISO 19005 and
put them in `doc/pdfa/`, which must not be checked in). `A33` and `A35` approve the recommendations
rounds 933 and 934 wrote — a movie annotation's poster image and a font a file does not carry —
and acting on either is a later round's. `A25` and `A28` ask for changes to `CLAUDE.md`'s wording
and to how an instrument's counter is switched off; `A27` and `A31` approve their recommendations,
`A31` adding that a warning should be emitted where that is easily possible.

## CI was red on `main`, and it was three things none of which was either merge

`tools/round.sh` says whether CI's last run on `main` passed, and after the merges it said
*failure*. Read, the run had **three** failing jobs and the last green run on `main` was a hundred
sessions earlier — so this was accumulation in the three jobs `doc/todo/02` §2 does not contain,
not something a merge brought. All three are fixed here and all three were reproduced locally
first (trap 13):

| job | what it said | what it was |
|---|---|---|
| `deny` | `error[yanked]: detected yanked crate (try cargo update -p wnaf)` | `wnaf 0.14.0` was withdrawn upstream after it was locked. `cargo update -p wnaf` takes 0.14.1, one transitive patch bump under `primeorder`; `cargo deny check` then reports `advisories ok, bans ok, licenses ok, sources ok` |
| `build (Windows)` | `error: field 'program' is never read` in `confined-transport` | the only reader is inside a `#[cfg(unix)] impl Source`, so the field is dead off Linux and the workspace's lints are errors in CI. `#[cfg(unix)]` on the field, matching `not_a_socket` directly below it — **not** `#[expect(dead_code)]`, which would be unfulfilled on Linux and fail the other way. Third instance of ADR 0194's shape |
| `nightly` (Miri) | `error: unsupported operation: mkdir not available when isolation is enabled` | four file-system tests in `pdf-syntax::file`, and **one unsupported operation aborts the whole run**, so they took 209 passing tests with them and the job said nothing about aliasing at all. `#[cfg_attr(miri, ignore = …)]`, the idiom `filter.rs` already uses; `cargo +nightly miri test -p pdf-syntax --lib` now reports 110 passed, 16 ignored, exit 0 in 674 s |

`doc/verify.md` carries all three beside the instruments that see them, with the rule each leaves —
the sharpest being that **a round adding a test which opens, creates or removes a file owes the
Miri attribute, and nothing local will tell it so.**

The six cross-target checks were then run in full, both targets and all three package sets, and are
green. The `doc/todo/02` §2 sequence ran a **third** time for these changes, because
`confined-transport` is under `viewer-confined` and `pdf-vfs` and `pdf-syntax` is under everything.

## What is left

- Applying `A32` to `doc/checks/launch-path.toml`, which belongs to whoever merges `round-935`.
- `batch5`'s remaining unwalked trackers, all smaller than this one.
- §10.7.4's mark for a shape its *transform* collapses — `doc/todo/11` item 8, now with a witness
  population and with the observation that the references split three ways rather than sharing a
  gap: `pdftoppm` paints a pixel at each collapsed point, `mutool draw` and this tree paint none.
- **CI's three jobs are fixed but not watched.** They are red the moment something upstream is
  yanked or a test touches the file system, and nothing in `doc/todo/02` §2 can see any of it —
  which is how a hundred sessions passed with `main` red. `tools/round.sh` does print it, and
  reading that line is the only guard there is.
