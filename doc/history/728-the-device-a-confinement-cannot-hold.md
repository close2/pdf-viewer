# 728 — The device a confinement cannot hold

Tenth merge round of the block. Four branches, and a batch in which **three rounds found a defect in
an instrument this project trusts** — the oracle's consensus, the round procedure's own §5, and a
host's geometry — while the fourth settled a design question by making the losing option fail in
front of it.

## The sequence, whole, on a quiet machine (load 1.40)

Both workers built first. `fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 ·
the fuzz check, exit 0 · `nextest` **2595 passed, 18 skipped** · conformance **192** + 5 + 1 + 1 ·
`cargo deny` all four ok · corpus **974 documents, 67 incomplete** · oracle **1945 pages — 983
agrees, 65 contradicted, 832 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18
no render** · `render-quorra` **933 agree, 22 differ, 2 refused** · both censuses ·
`fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. §5's binaries rebuilt and installed. Ledger
**445 implemented, 223 partial, 0 unreviewed**; `Query` 31 of 31 at **172 entry points**.

## The merge round's own finding: the ledger command is a formatter

Three branches were blocked twice by an uncommitted change to `doc/conformance/ledger.toml` — two
blank lines, whitespace-only. It was not a stray edit. **`cargo run -p conformance --bin ledger`
writes the file back in clause order** — its own module comment says so — and every merge round runs
it to read the counts. Two rows, §10.7.4's and §11.6.2's, had been hand-edited without the blank line
the writer puts between rows, so the tool restored it every time and the tree went dirty.

Committed the writer's own output, which is canonical by construction, and the command now leaves the
tree clean. **An instrument that writes is an instrument whose output belongs in the commit** — and
the tell was that the same two lines came back after being discarded.

## 724 — the option that does not exist

`doc/todo/34` §2 offered two boundaries and road B waited on the choice. The round expected to argue
against `wgpu` inside the confinement on price; **it never got that far.** A real Radeon 890M device
brought up and then confined, each stage in a child because the seccomp action kills:

| stage | outcome |
|---|---|
| bring up, draw, draw again, unconfined | drew |
| bring up, draw, confine, report | **confined, landlock unavailable** |
| bring up, confine, then draw | **SIGSYS** |
| bring up, draw once, confine, redraw the *same* frame | **SIGSYS** |

The killing call is the first after the filter is installed — `DRM_IOCTL_AMDGPU_GEM_CREATE`. **No
ordering helps: a device is a conversation with a kernel driver.** And the second row is a cost the
others hide — a process holding a device holds **9 descriptors against the confinement's ceiling of
8**, so `landlock_create_ruleset` fails `EMFILE` and the depth layer is lost *before* the filter is
reached.

The surface is counted rather than described: **55 distinct syscalls in a bring-up, 35 off the
interpreter's 28-call allow-list**; `/dev/dri/renderD128` across **25 distinct DRM request numbers**
at ~190 ioctls a frame; the shader cache read *and written*; an `AF_UNIX` socket to the X server on a
headless run. Seccomp cannot dereference an ioctl's argument, so **admitting those 25 numbers admits
whatever they can be made to say.**

So: **display lists, with the raster as a per-page fall-back chosen by size.** At 72 dpi — the list's
*worst* case, since a list is scale-invariant and a raster quadratic — the median list is 0.034 of
its raster and 153 of 958 pages exceed theirs, the tail being one ordinary population: **a scan's
decoded samples are its display list.**

**And the largest win is not on the launch path at all**, which corrects the briefing: under today's
confined path a smooth zoom ships a 4 MB raster *per frame* — 245 MB/s of a measured 1.04 GB/s
transport, 3.9 ms of latency each — and under display lists the host holds the `Arc` and that traffic
is gone. Two constraints found with numbers: the codec **must** preserve `Arc` identity (flat, the
corpus goes 0.37 → 0.91 of its raster), and two carriers hold trait objects that cannot cross as they
stand — 4 pages of 958, which the raster arm already covers. The codec was **not** built, and the
round says why rather than implying it ran out of time.

## 727 — agreement is not transitive, and the instrument reported one of two

