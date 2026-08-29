# 821 — A recipe that named one corpus, and a length chosen from a curve rather than its end

Date: 2026-08-29. Branch `round-821`, from `main` at `3ec61db7`. Parallel round, worktree `r821`,
beside 819, 820 and 822.
ADR: [0751](../adr/0751-a-recipe-that-named-one-corpus.md).
Touched: `fuzz/seed_x509.py`, `doc/verify.md`, and two new files — `doc/adr/0751` and this one.
No Rust changed, `tools/fuzz.sh` is unchanged, and no ledger row is touched: every clause this round
cites — §12.8.3.3.1, §12.8.4.3, §12.8.4.4 — already carries a non-`unreviewed` row, and nothing here
changes what the program does with any of them.

ADR 0747 handed three findings forward and this round took all three:

1. **Eight targets saturated** — confirmed and priced. **No change**: the search half of all eight
   together is under thirteen seconds, so there is nothing to save and nothing to buy.
2. **`display_list` and `confined_wire` under-run** — not in the sense 0747 meant. Both were
   *under-seeded*, and 0747's own campaign was the cure. `display_list` **stays at 600 s**;
   `confined_wire` goes from **1 000 000 runs to 4 000 000**, on its *rate* rather than on the
   fact that it is still climbing, which every target with a logarithmic curve always is.
3. **`x509`'s corpus thin** — the cause was the recipe's argument list. Fixing it and the script
   bought **+1342 edges and +1496 features from seeds alone**, where 0747's whole million-run
   campaign on this target bought +72 and +125.

## Machine and load — read this before any timing below

Four rounds share 24 cores. The one-minute load average over this round's runs moved between 0.4
and 36, and `display_list`'s own reported rate moved between 7100 and 3700 executions a second
*inside a single run* as siblings built. **No conclusion rests on a rate**: libFuzzer's `cov:` and
`ft:` are cumulative sets, and every coverage figure below is one of those. Wall clock is labelled
where it appears.

The fuzz targets ran sequentially, one at a time, each `nice`d. The document sweeps, a `clippy`
warm-up and the certificate harvest are not fuzz targets and did run beside `display_list`, which
is why its table carries the execution count beside the second. `confined_wire`, the eight and
`x509` ran on a machine at a load average under 1.5.

**And the corpora moved under this round.** `lexer` went from 4373 seeds to 4812 and `document`
from 12 171 to 14 046 without this round touching either: `fuzz/corpus` is one directory shared by
every worktree (ADR 0742), so a sibling round fuzzing is a sibling round writing here. Nothing
below is affected — every figure is a pair read inside one run — but a round comparing its
`INITED` against a neighbour's should know.

## 1. `display_list`, re-priced against the corpus 816 left behind

816 ran this target for its documented 600 s and it gained **+884 edges and +3549 features**, half
again what its whole seeded corpus had, writing 651 new units. That is why it wrote that the target
"has not been given its budget's worth".

Run again now over **six times** the documented length, on the corpus that run produced — 1512
seeds, `INITED cov: 2676 ft: 10805`:

| elapsed | executions | cov | ft | gained over `INITED` |
|---|---|---|---|---|
| 303 s | 1 486 346 | 2709 | 10 923 | +33 / +118 |
| **605 s** — the documented length | 2 233 680 | 2712 | 10 957 | **+36 / +152** |
| 904 s | 2 874 606 | 2717 | 11 013 | +41 / +208 |
| 1503 s | 3 644 601 | 2719 | 11 020 | +43 / +215 |
| 3601 s, `DONE` | 10 389 397 | 2733 | 11 087 | **+57 / +282** |

Seeds 1512 → 2019.

**The documented length now buys a twenty-third of the features it bought in 816, and six times the
documented length buys not quite twice that again.** 600 s stays.

## 2. `confined_wire`, re-priced the same way

816's million runs gained **+660 edges and +3189 features** and took 156 s on a machine at load 300.
Run now over **eight times** the documented length, on 8458 seeds, `INITED cov: 7965 ft: 19547`, on
a quiet machine:

