# 452 — A resource table copied once per operator

**Finding.** `Document::get` hands back an owned object, so resolving `/Resources /ExtGState`
deep-copies the whole category table — and a page with one `/ExtGState` entry per `gs` operator
copies an *n*-entry map *n* times. `1284722.pdf` is that page: 26 414 entries, 26 414 operators,
and 57% of 108 G instructions spent cloning and dropping one `BTreeMap`.

**Date.** 2026-08-12.
**ADR.** [0287](../adr/0287-a-resource-table-copied-once-per-operator.md).
**Touched.** `crates/pdf-model/src/content.rs`, `doc/todo/03-more-corpora.md`,
`doc/adr/0287-*`, this file.

## Measured

| | before | after |
|---|---:|---:|
| `1284722.pdf` | 11 011 ms | **142 ms** |
| `6081357.pdf` | 267 ms | **207 ms** |
| `0423548.pdf` | 2 133 ms | 2 098 ms |
| ISO 32000-2 page 101, instructions | 2 237 399 857 | 2 225 432 282 |

Every gate unmoved. The third document not moving is the correct answer: its cost is
`initial_backdrop`'s and belongs to `doc/todo/40`.

## Where it came from

`doc/todo/03` §1 named the witness — "the next candidate this population offers" — which is that
file's argument for taking a chunk a round arriving as a number rather than as a plan. Nothing in
the 974-document pdf.js corpus has a resource table of this shape.
