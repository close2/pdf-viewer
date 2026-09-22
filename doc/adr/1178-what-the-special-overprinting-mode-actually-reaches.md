# ADR 1178 — What §11.7.4.3's special overprinting mode actually reaches, and what refusing it costs

Status: accepted, 2026-09-22.

Answers the count ADR 1158 section 4 left open and handed to `doc/todo/23`: whether a producer
that enables overprinting inside a `DeviceCMYK` group is common enough for `render-raster`'s and
`render-gpu`'s by-name refusal to matter. It was left as *the conservative answer* precisely
because nothing in the tree printed the number. It is printed now.

## The instrument

`crates/pdf-model/examples/overprint_ink_group_census.rs`. Every column is the interpreter's own
verdict rather than a second copy of §11.7.4.3's five conditions — `DisplayList::overprints()`,
the commands whose `blend()` is `BlendMode::Overprint`, the non-isolated group the clause's last
paragraph builds around such an object, and `Unsupported::Overprint` — so a change to the rule
moves the number and leaves the predicate where it is. A document no object of which carries
Table 58's `/OP` or `/op` is not interpreted at all, which is what makes it affordable: an
`ExtGState` is the only place §8.6.7's parameters can be set, so the filter is an
over-approximation that cannot exclude a page it should have admitted.

```sh
cargo build --profile gates -p pdf-model --example overprint_ink_group_census
RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 12 --tree 12 -- \
    <target-dir>/gates/examples/overprint_ink_group_census @crawl.txt
```

## What it says

Over the 65 944 crawled documents, 2557 s, peak 10.75 GiB: 65 720 open, 48 586 of them state no
`/OP` or `/op` anywhere, and the remaining 17 134 are interpreted over 248 615 pages.

- **1826 documents and 9863 pages paint under the mode**, 27 435 261 marks in all. That is
  **2.8% of the documents that open** — not a handful, and far more than the one corpus witness
  the feature was built against.
- **145 documents and 493 pages do it under a non-Normal blend mode**, which is the case
  §11.7.4.3's last paragraph wraps in an implicit non-isolated, non-knockout group: 43 199 marks,
  42 075 of them under `Multiply`, then `Darken` 722, `HardLight` 256, `Screen` 106, `Color` 20,
  `Luminosity` 12, `Overlay` 8.
- **Not one page in the crawl carries an `Unsupported::Overprint`.** Both remaining reports are
  §11.4.6 NOTE 6's knockout case, for §11.7.4.3's group and for §11.7.4.4's first bullet, and the
  web does not reach either. What is left unbuilt around this mode is a population of zero.

Ten of the 1826 documents were opened and read against the clause's own conditions, in the
files' own bytes rather than through this tree: all ten state `/OP true`, `/op true`, `/OPM 1`,
a `DeviceCMYK` space and a `k` or `K` operator with a zero component. Ten of ten are the rule.

## The consequence for the two backends

`render-raster` and `render-gpu` refuse a page by name when `DisplayList::overprints()` is true,
which ADR 1158 section 1 took as the conservative answer while the number was unknown. The
number is 1826 documents, so that refusal is the largest by-name coverage loss either backend
carries, and it is **worth building rather than worth keeping**. `render-cpu` draws these pages
today; the two others do not.

## A second finding, which the census's own self-check made

The census prints how many pages carry the verdict and no command under the mode, because an
instrument that cannot say when it has missed something is measuring itself (trap 13's shape,
applied to a walk rather than to a sweep). It is **177 of 10 040**, and the first two causes it
exposed were this example's own — a `Command::Shaped` whose object was never entered, and a
`Says::absorb` that never folded the check's own field, which is why the column exists at all.

What is left is the interpreter's: `content::overprint` calls `note_overprinting()` where it
**computes** the mode, and `path.rs` asks for the fill's mode and the stroke's before it asks
whether either part marks the page. So a page can carry the verdict for a part that never drew.
That is 1.8% of the pages the two backends refuse — refused for a mark that is not on them —
and it is a defect in the flag rather than in the clause. Whoever narrows the refusal should set
the flag where the command is emitted, not where the mode is chosen.

## What is not decided here

Whether §11.7.4.3's group is worth building in `render-raster` and `render-gpu`, and in which
order. This ADR prices it; `doc/todo/23` carries the item.