| runs | elapsed | cov | ft | gained over `INITED` |
|---|---|---|---|---|
| **1 000 000** — the documented length | 36 s | 8179 | 20 228 | **+214 / +681** |
| 2 000 000 | 74 s | 8215 | 20 392 | +250 / +845 |
| **4 000 000** | 168 s | 8302 | 20 574 | **+337 / +1027** |
| 8 000 000, `DONE` | 395 s | 8355 | 20 797 | +390 / +1250 |

Seeds 8458 → 9338.

**It is still climbing at every one of those four points**, which is what 816 observed at the first
of them — and that is the finding: a logarithmic curve is always still climbing, so *still climbing*
cannot choose a length. What chose this one is the rate. **This target adds a couple of hundred
edges in 36 seconds where `display_list` adds three dozen in ten minutes**; it is the cheapest
coverage in the tree and it had the smallest budget of the fifteen. The gain per doubling is
+36, +87, +53 edges — flat within noise up to four million and falling after — so
**`-runs=4000000`**, which is under three minutes, is what `doc/verify.md` now says.

## 3. The eight saturated targets, priced

Two runs each: `-runs=0`, which loads the corpus, executes every seed and stops — the **replay** —
and the length `doc/verify.md` states. The difference is what the **search** costs. Sequential, on
a quiet machine.

| target | replay | documented | the search cost | and bought |
|---|---|---|---|---|
| `lexer` | 0.4 s | 0.7 s | 0.3 s | 0 / 0 |
| `object` | 0.4 s | 1.5 s | 1.1 s | +4 / +35 |
| `cmap` | 0.4 s | 1.5 s | 1.1 s | +6 / +15 |
| `sfnt` | 0.5 s | 5.0 s | 4.5 s | +2 / +8 |
| `xmp` | 1.9 s | 4.1 s | 2.2 s | +2 / +18 |
| `fragment` | 0.3 s | 1.7 s | 1.4 s | 0 / +15 |
| `cms` | 0.2 s | 0.7 s | 0.5 s | +2 / +2 |
| `forms_data` | 0.2 s | 0.7 s | 0.5 s | 0 / +3 |
| **all eight** | **4.3 s** | **16.9 s** | **12.6 s** | **+16 / +96** |

**There is nothing here to save.** Shortening these lengths would recover twelve seconds; lengthening
them would buy a target that has found sixteen edges in twelve seconds of searching. `lexer` reproduced
816's zero, on a corpus a sibling round had grown by 439 files in the meantime.

**The replay is the part that is not free and is the part worth keeping**: every seed executed
against today's code is a regression check on the parser that no `#[test]` in this tree performs.

## 4. `x509`, and what an argument list was worth

The recipe said `doc/pdf.js/test/pdfs/*.pdf`. Over the whole tree instead:

| | certificates |
|---|---|
| `doc/pdf.js` alone, the route the script already had | 22 |
| `doc/pdf.js` alone, all three routes | 43 |
| every PDF in the tree, all three routes | **941** — 710 first seen inside a signature, 222 stated by a document, 9 out of a fixture |

`grep -alr /ByteRange` over `corpus-cache`, `doc/corpora` and `doc/pdf.js/test/pdfs` finds 706
signed documents; nine of them are pdf.js's. The whole-tree harvest takes **7 min 10 s** as one
process.

What that bought, measured inside runs of the target:

| | seeds | cov | ft |
|---|---|---|---|
| the corpus 816 left | 533 | 955 | 1571 |
| after the harvest, before any fuzzing | 1450 | **2297** | **3067** |
| after the documented 1 000 000 runs | 1530 | 2375 | 3469 |

**The seeds alone added +1342 edges and +1496 features. 816's million runs on the old corpus added
+72 and +125.** The seeds are worth about nineteen times the campaign, in edges — and they also make
the campaign worth more: the same million runs that bought +125 features then buys **+402** now.

## Trap 13 — the seeder's new routes, run against what they are about

The plants are certificates rather than edits to the tree, because what is under test is a
recogniser. Two hundred sampled at random (seed 821) from the certificates the *unchanged* route
one harvested, above `3ec61db7`:

| plant | the new route says |
|---|---|
| a route-one certificate placed in a buffer between junk and trailing bytes | found, **200 of 200**, and the bytes returned are exactly the certificate |
| the same certificates, three bytes short | **0 of 200** |
| the same, with `signature`'s `BIT STRING` retagged a `SEQUENCE` | **0 of 200** |
| one placed inside a `FlateDecode` stream of a one-object PDF | raw scan 0, inflated scan 1 |
| `P521_CERTIFICATE` renamed `P521_CERT_BYTES` in `ecdsa.rs` | route three finds 3 where it found 4 |
| the 47 things the *loose* check kept and the field-list check drops | all 47 parse as RFC 5280 §5.1 `CertificateList` under `openssl crl`, and none of route one's 718 is lost; the largest seed falls from 1.5 MB to 3332 bytes |

**The first plant is also a cross-check between the two routes**: route one re-encodes each
certificate with a definite length of its own construction and route two returns the file's bytes
verbatim, and on all 718 the two agree byte for byte — which is what lets the SHA-1 filename
deduplicate across them.

## Crashers

**None.** `fuzz/artifacts` gained no `crash-`, `oom-`, `leak-` or `slow-unit-` file from any run
this round made. Principle 3's "every crasher found becomes a permanent regression test" had
nothing to bind on, and that is reported as a result because the `INITED → DONE` pairs above are
what make it a claim about the code rather than about an exit status (ADR 0747).

## Gates

`tools/round.sh` says this is not a fifth round, and the change is two documents and a Python
script — no Rust, no crate a gate rasterises with. So §2's core, both `fuzz/` lines included, plus
the line the change→gate map adds for a documents-only change. Run on a machine at a load average
under 1.5, with nothing beside them:

| line | result |
|---|---|
| `cargo fmt --all --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | exit 0, and the only `warning:` lines are the documented `proc-macro-error2` future-compat note and the cold-build `cxx-qt` gcc `-Wmaybe-uninitialized` block |
| `cargo nextest run --workspace` | 2790 run, 2790 passed, 18 skipped |
| `cargo test --workspace --doc` | passed |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets` | silent |
| `cargo test -p conformance -- --nocapture` | 200 passed, 0 failed |

`--bin quotations` and `--bin pointers` are in the sweeps below, which is where a documents-only
change owes them.

## The §4 sweeps, before and after

Twenty-one sweeps over a pristine `git worktree` of `3ec61db7` with its own build directory
(`r821base`, closed with it), and again over this branch. Every exit status is identical, `quoted`,
`retired` and `unpriced` exiting 1 on both sides as they have since ADR 0742's run — those three
take an argument this run did not give them. **Seven outputs differ, fourteen are byte-identical,
and none of the seven is a finding:**

- **`counts`** reads 9092 governing sentences against 9091 — this round's prose. Every other figure
  on the line is identical.
- **`errata-applied`** reads 61 440 places against 61 395. **1005 name an erratum this collection
  carries on both sides**, and the `#NNN` token count is identical at 203. It is also the only
  figure here that moved between this round's two branch-side runs, which is worth a sentence: a
  sweep whose population includes this file cannot be reported in this file without a one-step lag,
  so the numbers above are the run made on the tree as it stood before this paragraph was written.
- **`ledger`** differs only in the absolute path it prints. 875 rows, 0 new, both sides.
- **`overtaken`** reads 636 decision records against 635 — ADR 0751. Same 137 page-list notes over
  340 documents, same 45 overtaken, same three sub-counts.
- **`pointers`** reads 9304 path pointers against 9289, the fifteen being paths this round's files
  cite. **`102 absent` on both sides**, which is the finding-shaped number, and `162 symbol
  pointer(s), 13 undefined` is identical.
- **`quotations`** reads 6908 in 1116 documents against 6904 in 1114 — the two new files. **`2869
  verbatim` and `38 diverging` are identical on both sides**, so nothing this round wrote claims to
  be a quotation and is not one.
- **`tables`** reads 2642 attributed key citations against 2639, and **all three of the new ones
  agree with the table they name** — 2474 agreeing against 2471, with `101 absent` and the six
  contradicted denials identical. Those three are §12.8.4.3's Table 261, Table 255 and Table 238,
  and the sweep is why this round writes 238 where it first wrote 236.
