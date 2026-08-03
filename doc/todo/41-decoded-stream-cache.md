# A decoded-stream cache

Status: **priced at 0.7% and not taken.** Recorded so nobody prices it again.
Priority: 41
Code: `crates/pdf-syntax/src/filter.rs`, `crates/pdf-syntax/src/document.rs`

Measured over one interpretation of every corpus page (session 128): 6220 inflations of 38.08 MB.
Among the streams above 4 KB — 722 calls, 35.0 MB, 92% of the bytes — **35 are repeats costing
925 KB, 2.6%**. So a decoded-data cache is worth about **0.7% of interpretation**, against a real
memory cost, a bound to argue for, and a liveness invariant to write down.

Below 4 KB the count is worthless: an address freed with one document is handed to the next, so
"the same stream twice" cannot be told from "two streams at one address".

**The benchmark page is not representative**, and that is the lesson worth more than the number:
one 88 KB font program inflated twice is 58% of *that* page's inflation and 2.6% of the corpus's.
Price an item on the corpus, not on the page the profiler happens to open.

Take it only if something else makes a decoded-stream table exist anyway — a moving window of
interpreted pages (`doc/RENDER_LIBRARY.md`'s residency plan) would.
