# 0335 — The key a dictionary lookup allocates

**Status.** Accepted.

## Context

`doc/todo/47` — the cold document-wide search — has been taken apart one measurement at a time:
ADR 0256 made a *repeated* search 750× cheaper, ADR 0317 took a third of the instructions off the
interpretation half by memoising §7.4's filter chain, and ADR 0330 took 71% off §7.7.3.2's page
tree walk. What ADR 0330's own profile found next, and deliberately did not take, was the last
item on the file:

```rust
pub fn get(&self, key: &str) -> Option<&Object> {
    self.0.get(&Name::new(key.as_bytes().to_vec()))
}
```

`Dictionary` is a `BTreeMap<Name, Object>` and `Name` is an `Arc<[u8]>`, so probing the map with a
borrowed `&[u8]` is impossible unless `Name` implements `Borrow<[u8]>`. Without it, **every
dictionary lookup in the program** — not the page tree's, every one — builds a heap-allocated key
and drops it: a `Vec<u8>` allocated and copied into, an `Arc<[u8]>` allocated and copied into from
that, and both freed on the way out. Two allocations and two copies to compare against a key the
caller already holds as a `&'static str`.

ADR 0330 measured it at 1 529 118 804 instructions, 4.1% of a cold sweep, and left it alone on the
ground that it is the hottest path in the tree, belongs to no clause, and folding it into that
round's A/B would have made two changes one number. This is its own A/B.

## Is the `Borrow` contract actually satisfied?

`Borrow<[u8]>` is not a convenience conversion. `std` states the obligation on the implementor, and
a `BTreeMap` probed through a `Borrow` whose ordering disagrees with its keys' does not fail
loudly — it silently fails to find entries that are there. So the claim `doc/todo/47` inherited
from ADR 0330 — *"`Name`'s ordering is its bytes' ordering"* — was checked in the source before
anything was built, and it holds for a reason stronger than inspection:

