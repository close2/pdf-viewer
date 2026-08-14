# 0330 — The page a thousand neighbours are copied for

**Status.** Accepted.

## Context

`doc/todo/47` is the cold document-wide search, and after ADR 0317 took a third of the
instructions off the interpretation half it named what was left: §7.7.3.2's page tree, walked from
the root once per page, *"and it is not a cache question"*. It proposed an index the walk fills as
it goes, and it owed two things to whoever took it — re-measure the split, because the other half
had moved, and **measure `find_leaf` before believing the walk is all descent**, because
`Pages::get` was timed whole and §7.7.3.4's inheritance is applied inside it.

Both were owed for a good reason. The second one is what changed the answer.

## What the walk was actually spending

One cold sweep of ISO 32000-2 — 1023 pages, `--profile gates`, `find_cost … 0 split 100000`, so
that the number is `Pages::get` and `interpret_with` and nothing else — under callgrind, whose
counts do not move with a machine carrying a load average of 55:

| of one cold sweep of 1023 pages | instructions | share |
|---|---|---|
| the whole run | 42 884 714 194 | |
| `interpret_with` | 34 826 824 963 | 81.2% |
| **`Pages::get`** | **7 324 564 135** | **17.1%** |
| of which `Inherited::overlay` | 3 044 825 463 | 7.1% |
| of which `rectangle` | 1 062 525 395 | 2.5% |

**Two fifths of the walk was §7.7.3.4's inheritance, and every one of those instructions was
thrown away.** `find_leaf` overlaid the inheritable attributes of *each node it stepped over*
before asking whether that node was the page it wanted — and for a leaf that is not the one asked
for, the overlay is dropped on the next line. Overlaying a page copies its whole `/Resources`
dictionary.

The other three fifths were the same shape one level down. §7.7.3.2 makes `/Kids` "an array of
indirect references to the immediate children of this node", and the walk resolved every entry it
passed. `Document::get` hands back a **clone of the whole object**, because a value behind an
`RwLock` cannot be handed back as anything else — so reading Table 30's `/Count` off a neighbour
deep-copied a page dictionary, its `/Annots`, its `/MediaBox` and its `/Resources`, to look at one
integer and drop it.

**And the tree's shape decides how many times that happens.** ISO 32000-2's root has 9 children
and one of them holds 998 of the 1023 pages, so finding page *n* copies about *n* page
dictionaries: the sweep is quadratic in the page count, which is why the todo's own figure — 1.10
ms on the thousandth page against 1.12 s for all of them — is a per-page cost that grows rather
than the flat one it looks like.

## Decision

**The page tree is walked by asking each node for the entries Table 30 gives it, and nothing is
copied out of the document until the page asked for has been found.**

Two pieces, one in each crate:

- **`pdf_syntax::Document::get_key_of(id, key)`** reads one entry out of an indirect object
  without copying the rest of it: the object cache's read guard is taken, the entry cloned, the
  guard dropped, and the value resolved exactly as `get_key` resolves one. It returns
  `Option<Object>`, and the `None` is load-bearing — it says *this object is not a dictionary*,
  which is a different statement from `Object::Null` and one the page tree needs, because a
  `/Kids` entry naming something that is not a node must not be counted as a page. `get_key`
  cannot say it: a null is what an absent entry gives.
- **`pdf_model::page::Node`** is a node of the tree *before* it is copied: `Indirect(ObjectId)`
  for every child a well-formed `/Kids` names, `Direct(&Dictionary, Option<ObjectId>)` for the
  root the catalogue resolved, for a page the recovery scan found, and for a child a producer
  wrote out inline. `find_leaf`, `locate`, `collect` and `count_leaves` — the four walks — take
  one, and the only thing that asks for the dictionary is `build_page`, once, on the page that was
  asked for.

`Inherited::overlay` moved with it, from "every node the walk touches" to "the node it descends
into and the leaf it stops on".

### What it buys, in instructions

Two binaries from one tree, the same document, the same invocation — ADR 0317's method, and for
its reason:

```sh
valgrind --tool=callgrind --callgrind-out-file=<arm>.out \
    <arm>/find_cost doc/ISO_32000-2_sponsored_EC3.pdf zzzqqqxyzzy 0 split 100000
```

