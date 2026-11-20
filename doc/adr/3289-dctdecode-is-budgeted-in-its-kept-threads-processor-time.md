# 3289 — `DCTDecode` is budgeted in its kept thread's processor time, in this process

Session 2147. Status: accepted. Context: ADR 0014 (`zune-jpeg` owns `DCTDecode`, in process), ADR 1447 (kept
threads), ADR 3133 (the worker's processor budget), ADR 3150 (the in-process budget), principle 3's time budgets.

## The gap

CCITT, JBIG2 and JPEG 2000 decode under thirty seconds of processor time, in the worker or on a kept thread of
this process. `DCTDecode` ran on its caller's thread with no budget at all, and `zune-jpeg` runs to its end in one
call: a progressive codestream states as many scans over the same coefficients as it likes, and nothing bounded
the time one image could take. It is in process on purpose — it is page one's photograph, and a worker's start
would stand in front of the first page — and it stays there.

## Decision

- `pdf_sandbox::run_here_within(timeout, meanwhile, work)` runs any `work` on the kept threads
  `decode_here_within` already uses, under the same budget: `timeout` of the thread's processor time
  (`SandboxError::Exhausted`, the image's refusal, not counted) behind ten times it of wall clock
  (`Overran`, the machine's, counted). The abandoned-work count and `MAX_ABANDONED` are shared, so one
  image of either kind left running counts against both. In a process behind its own system-call filter the work
  runs on the caller, as `decode` does there.
- `pdf-model`'s `decode_jpeg` runs its decode through it with `REQUEST_TIMEOUT`, now public. Every route to a
  JPEG's samples passes through that function.
- **A caller on a `rayon` thread runs the pool's work while it waits** (`meanwhile`, `rayon::yield_now`, at most
  `MEANWHILE_SLICE` between looks). A banded decode hands bands to the same pool; with every pool thread
  waiting on a decode, nothing would run them and each decode would wait out its ceiling. The unit test
  `a_budgeted_decode_from_a_pool_thread_lets_the_pool_run_its_bands` holds it on a pool of one thread.

## Measured

Release-grade (`gates`) `open_one` on page one, the decode on the caller (before) against on the kept thread
(after), 61 interleaved runs each: `bug1815476.pdf` (the launch path's JPEG row) median 58.9 ms before, 60.5 ms
after, quartiles 55.0–64.0 against 57.2–63.6; `pdkids.pdf` 43.4 against 43.7 ms. Under callgrind with one
`rayon` thread, two runs each: 230.58 M instructions before and 230.40 M after on `bug1815476.pdf`, 10.936 M
and 10.931 M on `issue19634.pdf` (30 JPEGs). No cost is visible on page one.

## Cost

A banded decode spends its bands' time on the pool's threads rather than the kept thread, so for it the
wall-clock ceiling, not the processor budget, is the bound. A panic in the codec now ends its kept thread and is
reported as `Malformed` rather than unwinding into the caller.