- `Name` is `pub struct Name(pub Arc<[u8]>)` and derives `PartialEq, Eq, PartialOrd, Ord, Hash`
  (`crates/pdf-syntax/src/object.rs`). **There is no hand-written impl of any of the five anywhere
  in the tree** — grepped over `crates/` for `impl … for Name`, which finds only
  `PartialEq<&str>` (a *different* trait, comparing against a `&str`, not `Name`'s own `PartialEq`)
  and `Display`.
- A derive on a one-field tuple struct delegates to the field. `Arc<T>`'s `Ord`, `Eq` and `Hash`
  delegate to `T`. So `Name`'s three are `[u8]`'s three, exactly — which is also what makes the
  hashing half of the contract hold, and that half matters even though a `BTreeMap` never asks:
  `Borrow` is one contract and a later `HashMap<Name, _>` would inherit it.
- The equality half is not merely an implementation accident, it is what the standard says a name
  *is* (§7.3.5): *"Uniquely defined means that any two name objects that, after all escaping is
  expanded (see below), and the resulting sequences of bytes are not an exact binary match denote
  different objects."* Escapes are expanded at lex time in this tree, so the bytes in a `Name` are
  the bytes the clause compares.

If any of that had been false the fix would have had to be a different one — an interned key, or
a `BTreeMap<Arc<[u8]>, Object>` with the ordering stated once — so the check was worth its minute.
`object.rs::borrows_exactly_as_it_compares` is the standing form of it: over eight sample keys
chosen for the three ways a byte-wise order goes wrong (a prefix against what extends it, a byte
above 127 against one below, and §7.3.5's empty name), it asserts that `Borrow` hands back the
bytes, that `Name`'s `cmp` equals the bytes' `cmp` pairwise, and that the two hash alike. It
guards against a *future* hand-written impl, which is the only way this can now break.

## Decision

**`impl Borrow<[u8]> for Name`, and `Dictionary::get` and `Dictionary::remove` probe the map with
the caller's own bytes.** Four lines of impl, two lines changed, one test module added.

## What it buys, in instructions

Two binaries from one tree — ADR 0317's method, for its reason: this machine carries ten parallel
rounds and a wall clock on it is not evidence. ADR 0330's exact invocation, so that the two rounds'
numbers are about the same thing:

```sh
valgrind --tool=callgrind --callgrind-out-file=<arm>.out \
    <arm>/find_cost doc/ISO_32000-2_sponsored_EC3.pdf zzzqqqxyzzy 0 split 100000
```

| one cold sweep, ISO 32000-2, 1023 pages | instructions | |
|---|---|---|
| **whole run** | **37 642 044 068 → 36 920 639 974** | **−1.92%** |
| `Dictionary::get`, inclusive | 1 529 890 537 (4.06%) → 243 241 | the item, gone |
| `interpret_with`, inclusive | 34 785 695 797 → 34 512 393 072 | −0.79% |
| `Pages::get`, inclusive | 2 114 147 099 → 1 674 108 309 | −20.8% |
| `malloc`, self | 970 008 670 → 784 946 458 | **−19.1%** |
| `free`, self | 1 407 620 170 → 1 187 110 535 | **−15.7%** |
| `<Arc<[u8]>>::drop_slow` | 62 394 035 → 26 332 713 | −57.8% |

**The instrument agrees with the round that named the item.** `doc/todo/47` recorded
1 529 118 804 for `Dictionary::get`; this build of the same source measures **1 529 890 537**,
0.05% apart, which is the repeat ADR 0330 predicted and the check that the two arms are comparable
at all. The before arm's whole run, 37 642 044 068, is likewise 0.09% from ADR 0330's after figure.

**Where the 1.53 G went, because only 0.72 G of it came off the run.** The rest is the map's own
search, and it did not disappear — it stopped being a function. With no allocation left in the
body, `Dictionary::get` inlines into its callers, so the comparisons move there:
`Document::get_key_of` +314 171 539, `Interpreter::resource_entry` +44 898 770,
`Document::get_key` +14 620 842. What is actually removed is the allocation, the copy and the
free — about 220 instructions on each of **3 278 302** calls.

**3 278 302 is a call count taken from the profile rather than estimated** — the `calls=` lines of
the before arm's output, summed over `Dictionary::get`, which that arm has as a real function
precisely because the allocation kept it from being inlined. It is a floor on the number of
dictionary lookups rather than the number: a call site the before arm *did* inline is not in it.

## The output is identical, which is the gate

`doc/todo/47`'s standing rule is that a search returning different results is a defect and not a
speed-up, and the byte count `find_cost` prints is not enough to say so. Both arms' `readback`
example was run over **all 1023 pages** and the concatenation compared:

```sh
for p in $(seq 1 1023); do <arm>/readback doc/ISO_32000-2_sponsored_EC3.pdf $p; done > <arm>.txt
cmp before.txt after.txt        # silent
sha256sum before.txt after.txt  # ed074b1c…  both, 2 730 201 bytes
```

`find_cost`'s own split line agrees at 2 658 697 bytes on both arms.

**The corpus gate was run on both arms too and its report is identical line for line** — 974
documents, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **62 incomplete**, and the
same three silence counts (5 over 2, 57 over 9, 1228 over 43) — with the four named lists compared
as sorted sets and differing in nothing. The only line that moves is the wall clock: 17.0 s before,
24.1 s after, on a machine carrying ten rounds.

**That last figure is the one caveat and it is the machine's.** Two of this round's corpus runs
failed the gate's *per-document wall-clock* assertion, `22060_A1_01_Plans.pdf` at 35.2 s and 80.1 s
against a 30 s budget, while the load average was between 119 and 136; the same binary passes at
24.1 s total when it is not. `KNOWN_SLOW` is empty, so that assertion has no tolerance at all, and
`doc/performance.md` already says of this document that "a per-document wall-clock budget measured
under a 24-way parallel load is a property of the machine as much as of the file". The instruction
count settles it rather than the clock: **page one of that exact document is
59 483 506 202 → 59 482 004 881, −0.003%**, so nothing this change did could put five seconds on it.

## What it does *not* buy, and the instrument that says so

The item is a per-lookup cost, so it pays where lookups are the work and vanishes where pixels
are. Measured rather than assumed, `open_one` on page one of two large corpus documents, one
text-and-vector and one the heaviest in the corpus:

| `open_one`, page one, `RAYON_NUM_THREADS=1` | instructions | |
|---|---|---|
| `freeculture.pdf` | 1 086 502 241 → 1 086 468 287 | −0.003% |
| `22060_A1_01_Plans.pdf` | 59 483 506 202 → 59 482 004 881 | −0.003% |

Both pages are image work — `apply_soft_mask`, `convert_three`, `area_averaged` — and a dictionary
lookup is 0.002% of them. The change is real in both (`Dictionary::get` gone, `malloc` and `free`
down by its share) and worth nothing at that scale, which is the honest shape of a per-lookup
saving and not a disappointment.

**And the first attempt to measure those two pages was the trap, so it is written down.** Run with
rayon's default thread count they read **+0.154%** and **+0.010%** — a regression, on a change
that cannot cost anything. The diff says why in one line: `crossbeam_deque::Stealer::steal`
+1 240 958 and +5 113 229, `WorkerThread::wait_until_cold` +421 193 and +1 700 720. Callgrind
counts every thread, so a work-stealing pool's *spin* is in the total, and on a page whose real
delta is a few thousand instructions the spin is three orders of magnitude larger.
`doc/performance.md` already says *quote the clock for a parallel change and the counter for a
serial one*; this is its converse — **pin the pool before counting a serial change in a program
that has one.**

Two paths where lookups *are* the work, both serial and both deterministic:

| | instructions | |
|---|---|---|
| `callgrind_interpret`, ISO 32000-2 page 101 | 1 374 953 559 → 1 361 198 350 | **−1.00%** |
| `open_cost`, ISO 32000-2's launch path | 241 989 744 → 239 910 560 | **−0.86%** |

The second is the one worth keeping: `CLAUDE.md` makes startup a first-class requirement, and
opening a 1023-page document costs 0.86% fewer instructions for a change nobody made for that
reason. Both examples' own printed output is identical between the arms — 150 350 commands from
`callgrind_interpret`, and `open_one`'s 126 commands with an empty `unsupported` list.

## What it costs

- **No memory.** Nothing is kept, interned or cached; a lookup allocates nothing where it used to
  allocate twice. This is the second item on `doc/todo/47` in a row that costs nothing (ADR 0330
  was the first), which is worth noticing on a file whose first three candidates were all priced
  against memory.
- **Four lines of impl and two changed lines**, against a doc comment and a test module that are
  longer than both. That ratio is `CLAUDE.md`'s rule working rather than failing: the body is one
  expression and the *reason it is sound* is the part a reader needs.
- **One trait impl on a public type.** `Name: Borrow<[u8]>` is now part of the crate's API and
  binds every future `Ord`/`Eq`/`Hash` on `Name` to the bytes'. That is a constraint, and it is one
  §7.3.5 already imposes.

## What was considered and not done

- **Interning the key at the call site** — a `static NAME_TYPE: Name` per literal, or a
  `phf`-style table. It removes the same allocation and adds a name to maintain per key, and the
  measurement says the allocation is all there was.
- **`Borrow<str>` instead of `Borrow<[u8]>`.** `Dictionary::get` takes a `&str`, so it would read
  slightly better at the call site — and it would be **wrong**: a `Name` is bytes and need not be
  UTF-8 (§7.3.5 excludes only the null), so no total `Name → &str` borrow exists. `key.as_bytes()`
  at the one place that has a `&str` is the honest direction.
- **Making `Object::Dictionary` an `Arc`**, which ADR 0330 also listed. Still the larger and
  possibly right answer, still untouched by this; it makes `Document::get` cheap where this makes
  the *probe* cheap, and they are different costs.

## What this closes

`doc/todo/47` is deleted with this. Every candidate it raised is now either built (the readback
cache, ADR 0256; the decoded-stream memo, ADR 0317; the page tree, ADR 0330; this) or refused with
a number in an ADR (parallelism on memory, ADR 0260; a text-only extraction path, on divergence;
skipping pages by scanning bytes, on soundness). The argument lives in those five ADRs, which is
where a file is meant to end up rather than in a file nobody deletes.

The owner's original question — *"6.19 s doesn't sound that fast, can we easily improve this?"* —
is answered in both halves: a repeated search is 750× cheaper, and a first search has lost a third
of the interpretation, 71% of the page tree walk and this. What is left of a cold sweep is
`interpret_with` at 93.5% of it, which is the page being read, and the only way to make *that*
smaller is to read fewer pages — which is candidate 1, refused, and candidate 2, priced at 2.8× to
4.3× the peak memory against an owner's stated bound.
