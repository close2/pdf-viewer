# A decoded-stream cache

Status: **priced twice and not taken.** 0.7% over the pdf.js corpus, **78% on one web document** —
recorded so nobody prices it on one population again.
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

## And the price was taken on the wrong population, which is this file's own lesson one step out

**`3129278.pdf` from the `SafeDocs` corpus spends 78% of its 380 G interpretation instructions in
`filter::flate`, inflating one `ICCBased` profile 1053 times** — once per `cs` on a page of 1053
distinct axial shadings (ADR 0288). The 0.7% above is right about the 974 and says nothing about
that page.

So the rule this file states — price an item on the corpus rather than on the page the profiler
opened — needs one more clause: **the corpus is not the web either.** 974 documents assembled from
bug reports have their own shape, and 65 944 crawled pages have another.

**What was taken instead is narrower and is why this item is still open**: a memo of the *parsed
profile*, keyed by the stream's `ObjectId`, in `Interpreter::icc_spaces`. It needs no eviction
argument, no liveness invariant and no bound on decoded bytes — the three things this item still
owes — and it took that page from 34 450 ms to about 1 550. A general decoded-stream cache would
have taken it further, and the same distance costs the three arguments above.