**`pdfref::decide` counted one maximal agreeing set where a page can have two.** With three
references, `a ~ b` and `b ~ c` while `a ≁ c` leaves `{a,b}` and `{b,c}` — neither contained in the
other, neither a majority the other is not — and the subset loop discarded the second without
counting it. **The survivor is the one whose bitmask is smaller: the order the variants happen to be
declared in.**

**41 pages of 1945 carry more than one maximal consensus, and on 4 the sets disagree about us.** All
four are contradicted; **all four would have agreed under the set thrown away.** On one of them our
raster is **byte-identical to `ghostscript`'s** over the whole 595×842 page — a consensus that accepts
us by identity was available and unreported. And the group named for a *tight* consensus is decided on
two of its three pages **by a pair that is not the tightest on the page**, which answers a lead from
the previous round's own table that nobody had read.

**Nothing moved**, and that is the round's judgement rather than its limit: the third page goes the
*other* way — rescued by the pair that agrees least — so no single rule follows, and replacing an
enumeration order has three order-independent candidates each with a hazard. It gets its own round.
All 1945 per-page lines byte-identical; every failing bound in the pool still named.

**It also refused to file this under trap 9**, correctly: nothing is wrong with the two references
agreeing — the point is that there were *two* agreements and the instrument reported one.

## 726 — a document that decided a window's width, and §5's own hazard

Three debts from 721's reading, and the deliverable is `viewer_host::popup` rather than the widget
code: §12.5.6.2's `/T`, Table 166's `/M` through one `stamp`, `/Contents`, the upright box, and one
refusal. Photographed in **all three** hosts with the same title bar, the same date and the same
wrapped text. Thirteenth consecutive host item needing **no new message**.

**Trap 19, and only the screen could say it**: a `GtkFixed` *measures* its children, so a document
with popups placed beside its page let the document decide the window width — **509 → 1229 device
pixels in nine frames, with nothing on screen looking wrong.** Qt's equivalent was right by accident.
And **trap 11 caught the round's own report**: its first trace line fired on windows *placed*, so
zero-area windows printed the same silence as none.

**The hazard that belongs to this project's process rather than to its code**: `doc/todo/02` §5 named
a **literal build directory** — the main tree's — so a worktree round following the instruction
installs *a neighbour's binary*. The round rebuilt, installed and ran the GTK host three times seeing
nothing for a feature that worked. That is trap 15's subject reaching a round **through an
instruction**, and it has been in the round procedure for every parallel batch. §5 derives it from
`cargo metadata` now.

Two claims corrected: the `Popups` debt reason said "seven of the corpus's documents" where the
measurement is **seven open popups on two documents**, and 709's "two accessors" for the accessibility
lines is **three**, because the ABI asks a count before an indexed accessor and a line has two counts.

## 725 — a row that argued from correct facts to the opposite conclusion

The pair rule sent it to a family no round of this method had opened, and **§10.7.5's row said a
`shall` was unpaid that ADR 0285 had paid**. The row narrates the original measurement correctly and
then concludes the wrong way; four witnesses say otherwise, including **`doc/todo/11` — the pointer
the sentence itself ends with** — which heads that item *closed*. Its two stated reasons for not
paying are the two the deciding ADR resolved the other way. **No sweep here can print it: the defect
is a conclusion.**

**Two places counted `/SA true` and disagreed** — 49 and 30, neither naming a population or a
command. A name census cannot arbitrate, because the clause fires on the *value* and `/SA false`
states the entry too; asked by value: **50 of 974, 60 of 1251 curated, 15 207 of 65 944 crawled**,
with a neighbouring recorded figure reprinted exactly as a backwards control.

And **a wrong table number in a form the ninth sweep counts instead of printing** — the cited table
states no entries, so it lands in the *keyless* count rather than among the absences. Calibrated over
three states of the cell to show the instrument sees it once corrected.

## Owed

- **The display-list codec**: a two-sided encoder, a fuzz target, and the deferred-producer question
  — road B's next step, now that the boundary is decided.
- **The consensus enumeration order** (727): three candidates, a hazard in each, its own round.
  `doc/todo/12` carries it.
- **`issue6069.pdf`**, still the one page whose printed line cannot say what its verdict rests on.
- **§14.7's tree on the native hosts' own accessibility interfaces**, the largest remaining UI debt.
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
