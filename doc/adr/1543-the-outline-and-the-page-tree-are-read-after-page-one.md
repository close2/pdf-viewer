# 1543 — The outline and the page tree are read after page one, on a thread the host chooses

Status: accepted. Session 1354. Reverses `doc/todo/42` item 2's "closed by ADR 0182's structure":
that item allowed the outline to be eager because something slower ran beside it, and said "[i]f
the device ever became free, this would be back". Amends ADR 0890 in one respect: the opening
announcement no longer builds `Open::page_indices`. Supersedes nothing.
Context: `CLAUDE.md` principle 2 ("[n]othing eager … no full page-tree walk … Anything not needed
to show page one is deferred until first use"); ISO 32000-2 §12.3.3 (the outline), §7.7.3 (the
page tree), §12.4.2 (page labels), §12.3.2.1 and Table 29 (`/OpenAction`), §12.6.3 (`/O`, `/PO`);
`viewer-core` rule 4 and `doc/todo/49` ("no threads the core was not handed, and no blocking");
ADRs 0182, 0247, 0260, 0890, 1044, 1531, 1544; traps 50, 98, 104.
Code: `crates/viewer-core/src/open.rs` (`Open::document` an `Arc`, `Open::outline` a `OnceCell`,
`Open::outline()`, `Open::page_indices()`, `Open::section_at`, `Preparation`, `Prepared`);
`crates/viewer-core/src/viewer.rs` (`Viewer::preparation`, `Viewer::prepared`,
`announce_page_reading`, `Query::Outline`); `crates/viewer-ui/src/bin/quorra/{window,dispatch,app}.rs`
and `quorra.rs` (the preparation thread, `App::preparing`, `take_the_preparation`);
`crates/viewer-ui/tests/launch_path.rs`; `doc/checks/launch-path.toml`.
Tests: `crates/viewer-core/tests/preparation.rs`'s three.

## 1. What the open read that page one did not need

`launch_path`'s counted open — `Command::Open` exactly as `quorra` sends it, under callgrind,
cleared environment — by function on ISO 32000-2, 185.1 M instructions in all:

| | M instructions | needed for page one |
|---|---|---|
| `Open::read`, §7.5's cross-reference | 57.2 | yes |
| `Outline::read`, §12.3.3's 988 items | 35.5 | no |
| `announce_page`: `Pages::indices`, every node of §7.7.3's tree, for the caption's section | 65.1 | no |
| `section_at_with` | 0.8 | no |
| dropping the two with the viewer | 15.7 | no |
| `Pages::new` (the count; `/OpenAction`'s page) | 3.8 | yes |
| `PageLabels::read`, §12.4.2 (page one's caption label) | 1.6 | kept: 0.9% |
| `Destination::open_action` | 1.2 | yes: it chooses page one |
| `page_events`, §12.6.3's `/O` and `/PO` | 0.2 (8.0 on WTPDF) | yes: the clause orders them after the open action; the 8.0 is the object stream holding page one's annotations, which the anticipation reads next |

`ViewState::of` (the optional content configuration page one is interpreted under) and
`Opening::read` (Table 29's layout, one name) are below a tenth of a million. **The outline and the
whole page tree are 100.6 M of 185.1 M, and the caption's section is the only reason either was on
the open.** On a cold page cache they are also the reads: 4320 KiB in 1075 calls, against 102 KiB
in 27 without them, because 988 items and 1023 page nodes live in object streams across the file.

## 2. The decision

`Command::Open` reads neither. `Open::outline` is a `OnceCell`, `Open::page_indices` already was,
and the opening `Event::PageChanged` names a section only where both are already read (the cost of
the opening caption is its section, for the moment the next paragraph takes to read it). Every
other reader is a first use and reads on the thread that asks: `Query::Outline`, a page turn's
caption, a link's or a fragment's page change.

**So that no first use is on the thread that draws, a host may read them beside it.**
`Viewer::preparation` hands out a `Preparation` holding a second `Arc` of the immutable document —
`pdf_syntax::Document` is `Sync` since ADR 0260 — and `Preparation::run` reads the outline and
places the tree on any thread, touching none of the core's state; `Viewer::prepared` takes the
answer and raises `PageChanged` again where the page has a section. Rule 4 holds as written: the
core spawns nothing and waits for nothing, and what crosses is a handle to a file and a value. The
answer names its file, not only its id (`Arc::ptr_eq`), so one about a document closed and reopened
under the same id is dropped — trap 104's lesson applied before it could bite.

`quorra` takes the preparation at the join, before the opened document's events are received,
reads it on a thread that wakes the loop when done, and holds the panel's outline until it lands
(`App::take_the_lists` does not ask for it meanwhile, or it would be read on the drawing thread in
front of page one). The gate's first-page phase does the same and joins that thread after its clock
has stopped (ADR 1544 prints when it finished); its page-turn phase runs the preparation before
turning, because a person's first arrow key in `quorra` finds it done.

## 3. What it costs and who still pays

- The caption's section arrives a few milliseconds after page one rather than with it — 0.2 to
  11.9 ms after the join on the five rows; a document with no outline loses nothing.
- `quorra-gtk`, `quorra-qt`, `quorra-confined` and the C ABI do not yet take a preparation (round
  1352 had those crates). For them the open is cheaper by the same 100.6 M, the opening caption has
  no section, and their first page turn — or their panel's `Query::Outline`, whichever is first —
  reads the outline and the tree where the open used to. Taking the preparation is three lines in
  each host and `doc/todo/42` names it.
- `Open::document` is an `Arc<Document>`; every `&open.document` still coerces to `&Document`.

## 4. Measured

Both arms built once in the batch's target directory and copied aside before and after the change
(md5 `5b4f3587…` and `a49ab8bb…`, trap 50); three gate runs of each, alternated, release, pinned,
2026-10-05, load averages 1.4 to 2.5. Instructions are the counted open (identical to a hundredth
of a per cent across runs); clocks are each run's minimum of nine children.

| row | instructions, thousand | read | cold open, ms | warm open, ms |
|---|---|---|---|---|
| BPC, 5 pages | 4963.8 → 4029.3 | 109 → 89 KiB, 30 → 25 calls | 0.58–0.81 → 0.56–0.69 | 0.39–0.49 → 0.32–0.40 |
| WTPDF, 57 | 26692.3 → 13183.7 | 350 → 118 KiB, 88 → 30 | 2.34–2.77 → 1.31–1.61 | 1.93–2.45 → 0.91–1.17 |
| ISO 32000-2, 1023 | 185137.9 → 70348.7 | 4320 → 102 KiB, 1075 → 27 | 21.97–26.75 → 5.43–6.13 | 12.53–15.94 → 4.38–5.41 |
| bug1815476, 1 | 3419.2 → 3401.6 | 84 KiB, 24 (unchanged) | 0.42–0.64 → 0.46–0.56 | 0.28–0.41 → 0.30–0.76 |
| xfa_filled, 1 | 1821.4 → 1160.3 | 99 → 87 KiB, 26 → 24 | 0.80–0.94 → 0.75–0.86 | 0.14–0.20 → 0.10–0.14 |

ISO 32000-2's open allocates 3.9 MiB where it allocated 11.6. **Principle 2's "[a] 500-page
document must open no slower than a 5-page one" was 38× on a cold cache and is 9×**; what remains
is §7.5's cross-reference (57.2 M of 70.3 M), `doc/todo/42` item 1's design question.

**Time to first page does not move, and the timeline says why**: the device is the longer thread on
four rows in both arms — the document thread finishes 9 to 19 ms before it — so a cheaper open
shortens nothing a warm launch waits for. What it buys is the cold launch (ISO 32000-2's open was
22 to 27 ms on a cold cache, longer than the device's 16 to 20), 4.2 MB of reads, and the open of
every other host; in `quorra` the preparation lands 0.2 to 11.9 ms after the join, before a person
can press a key, so no turn pays for it. Every frame hash
is the same in both arms. The page-turn figure is unchanged (4.0–5.3 ms against 4.1–5.4).

## 5. `bug1815476.pdf`

The one row whose document thread is the longer: in the timeline it opens at 2.1–2.9 ms and has
page one interpreted at 25.9–32.9, against the device at 23.7–29.9. Callgrind on that thread alone
(`--separate-threads=yes`): the open 2.3 M instructions, `anticipate` 318 M — `image::unpack`
246.5 M, `pdf_font::substitute`'s catalogue 28.2 M, a JPEG 20.7 M, the annotations 32 M. **None of
it is deferrable**: each is ink on page one. The lever is the first: the CCITT decode is under 1 M,
and the 246.5 M is its one-bit output unpacked to RGBA through the per-sample arm of `unpack`, one
`sample_rgba` call a pixel for a sample that can take two values. An arm that converts a byte of
eight such samples at a time is `pdf-model`'s to build and is named
in `doc/todo/42`. Its `peak_anon_mib` reads 44.5 to 46.2 MiB over the six runs; the 50.73 one run
read in round 1346 was before ADR 1532 took 4.5 MiB of GL off every row, so the band stays.
