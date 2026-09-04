# 925 — The largest item in a large document's open was a walk nobody had measured

2026-09-04. Argued in
[ADR 0890](../adr/0890-where-a-large-documents-open-goes-and-the-page-tree-walk-on-the-launch-path.md)
(where the time goes, and the walk on the launch path) and
[ADR 0891](../adr/0891-the-open-is-linear-in-the-objects-the-table-names.md)
(what cannot be made lazy, and what the resource port did to the other false sentence). A fifth
round, on its own branch, merging round 922's instrument.

Session 922 measured `CLAUDE.md` principle 2's claims and found that *a 500-page document must
open no slower than a 5-page one* is false by about thirty times. It did not say where that time
goes. This round profiled the open, and the profile found a **second** false sentence in the same
bullet that 922 had recorded as holding.

Touched: `crates/pdf-model/examples/open_cost.rs`, `crates/pdf-model/src/{outline,destination,
retrieval}.rs`, `crates/pdf-transform/src/split.rs`, `crates/pdf-transform/tests/support/mod.rs`,
`crates/pdf-vfs/src/worker.rs`, `crates/viewer-core/src/{open,viewer}.rs`;
`doc/questions/Q25`, `doc/performance.md`, `doc/checks/launch-path.toml`; two ADRs, this file.
No clause's reading moved and no ledger row changed status: §12.3.3, §12.3.2 and §7.7.3 answer
exactly what they answered, and all three rows were already non-`unreviewed`.

## 1. The instrument had a hole, and it is why nobody had seen this

`examples/open_cost.rs` says in its own module comment that it measures "everything
`viewer_core::Open::around` and `viewer_core::notes::about` do before a window exists". It did
not: `Viewer::open` ends with `announce_page`, which is neither, and which is where the largest
item was. **A step an example's comment claims and its output omits is invisible exactly the way a
missing gate line is** — ADR 0885 read the *no full page-tree walk* claim off this example and off
`Pages::new`, and was right about both and about a step two functions earlier.

## 2. What scales, and with what

Three things are 96% of a 1023-page document's open, and they are linear in **three different
populations**: 112 269 cross-reference entries, 1023 pages, 988 outline items. None of the three
is linear in the page count, which is what the sentence names. `tools/state.sh launch` prints the
open; `cargo run --release -p pdf-model --example open_cost -- <file>` prints the split, and ADR
0890 has the table.

The largest of the three is `Pages::indices()` — every node of §7.7.3's tree resolved — reached
from `announce_page` through §12.3.3's `Outline::section_at`, because the window caption names the
section the reader is in. That is the full page-tree walk principle 2's first startup bullet
forbids by name.

## 3. What was made lazy, and what was not

**Made lazy, and it costs nothing:** the walk was being paid *again on every page turn*, together
with a fresh `Pages`. `pdf_syntax::Document` is immutable, so the map is a function of the file;
`Open` keeps it in a `OnceCell`, `Outline::section_at_with` takes a prepared map, and
`Destination::page_index_with` lost the `Pages` parameter it stopped using the moment it was given
one. **0.77 ms off every arrow key on ISO 32000-2**, measured as an A/B in one binary.

**Not made lazy, deliberately:** taking the walk off the *open* means the caption gains its
section on a second `Event::PageChanged` that six hosts would have to expect. It buys the
1023-page open a fall from about 11.3 ms to 4.4 — and the sentence is still false by nineteen
times, because §7.5.6 makes reading all of a cross-reference table the price of reading any of it.
A host-visible change that does not make the claim true is one to ask for, not one to make; it is
in `Q25` with the arithmetic.

**And one alternative was priced and refused rather than tried**: stopping the walk at the page
being shown is a *regression*, because a destination the truncated map does not hold is answered
by `document.get`, which parses and clones the object, and the outline names about as many
distinct pages as the document has pages. ADR 0891 records it so the next round does not
re-derive it.

## 4. The other sentence, and what session 920 changed about it

*No system font enumeration* is false for a page naming an unembedded font. Session 920's resource
port changed **who** enumerates rather than whether: the broker walks the directories on behalf of
a confined worker that asks by description, so the cost stays on the launch path and gains a copy
across a pipe — while a host that offers nothing gets a worker with no machine fonts at all, where
the sentence is true. So the port makes it conditional on the *host* on top of conditional on the
document.

Three of principle 2's startup prohibitions are now known false as flat sentences and true as the
general rule two bullets below them means them. All three are `Q25` items 4 to 6. Nothing in
`CLAUDE.md` was amended, for the third round running.

## 5. What the gates said, and one band that moved

`doc/todo/02` §2 ran whole and green, both lint lines under `RUSTFLAGS="-D warnings"`, both `fuzz/`
lines, every corpus walk under `tools/bounded.sh --tree 12` with the one-walk rule waited on by
`/proc/PID/exe`. §5's ten binaries and two libraries were rebuilt in `release` and installed after
the last edit, which is the rule this round measures under.

**`doc/checks/launch-path.toml`'s page-turn floor for ISO 32000-2 moved from 4.5 ms to 3.8**, and
that is the one number this round changed: a floor derived from a slower program is a floor a
faster one falls through. The file carries the derivation beside the band.

**The launch gate declined to judge the clock on this afternoon's machine, and said so** —
calibration 0.900 and 0.909 ms against a band of 0.620 .. 0.780, over runs an hour apart at load
averages of 6 to 12, with three neighbouring rounds building. That is ADR 0884's guard working:
the run exits 0, prints every figure, and judges the eight with no machine in them. One
observation from those runs is worth carrying forward rather than acting on: the device-inclusive
memory high-water read 103 to 119 MiB against bands starting at 127 — the same *downward* drift in
the driver's allocation that got that figure demoted out of the deterministic pair in session 922,
seen a third time.