| ISO 32000-2, 1023 pages, one cold sweep | instructions | |
|---|---|---|
| whole run, before | 42 884 714 194 | |
| whole run, after | **37 676 397 310** | **−12.1%** |
| `Pages::get`, before | 7 324 564 135 | 17.1% of the run |
| `Pages::get`, after | **2 123 154 570** | **−71.0%**, and 5.6% of the run |
| `interpret_with`, before / after | 34 826 824 963 / 34 819 564 096 | −0.02%, which is the check |

The readback both arms produce is **byte-identical** — 2 658 697 bytes — which is `doc/todo/47`'s
stated gate: a search that returns different results is a defect and not a speed-up. The
interpretation row is the other half of that check: this change may not touch it, and it does not.

**The first hundred pages of the same document move by 0.5%** — 3 143 147 827 → 3 128 076 011 —
and that is the shape of the thing rather than a disagreement: the walk to page 40 crosses a
handful of neighbours and the walk to page 900 crosses hundreds, so a limited run measures the
cheap end. It is worth knowing because `find_cost`'s page limit exists to make the split runnable
under callgrind, and a hundred pages is what ADR 0317 needed and not what this one does.

**One walk gains nothing and it is worth saying which.** `Pages::indices` visits every node of the
tree exactly once and steps over nothing, so there is no copy to avoid — a first touch has to
parse the object whatever asks for it. Measured on the open path that uses it,
`Outline::section_at` is 88 529 236 → 89 226 065, **0.8% worse**: two cache lookups where there
was one, on a walk with nothing to skip. It is the honest shape of the thing rather than an
exception to it, and it is small enough to leave.

### What it costs

- **No memory at all**, which is the part worth stating twice. `doc/todo/47` proposed an index of
  leaves or a cache of resolved pages and asked which; the answer is *neither*, because what the
  walk was spending was never the descent. A cache would have paid for the copies instead of
  removing them, and `CLAUDE.md`'s startup rule would then have had to be argued with. It does not
  arise: nothing is kept between calls, `Pages` is still built per lookup, and no page tree is
  walked eagerly.
- Lines: `page.rs` and `document.rs`, most of it the type and the arguments above.
- One public method on `Document`, and it is a *narrowing* rather than a widening: it answers less
  than `get` does, which is the whole reason it is cheaper.

## What was considered and not done

- **The index `doc/todo/47` proposed.** It would have to outlive a `Pages`, which is built per
  lookup, so it would live either in `pdf_syntax::Document` — where a page tree does not belong,
  clause 7 knowing nothing about clause 12 — or in `viewer-core`'s `Open`, where it would be a
  second answer to "which page is index *n*" maintained beside the first. Both cost memory and an
  invariant; this costs neither and is measured to be enough.
- **Making `Object::Dictionary` an `Arc`**, which would make *every* `get` cheap rather than this
  one path. It is the larger and possibly right answer, and it touches every crate that reads a
  PDF; nothing here forecloses it. This change is what the measurement justified.
- **A borrow of the cached object rather than a copy of one entry.** `get_key_of` could hand a
  closure the object under the read guard and copy nothing at all. It would run caller code while
  holding a lock the caller may re-enter, which is a deadlock waiting for the first caller that
  resolves anything, and the entry it copies is one value rather than a dictionary.

## What the measurement found next, and did not take

**`Dictionary::get` allocates on every lookup**, and it is 1 529 118 804 instructions — **4.1%** of
the sweep after this change, 2 503 477 461 before it. The line is
`self.0.get(&Name::new(key.as_bytes().to_vec()))`: a `BTreeMap<Name, Object>` keyed by
`Name(Arc<[u8]>)` cannot be probed with a borrowed `&[u8]` unless `Name` implements
`Borrow<[u8]>`, so every dictionary lookup in this program — not the page tree's, *every* one —
builds a heap-allocated key and drops it. `Name`'s ordering is its bytes' ordering, so the
`Borrow` is sound and the map's invariant survives it.

It is not taken here for the reason this round exists: it is a change on the hottest path in the
tree, it belongs to no clause, and it wants its own A/B rather than a ride on this one.
`doc/todo/47` carries it.

## What this does not decide

`doc/todo/47`'s remaining candidates are unaffected: parallelism is still declined on memory (ADR
0260), a text-only extraction path is still refused for divergence, and skipping pages by scanning
bytes is still unsound.
