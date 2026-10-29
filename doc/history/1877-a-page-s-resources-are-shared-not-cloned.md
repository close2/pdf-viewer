# 1877 — A page's resource dictionary is shared among the pages that name it, not copied per page

Slot 7 of batch 128, 2026-10-29, a "parsers" round (robustness, Opus 5.5), one session; ADR 2588, ADR 2589 and Q753
unused. I worked in `scratchpad/r1877/` and two exports of this tree built there. No ledger row moved: §7.7.3.3,
§7.7.3.4 and §7.8.3 stay `implemented`; the slot is principle 3's.

**Measured first.** The input rebuilt from round 1864's description: N pages under one node, one `/ExtGState` of
20 000 graphics states, every page naming it `3 0 R` or inheriting it. At HEAD `quorra-transform archive --to 4`
peaks at 2.6 GB for 100 pages and runs the worker out of memory (exit 2, 4.17 GB) at 1 000 and 2 000.

**The change.** `Page::resources` is an `Arc<Dictionary>`; a walk keeps one handle per object a `/Resources` entry
names, read through the new `Document::stated_key_of` (the entry before the reference is followed; `get_key_of` is
now it plus `resolve`), and an inherited dictionary is the ancestor's handle. Sharing follows §7.8.3's reference and
never the content. The memo holds one entry per visited node, under `MAX_NODES_VISITED`, and dies with the walk.
The interpreter and a form's restore take a handle where they took a deep copy per interpretation; the archive
survey holds the page's handle. Every other reader derefs; nine sites keep their copy byte for byte (ADR 2588).

**Result.** 2 000 × 20 000 by reference converts, exit 0, 101 372 KiB beside HEAD's 4 167 132; 100 pages 93 408 KiB
beside 2 629 548 (the table at N = 100, 1 000, 2 000 is ADR 2588's). The inheriting shape is refused at exit 4 from
33 pages at HEAD and here alike, width aside: trap 243's shape, found and named.

**The premise that did not hold.** "The clone is the only copy": `Document::get` copies a whole object on every
call, so sharing needed the reference, hence the memo keyed by it, and the interpreter cloned again per page.
`turn_path` moved down, not level: interpretation-heavy pages 11–33 % faster in both interleaved pairs.

**Plant.** The memo bypassed (a fresh resolve per page): `shared_resources` failed 3 of 6, sharing by reference,
the 2 000-page count and the copy-on-write test, and its run went from 0.02 s to 2.33 s; restored, 6 of 6.

**Gates.** `cargo test -p pdf-model --no-fail-fast` exit 0 (2 361 passed, 144 binaries); `--test shared_resources`
exit 0 (6); `cargo test -p pdf-transform --test archive` exit 0 (501 passed); `RUSTFLAGS="-D warnings" cargo clippy
-p pdf-syntax -p pdf-model -p pdf-archive -p pdf-transform --all-targets` exit 0; `cargo fmt --check` on the four
exit 0; `cargo check --workspace --tests --examples` exit 0; the six arms exit 0 each, against HEAD's six arms 0 pages moved by digest (968 at 1x, 965 at 4x); `raster_golden` exit 0 (2); `turn_path` in one `--clock --nice 0` hold,
HEAD's export and this one alternately twice: this change exit 0 both (2 passed), HEAD exit 101 both on bands it
left; `--bin cited` exit 0, `--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 0 (470 passed).
