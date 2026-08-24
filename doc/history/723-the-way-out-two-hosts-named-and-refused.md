# 723 — The way out two hosts named and refused

Ninth merge round of the block. Four branches, three blocked by a stray whitespace edit on `main`
and then clean, and a batch in which **two rounds found the instrument they were sent to read was
miscounting**, one found a defect against a principle this project states in `CLAUDE.md`, and one
re-measured a road's purpose before building on it.

## The sequence, whole, on a quiet machine (load 1.31)

Both workers built first. `fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 ·
the fuzz check, exit 0 · `nextest` **2585 passed, 18 skipped** · conformance **192** + 5 + 1 + 1 ·
`cargo deny` all four ok · corpus **974 documents, 67 incomplete** · oracle **1945 pages — 983
agrees, 65 contradicted, 832 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18
no render** · `render-quorra` **933 agree, 22 differ, 2 refused** · both censuses ·
`fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. §5's binaries rebuilt and installed. Ledger
**445 implemented, 223 partial, 0 unreviewed**.

Both new instruments verified on the merged tree: `--bin unpriced` prints **no contradicted page
sitting in no note**, and `tools/state.sh windows` prints its sixteen-row reading.

**The merge itself is worth one line.** Three branches were blocked by an uncommitted change to
`doc/conformance/ledger.toml` on `main` — **two blank lines**, whitespace-only, the file parsing
identically before and after. 719 merged anyway because it deliberately touched no ledger row. A
stray edit in a shared tree stops four rounds' work as effectively as a real one.

## 721 — the way out that two of three programs refused

`CLAUDE.md` §3: *a document's restrictions are the reader's to set, and **it shall always be
possible to turn them off***. It held in **one program of three**.

`Command::Restrict` reached one window. In `pdf-viewer-gtk` and `pdf-viewer-qt` a document's
restrictions could not be turned off at all — **while both answered every refusal with a sentence
naming `--ignore-restrictions`, a word their own parsers rejected with "is not an option this
program has".** Each told a person the way out and then refused the way out. Two ledger rows said it
could be turned off; true of `viewer-core`, false of two of the three things a person runs.

**And the round could not sort its list until it fixed the instrument.** `names_in_code` grepped
`Command::[A-Za-z]+` with **no word boundary**, matching the tail of `PathCommand::Close` — a
*path* close, which `viewer-ui` writes on every rounded rectangle of its own chrome — and the
population included the trace formatter, which matches `Command` exhaustively in order to *print*
names. The neighbouring section's own comment names that as its reason for asking one crate only;
this section was written sixty lines below it and asked anyway. **The condition was documented and
not applied, in one file.** So `viewer-ui reaches 25 of 25` and `every Command reaches at least one
window` were both false, and trap 11 gains a seventh and eighth instance.

The count now carries **the reading**: fifteen rows, one per unreached variant, `debt` or `not a
debt` with the reason, checked in both directions — `UNREAD` for a gap with no reason, `SPENT` for a
reason whose variant is now reached. That is what an uninterpreted count could not do, and two
rounds had read "eleven queries" off the old one and walked past.

**One honest limit, stated rather than dressed:** both status widgets cut the refusal sentence
before its tail, so both set a tooltip — *checked in the code, not in a picture*, because there is
no window manager here and a GTK tooltip needs a crossing event this environment cannot deliver.

## 719 — the ceiling is no longer what catches the bomb, measured before anything was built

`VmPeak` off the live confined worker against the 4 GiB ceiling, with both bombs rebuilt from
`doc/todo/10` §2's description **to the byte** (389 317 and 1 847 467):

| | `VmPeak` | of the ceiling |
|---|---|---|
| worker started, no document | 147 568 KB | 3.5% |
| an ordinary document | 147 568 KB | 3.5% |
| Bomb A (0.39 MB → 400 MB) | 147 568 KB | 3.5% |
| Bomb B (1.85 MB → 1.9 GB) | 147 568 KB | 3.5% |

Identical to the kilobyte, reached during start-up. **Road D moved this item's purpose**, and the
argument road B now rests on is stated rather than assumed: the ceiling is a backstop for the
read-whole paths and for what nobody has thought of.

**Asking what *does* still reach it produced the finding.** `VmPeak` was start-up size plus
**exactly three times the document's length, to the kilobyte, at every size measured** — frame
buffer, decode copy, and `pdf_syntax`'s `Arc<[u8]>`. A 1.4 GB document sat at **99.7%**; 1.5 GB
killed the worker. And the breach was worse than recorded: stderr inherited from the host means a
pipe gets `killed by signal 6` while a **file** gets `signal 25` — `SIGXFSZ`, because `RLIMIT_FSIZE`
is 0 — and the worker says **nothing**. A logged deployment got the wrong cause and silence. Trap 18.

