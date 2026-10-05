# 1507 — A display list is bounded in bytes, and `interpret` takes no clock

Session 1336. Status: accepted and **built**.
Context: CLAUDE.md principle 3 ("Explicit memory and time budgets guard against decompression
bombs, xref cycles, and pathological content"); `doc/todo/49`'s "a count is not a cost";
`doc/todo/10`'s roads A–D and the owner's brief there. Amends nothing; sits beside ADRs 0306
(`MAX_OPERATIONS` counts operators), 0793 (the nesting bound), 0810 (`MAX_TILE_COPIES`) and 1411
(the cycle named).
Code: `crates/pdf-model/src/content/list_budget.rs` (new), the charge sites in `content/marked.rs`
(`Interpreter::draw`), `content/run.rs` (the operator loop), `content/pattern.rs`
(`repeat_cell`), the checkpoint in `content.rs`, `Unsupported::ListBytes` and
`Interpretation::list_bytes` in `content/report.rs`; `viewer-core/src/report.rs` words it.
Instrument: `crates/pdf-model/examples/display_list_census.rs`. Test:
`tests/hostile_budgets.rs` (`a_type3_cycle_through_a_tiling_cell_is_refused_by_its_list_bytes`
and its control).

## 1. What the 3.16 GiB was

The `page` target's out-of-memory input is not a property of the two mutated bytes. Run alone, the
unmutated corpus file `ContentStreamCycleType3insideType3.pdf` (2440 bytes) builds, in a release
build, **3 929 387 commands and 2 047 756 clips** — 472 MB of commands at 120 bytes each and 725 MB
of clips — with a 1.8 GiB peak and 7–8.5 s of interpretation (`display_list_census --stages`,
`VmHWM` reset between stages: the open is 0.8 MiB; everything is the interpretation). The target
interprets every input under 16 KiB twice for its purity check and holds the first while the second
runs, which is the 3.16 GiB. What grows is the display list: glyph `c` of the second Type 3 font
fills with a tiling pattern whose cell shows the first font again, each tiling copy carries the
cell's clip, and **no budget counted a clip** — `MAX_OPERATIONS` charges a copied command as one
operator and sees nothing of the clip beside it. Neither the per-glyph rasters (there are none in
`interpret`) nor the reader's stack (bounded by `MAX_FORM_DEPTH`) take part.

## 2. The bound, and its census

`MAX_LIST_BYTES`, 512 MiB per interpretation. **Asked in the comparison the operator loop already
made**: the loop compares its count with a `next_check` that is never past `MAX_OPERATIONS`, so the
count's bound is reached exactly as before, and every 64 operators the slow path charges what the
list gained — each new top-level command at its own 120 bytes plus the dash array in force (a
stroke clones it; a path is not charged, since its segments each cost an operator and copies share
it), and each new clip with the path it owns, walking the clip table from where the last check
stopped so no route that adds a clip is missed. A tiling's copies are charged as they are made, and
asked before each copy with the last copy's cost, so the copy that would cross the bound is not made;
once spent, every later operator in every enclosing stream stops at once. Reported once as
`Unsupported::ListBytes { charged, bound }`. The charge is a function of the document alone, and
`tests/replacement.rs` holds a page rebuilt from a checkpoint to the same figure.

**What it costs, measured.** The first build charged each mark in `Interpreter::draw` and asked the
bound per operator: callgrind put it at +10.7 k instructions on `xfa_filled_imm1344e.pdf`'s page
(574 commands) and +2.0 k on the launch gate's open of it. The build kept charges nothing per mark
and adds no comparison per operator; the same page counts 5033.5 k against 5035.9 k with the bound
compiled out — inside the noise, nothing — and the launch gate's open counts 1821.46 k against
1821.46 k. That open was 1823.5 k against a 1820 k ceiling when the batch first ran it, and **1821.5 k
with this bound compiled out**: the base commit counts 1819.4 k, and callgrind names the whole
2.06 k — `pthread_getattr_np`, which std's main-thread stack guard calls before `main`, parsing
`/proc/self/maps` one `sscanf` a line, 49 lines where the base had 48. That is the linker's layout
of the test binary rather than anything an open does, so `doc/checks/launch-path.toml`'s ceiling
for that row rises to 1825 k with the reason beside it, and the gate exits 0 at 1821.4 k.

**The four corpora's lists, again, under the charge kept** (891 s, 8.9 GiB peak): the same
largest pages at the same figures — `poppler-12206-0.pdf` 214.3 MiB, `GHOSTSCRIPT-697013-0.pdf`
441.6 MiB and stopped by `MAX_OPERATIONS` — and the cycle the only page refused.

`display_list_census` over the first page of **90 150 documents** (`doc/pdf.js`, `doc/corpora`,
and the safedocs, Tika-tracker and openpreserve caches; 999 s behind the lock, 6.8 GiB peak):
median 117 KB, 99th percentile 1.77 MB, 99.9th 24 MB. The largest list a page builds inside every
other bound is **`poppler-12206-0.pdf` at 214.3 MiB** (957 898 commands, 435 681 clips, nothing
reported); 512 MiB is 2.4 times it. The only other page past 256 MiB, `GHOSTSCRIPT-697013-0.pdf`,
is stopped by `MAX_OPERATIONS` at 441.7 MiB (measured with the bound lifted in a scratch build),
so the bound moves **no refusal any page of the five corpora already had** — the owner's brief in
`doc/todo/10`, that a bound which stops a bomb must not cap a big document, read as a number.
At 512 MiB the cycle stops at 1 762 627 commands and 918 598 clips, 0.82 GiB peak and 2.5 s in
release; the fuzz target's two interpretations take 1.43 GiB and 13.4 s where they took 3.16 GiB
and 26 s. Its raster is byte-identical (`raster_golden`: the list and the report moved, no pixel).
It is drawn and reported in both windows: `quorra` and `quorra-confined` show the page (237 and
210 colours under Xvfb) and print the `MAX_LIST_BYTES` sentence with both numbers, the confined
one per page as it prints every bound. `tools/drive-windows.sh` step 27 had used it as a page the
graphics device refuses; it uses `drive-coverage.pdf`, the page step 28 already used, since the
device now draws the cycle.

## 3. Why there is no deadline inside `interpret`

Considered and declined. A wall-clock check every N operators against a caller's deadline is the
cheap half of `doc/todo/10`'s road A, and it costs the one property the oracle, the fuzz target's
purity assertion and `replace` all rest on: `interpret` would no longer be a function of the bytes,
since the same page would draw a different prefix on a loaded machine. The owner ordered roads
D → B → C there, and B — the confined worker, killed from outside by its `Canceller` and
`REQUEST_TIMEOUT` (ADR 0241) — is where unbounded *time* is ended; ADR 1447's in-process deadline
abandons a *decode* whose output is all-or-nothing, which a display list's prefix is not. So time is
bounded here by deterministic work budgets — operators, list bytes, tile copies, the reach scan —
and the clock stays the host's. What would reopen this: a host that interprets unconfined and
in-process and cannot abandon a thread, with a page that stays inside every work budget and still
takes seconds.

## 4. What is left

The 553 483 text-showing operations of a damaged stream with no font set (the `page` target's other
slow unit, 1.5 s in release and 21 s in the fuzz build) are charged one operator per show and do no
drawing; `doc/todo/49` carries it. Image samples are not charged (they are `Arc`-shared and
`MAX_SAMPLES` bounds a decode).
