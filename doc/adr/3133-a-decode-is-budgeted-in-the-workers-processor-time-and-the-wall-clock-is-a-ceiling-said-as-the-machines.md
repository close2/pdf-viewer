# 3133 — A decode is budgeted in the worker's processor time, and the wall clock is a ceiling said as the machine's

Session 2108. Status: **accepted.** Context: ADR 0241 (the confined worker), ADR 0194 (the deadline on three
platforms), ADR 1447 (the in-process deadline), ADR 1507 (no clock inside `interpret`), ADR 3114 (a test waits for
an event), `crates/pdf-sandbox/src/clock.rs`.

## 1. The failure, and its cause

Under a load of 155 to 258 with the user slice at its `MemoryHigh`, round 2103 saw `raster_golden` draw
`issue19517.pdf` as `6f409240f437a975 0f7448358dc8a387` (list, reports) in one run and the file's
`11d684437d30a46f 4f53cda18c2baa0c` in every other, and `pdf-model`'s `thumbnails` and `matte_reduced` fail in one
nextest and pass alone. `REQUEST_TIMEOUT` was thirty seconds of **wall clock**, and the parent killed the worker at
it.

The proof, by a stall injector (`SIGSTOP` of the test's own sandbox worker once it is decoding, `SIGCONT` 32 s
later), in the tree before the change:

| stalled | outcome |
|---|---|
| `issue19517.pdf`'s decode, alone | list `6f409240f437a975`, reports `0f7448358dc8a387`, the report "Im0: the sandbox worker did not answer within the time it was given": the alternate value exactly |
| `matte_reduced`'s control decode | "with no /Matte the samples are drawn as they stand, so the page can tell: [255, 255, 255]", 2103's message |
| `thumbnails`' decode of `issue19517.pdf` | `issue19517.pdf` joins the pages whose miniature disagrees, 2103's message |

The decode is not slow. `issue19517.pdf`'s worker spends under three seconds of processor time on it (2.6 s
sampled from `/proc` at a load of 157 and `nice 19`, over 17 s of wall clock), so a page lost its image because the
worker was given less than a tenth of a processor for thirty seconds. The page was a fact about the machine, which
is the property ADR 1507 refused to give `interpret` and the oracle and the golden rest on.

## 2. The decision

- **The budget is the worker's processor time on the request**, read by the parent from `/proc/<pid>/stat`
  (`utime + stime`) when the request is sent and while it waits; still thirty seconds, still enforced from outside
  by a kill, and `TimedOut` stays its refusal and its sentence. The worker is not trusted to time itself (it is
  read, not asked), and the reading at the request's start is subtracted, so the old objection to a processor
  limit — a long-lived worker killed for its past — does not arise. The parent sleeps in `poll` for the processor
  time left, which a one-thread worker (its filter refuses `clone`) cannot outspend, so no timer is added.
- **The wall clock stays, as a ceiling of ten times the budget, and its refusal is its own variant**,
  `SandboxError::Stalled { after }`: "a bound the machine's load decides as much as the image does". It bounds a
  worker that waits without computing — stopped, starved, or subverted — which spends no processor time; principle
  3's explicit time budget is kept. Where `/proc` cannot be read (no Linux, or a parent not allowed), the wall clock
  is the whole budget at thirty seconds as before, refused as `Stalled` because that is what it is.
- **A caller that must be a function of the bytes can tell.** `SandboxError::is_the_machines` names `Stalled`,
  `Overran` and `TooManyOverran` (the in-process path's wall-clock refusals, unchanged), and
  `pdf_sandbox::wall_clock_refusals()` counts them in the process. `raster_golden` refuses to compare or to write
  a run that counted one; a count rather than a mark in the page's report, because the report is the document's
  and its words are what the golden digests.
- `confinement.rs`'s deadline test keeps its kill and drops its one-second wall assertion: a quarter-second of
  processor time takes longer than a second on a loaded machine, and `TimedOut` is now returned only by the
  processor budget, so the variant carries the assertion. `matte_reduced`'s control states that its image was
  drawn, so a lost decode names itself rather than a white pixel.

## 3. After

The same three stalls, after the change: `issue19517.pdf` is the file's `11d684437d30a46f 4f53cda18c2baa0c`
with no report; `matte_reduced` and `thumbnails` pass (37.6 s and 40.4 s, the stall inside each). Calibrated
(trap 13): with the ceiling planted at ten seconds and the golden check's worker stalled for fifteen, the check
stopped with "1 image decodes in this run were refused by the wall clock, not by their processor budget", exit 101,
before anything was compared or written.

## 4. The cost

A worker on a loaded machine is waited for longer — up to three hundred seconds of wall clock where it was thirty
— before a caller hears it is stalled; an image that genuinely spends thirty seconds of processor time is ended at
the same thirty on an idle machine as before. On a platform with no `/proc` nothing changes but the variant's name.
Reclaim under memory pressure is charged to the worker as system time, so a starved worker's reading can rise a
little with the load; `issue19517.pdf`'s eleven-fold margin under its budget is what makes that immaterial here.