Fixed: the payload dropped at the decode (three copies → two, again exact — 1.4 GB doc 4 183 592 →
2 816 412 KB), a message budget **derived from the worker's own ceiling** with every term read or
measured, `try_reserve` wherever a stated length becomes an allocation — *the host had no guard at
all against a subverted worker's 2 GiB claim* — and the worker's stderr as a drained pipe carried
into `WorkerDied`. 1.5 GB now opens at 71.8%; 1.6 and 1.9 GB are refused **by name** with `VmPeak`
unmoved, and the worker goes on to open two more documents.

**The tier change is priced, not made**, which is the right call: confined rendering roughly doubles
a small document's cost and 41–53 ms of a large one is the document crossing. What it waits on is
`doc/todo/34` §2, still unargued.

## 722 — the sixth criterion answered pool-wide, and the instrument that answers it

Not a seventh criterion — ADR 0497's **sixth**, pointed at the whole pool. It has a precondition
nobody could evaluate at scale, and **five rounds closed with it owed**: which of the gate's four
bounds does each contradicted page fail on, and does its note name that measure?

`--bin unpriced` is that precondition, and it rasterises nothing — which bound fails is
`Tolerance::accepts`' arithmetic on the gate's own printed line. Restricted to `CONTRADICTED` by
trap 11, since on an `ambiguous` page the bound decided nothing.

Two findings from pointing it at the pool. **`CONTRADICTED_TIGHT_CONSENSUS` names one measure in 160
lines and it belongs to one of its three pages** — re-argued in the bound that actually fails, **two
renderers that are not us fail it too, and both by more**, so taking us out of the room does not
rescue it. And **one page's verdict rests on six channels of eighty thousand**: at the two decimals
the gate prints, `differing 6.55%` against `bound 6.55%` are identical, and at full precision it is
5244 channels against an allowance of 5238. It stays contradicted; what is new is that **a page's own
line can stop being able to say what its verdict rests on.**

**And it corrected two instruments.** 69 page names were invisible to the nineteenth and twentieth
sweeps, because a filter rejecting a `.pdf` token preceded by `/` was correct when written and
**overtaken one round later** by ADR 0541's corpus labels — where the label *is* the identity,
precisely because bare names collide. `overtaken` 320→340, `quoted` 86/21→91/13, and contradicted
pages held by no note **5 → 0**.

## 720 — the rule took a round out of §12.5, and the report that was narrower than the drawing

716's corrected rule worked: §12.4.4 ~ §12.4.4.1 at 34 shared sequences, the strongest pair below any
clause-level parent, **the first round in four to leave §12.5**. And it sharpened why:
self-reinforcement is a property of the *family*, so a tie-break reordering pairs inside the head
family could never escape it.

**A code defect, trap 5's exact shape.** `note` took a `&Style` where `frame` takes a `&Transition`
and refuses on the direction for four travelling styles — so a `/Di` outside the four quarter turns
**shaped no frame and produced no sentence**. A cut in silence. Two doc comments asserted the
property that had failed, one claiming the report existed since session 393. Fixed by asking the
*same expression* `frame` refuses on rather than a second list, held by a property test over 13
styles × 7 directions.

Then measured for consequence: over the crawl, of **464** transitions on those four styles **every
one** states 0, 90, 180 or 270 — so the widened report fires on nothing that exists and can fire on
no conforming file. The crawl's only unrecognised `/S` is the **empty name**, on 106 pages.

**Two accepted errata that cannot both be applied** — one rewriting Table 161's range as an odometer,
one inserting the repeating form after the same clause, both `Review/Accepted`. Our code follows the
published sentence's own count. The rule now heads `doc/errata-read.md`: **an erratum is evidence
about the standard in the way another renderer is evidence about our reading.**

**And a hazard that cost twenty minutes**: `git checkout -- doc` destroys a parallel worktree's
submodule symlinks, `git status` says nothing, and the symptom advises a command that would clone a
second copy rather than restore the link. Now beside the `git add -A` rule — and every sweep was
re-run with the links restored, byte-identical, so no number is affected.

## Owed

- **`AccessibilityNode::lines` does not cross the ABI**, still open; 721 did not verify its "two
  accessors" claim because the sort turned up a debt that outranked it.
- **`doc/todo/34` §2** — display lists across the confinement boundary, or the device inside it —
  which road B now waits on.
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
