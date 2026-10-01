# 1447 — Three loops of a symbol dictionary are bounded in a patch, and an in-process decode has a deadline

Session 1306. Status: **accepted**; the in-tree half **built**, the codec half **written and not
applied** — it is the owner's to apply to the fork.
Context: `crates/pdf-sandbox/src/in_process.rs` (new), `crates/pdf-sandbox/src/lockdown{,_linux}.rs`
(`is_confined`), `crates/pdf-sandbox/src/lib.rs`
(`Sandbox::with_timeout`, `SandboxError::{Overran, TooManyOverran, Thread}`),
`crates/pdf-sandbox/tests/{in_process_deadline.rs, symbol_dictionaries/mod.rs, confinement.rs}`,
`fuzz/fuzz_targets/{jbig2,jpx}.rs`, `doc/patches/hayro-jbig2-symbol-dictionary-bounds.patch`.
Amends: ADR 1424 section 3 (the budget was the worker's deadline alone). Clauses: ISO 32000-2 §7.4.7;
ITU-T T.88 (08/2018) sections 6.5.5, 6.5.10, E.3.4 and Table 15, cited and paraphrased, now held
(`doc/third-party-data.md`).

## 1. The cause, read against T.88 rather than guessed

Round 1293's seven inputs were gone from every disk, so the `jbig2` target was run again for ten
minutes behind the lock: 28 timeouts and one slow unit. Every one is inside the symbol dictionary
decoder, and there are **three** causes, not the one expected:

- **Empty height classes (24 inputs).** Section 6.5.5 step 4 loops over height classes until the
  count of decoded symbols reaches SDNUMNEWSYMS; a class whose first delta width is OOB decodes no
  symbol; section E.3.4 feeds the arithmetic decoder 1-bits once its data is exhausted, for as long
  as decoding continues. The 1 624-byte input of ADR 1424 decodes 21 of 27 symbols and then one
  empty class after another, each one taller by one, until HCHEIGHT leaves the 32 bits Table 15
  gives it — about 36 million classes a second here, two minutes to 2^32. With a delta of zero it
  would never end.
- **Zero-length export runs (2).** Section 6.5.10 repeats until EXINDEX reaches the symbol count,
  and step 3 allows a run of length zero, which does not advance it. A dictionary of three symbols
  decoded eleven zero-length runs at index 0 and does not stop.
- **Symbol pixels (3).** A symbol is bounded only by its width's and height's 32 bits; a 624-byte
  input defines 6.9 billion symbol pixels in 5 190 symbols for a 50x178 page.

**The standard bounds none of the three**: it neither requires a height class to hold a symbol nor
limits zero-length runs, and it limits a symbol's size only through the integer types. Each bound
below is therefore this program's choice, stated with its number.

## 2. The bounds, and the census they sit above

A census of every JBIG2 stream the corpus holds — 54 498 distinct streams, 34 482 of them with
symbol dictionaries, 1 537 956 height classes, run behind the lock through the pinned codec with
counters — gives each bound its margin:

| bound (in the patch) | what real streams reach | margin |
|---|---|---|
| empty height classes ≤ SDNUMNEWSYMS | **0** in every stream that decodes (6, in two streams refused anyway) | total |
| zero-length export runs ≤ symbols + 1 | **1** per dictionary at most — the leading run that exports symbol 0 | total |
| symbol pixels per decode ≤ 2^28 | **41 843 627** (a 4256x6258 page) | 6.4x |

**The bound the brief proposed would have refused real files.** A symbol bitmap larger than the page
region it is placed in is not pathological: `/JBIG2Globals` dictionaries serve every strip of a striped page, and
the census found a symbol 12.8 times its page's area and a dictionary 474 times it. The pixel budget
is absolute for that reason. The census's slowest real stream decodes in 0.92 s; its median, 7.5 ms.

## 3. Where the fix lives

`hayro-jbig2` at the pinned revision exposes no limit and no cancellation: `Image::decode` runs
every segment in one call, and the `Decoder` trait is called only after the page is composed
(read in the checkout, per the habit of reading a dependency's public API first). The bounds are
therefore a patch to the codec: `doc/patches/hayro-jbig2-symbol-dictionary-bounds.patch`, against
`64efcaca`, three new `SymbolError` variants, `git apply --check` clean. **It is not applied here**:
the dependency is the owner's fork (`Cargo.toml`; ADR 0014 makes a disagreement with T.88 an issue
to report rather than a defect to fix here), a round cannot push to it, and vendoring a patched copy would be a second copy of a
codec with no rule in `doc/stack.md` admitting it. ADR 1349's precedent — the tree writing its own
decoder — is not this case: that was a capability the codec lacked an API for, this is three bounds
of a dozen lines each. Applied to a scratch copy, the patch refuses all 29 inputs in 0.27 ms to
0.52 s, a ten-minute fuzz run over it finds no timeout, and the patched census decodes every
real stream exactly as the pinned one does.

**The in-tree half is the deadline, on both paths.** The confined path already killed the worker at
`REQUEST_TIMEOUT`; `Sandbox::with_timeout` states a shorter one, which the regression test uses
(250 ms, each input refused inside a second, the next request answered). The in-process path had
no bound at all. A thread cannot be killed, so `decode_here_within` bounds the **caller's wait**: the
decode runs on a kept thread, an overrun is `SandboxError::Overran` and the decode is *abandoned* —
it runs on, holding its memory, possibly for ever. At `MAX_ABANDONED` (two, a choice) still running,
further in-process decodes are refused with `TooManyOverran` rather than spend a core each.

**Kept threads rather than one per decode, measured**: a new thread per decode put the median of a
137 KB stream from 22.1 ms to 30.1 ms and of a 1.2 KB stream from 207 to 367 µs (a cold allocator
arena each time); a kept thread measures 21.5 against 21.3 ms and 206 against 199 µs.

**A process behind its own filter does not take that path.** The confined viewer (`viewer-confined`,
`pdf-vfs`) decodes in process because it can start no worker (ADR 0218). The first build sent its
decodes to the kept thread, and the merge's tier-3 walks (`awkward_classes`, `read_corpus`) found
every JPEG 2000 and JBIG2 document killed by `SIGSYS`: the kernel's audit line names system call
157, `prctl` — Rust names a thread it starts, and `lockdown_linux.rs` keeps `prctl` off the list on
purpose. Widening the filter would be the wrong answer, and so would dropping the name: a new
thread's first allocation can still ask glibc's arena question of the kernel, which the filter
also kills for (`crates/viewer-confined/tests/confined.rs`, ADR 0218). And the deadline is not owed
there: a confined process is bounded from outside, by the host holding its `Canceller` and by its
address-space ceiling (ADR 0241). So the confinement records that it was installed
(`lockdown::is_confined`, set by `lockdown_linux::apply` once the filter is in), and `decode` sends
a confined process's in-process decodes to `decode_here`, on its own thread, as before.
`confinement.rs`'s `a_confined_process_decodes_in_process_on_its_own_thread` holds it — it fails,
with the same `SIGSYS` on `prctl`, when the routing is removed — and both walks are clean again.

**What does not change.** The fuzz targets call `decode_here`, on their own thread: libFuzzer's
`-timeout` is their bound, and the in-process deadline would abandon a slow input at thirty seconds
and then refuse every input once two were running — a fuzzer that stopped decoding. So the
tree's `jbig2` target keeps finding these timeouts until the patch is applied — that is the
evidence the patch is still owed, not a defect of the deadline.

## 4. What is owed, and by whom

- **The owner**: apply the patch to `close2/hayro`, bump `Cargo.toml`'s `rev`, and the three
  sentences reach the page as `JBIG2: …` refusals; `symbol_dictionaries::ended` already admits them.
- **Evidence for the same fork, not acted on**: of T.88's ten Annex K conformance streams, the
  pinned codec decodes two; the other eight are refused with or without the patch.
