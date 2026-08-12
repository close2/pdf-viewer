# 0287 — A resource table copied once per operator

**Status.** Accepted.
**Context.** `doc/todo/03` §1 named the next candidate its 65 944-document population offers:
"`1284722.pdf` is 11.1 s of `interpret` for 94 596 commands".

## What the page is, and why 11 seconds

Page one of `1284722.pdf` is a technical drawing: 70 209 strokes, 24 385 fills, and — from the
content stream's own operator census — **26 414 `gs` operators**. Its `/Resources /ExtGState` is an
indirect reference to a dictionary with **26 414 entries**, one per `gs`.

`callgrind` over the interpretation, 108 G instructions:

| | share |
|---|---:|
| cloning a `BTreeMap<Name, Object>` | 32.9% |
| dropping one | 23.7% |
| `malloc` / `free` beneath both | 15.9% |
| everything else, including the lexer | 27.5% |

with `apply_ext_gstate` → `resource_entry` → `Document::get_key` inclusive at 98%. So the page
spent four fifths of its interpretation **copying one 26 414-entry map 26 414 times**, to read one
entry out of each copy. Quadratic, in a function whose whole body was two lines.

The cause is an API shape rather than a mistake: `Document::get` hands back an **owned** `Object`,
because that is what makes every reader in this tree simple and re-entrant. Resolving
`/Resources /ExtGState` therefore deep-copies the category table out of the document's own cache,
every time anybody asks.

## What was done

`Interpreter::resource_entry` now splits the two shapes the standard allows a resource category to
take:

- **A direct dictionary** is already in hand, so the entry is read in place. No copy at all, and
  this is the common case.
- **An indirect reference** is resolved once and remembered, keyed by its `ObjectId` —
  `Interpreter::resource_tables`. An `ObjectId` is exactly what identifies a table, so two resource
  dictionaries naming the same object share the entry, and nothing about the page's structure has
  to be guessed at.

What it costs is a second copy of each table beside the document's own cache, bounded by the number
of distinct resource tables the page's forms reach. On the witness that is one copy of one map
against 26 414 of them.

## Measured

`examples/open_cost`, release, this machine, the three documents `doc/todo/03` names as its
population's slow ones:

| | before | after |
|---|---:|---:|
| `1284722.pdf` (94 596 commands) | 11 011 ms | **142 ms** |
| `6081357.pdf` (27 319 commands) | 267 ms | **207 ms** |
| `0423548.pdf` (1 481 commands) | 2 133 ms | 2 098 ms |

**77× on the witness**, 22% on the second, and nothing on the third — which is the right answer for
the third and worth saying: `doc/todo/03` already attributed `0423548.pdf`'s remaining seconds to
`initial_backdrop` allocating a whole surface per group, which is `doc/todo/40`'s item and not this
one. An optimisation that improved all three equally would have been measuring something else.

And on an ordinary page it costs nothing, which is the other half of the benchmark: page 101 of ISO
32000-2 under `callgrind`, **2 237 399 857 → 2 225 432 282 instructions**, 0.53% *cheaper*. Every
gate is unmoved — the corpus's 65 incomplete, the oracle's 905/68/786, both text gates, quorra's
915/37/5/17.

## Why this was not found before

Nothing in the 974-document pdf.js corpus has a resource table of this shape: the gate that would
have shown it runs in seconds because its documents are small. It took a population of 65 944
crawled pages to produce one, and `doc/todo/03`'s survey to name it — which is that file's whole
argument for taking a chunk a round, arriving as a number.
