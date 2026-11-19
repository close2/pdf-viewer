# 2108 — A decode is budgeted in processor time, and the waiter's self-test orders events

Session 2108, an instruments round in `pdf-viewer-audit` at `af84fb4b` on round 2103's three load-dependent
results. ADR 3133 is the sandbox's budget, ADR 3134 the self-test's; ADRs 3135-3136 and Q930 unused.

**Premise.** 2103's suspicion held: `REQUEST_TIMEOUT` was thirty seconds of wall clock. A stall injector
(`SIGSTOP` of the test's own sandbox worker once it decodes, 32 s) reproduced all three before the change:
`issue19517.pdf` drew list `6f409240f437a975` and reports `0f7448358dc8a387` ("the sandbox worker did not answer
within the time it was given"), the alternate value exactly; `matte_reduced`'s control drew `[255, 255, 255]`;
`thumbnails` added `issue19517.pdf` to its disagreeing pages. Its worker spends 2.6 s of processor time on the scan
(sampled from `/proc` at a load of 157, `nice 19`, over 17 s of wall clock).

**The budget (ADR 3133).** Thirty seconds of the worker's processor time on the request, read by the parent from
`/proc/<pid>/stat`, `TimedOut` as before; the wall clock is a ceiling of ten times it, refused as the new
`SandboxError::Stalled`, and the whole budget where `/proc` cannot be read. `is_the_machines` names `Stalled`,
`Overran` and `TooManyOverran`; `wall_clock_refusals()` counts them, and `raster_golden` refuses a run with one.
After: the same three stalls draw the file's `11d684437d30a46f 4f53cda18c2baa0c`, and both tests pass. Calibrated
(trap 13): a planted ten-second ceiling and a fifteen-second stall inside the golden check gave "1 image decodes in
this run were refused by the wall clock", exit 101, nothing compared. `confinement.rs`'s deadline test drops its
one-second wall assertion (the variant now carries it); `REFUSED_WITHIN` moves to the in-process test, which also
asserts the count.

**The self-test (ADR 3134).** Both of 2103's cases ("took 3 s", "returned after 3 s") counted seconds. A `bash`
shim that starts the case's `--start` four seconds late fails the old self-test on exactly those two and passes the
new, which checks that the run (ended only by the test's `go` file) had not ended when the call returned, and that
the holder still held the lock. Calibrated: `--start` without its `&` fails both new checks.

**Trap.** A sampler matching `pdf-sandbox-worker` by name finds other rounds' workers; match its parent's name.

**Gates.** Over `-p pdf-sandbox -p pdf-model -p conformance`: fmt 0; clippy `-D warnings --all-targets` 0;
`pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 100 (3420 of 3421: `records`, this record's gates unwritten;
`expected-failures-current.json` lists none), after a first run failed `clock`'s budget test in a 37 ms process whose
reading was zero ticks (both `clock` tests now spin until it moves); rustdoc 0 (101 first: a public doc linked the
private `STALL_FACTOR`); `fuzz` `cargo check --bins` 0; `cargo test -p conformance` 101 (`records`), `--test records` 0.
Golden check 101 with no wall-clock refusal: `issue19517` held; 7 moved that are 2102's and 2104's, unrestated
(`issue3521`, and six reports only), none naming a filter the worker decodes.

**Left.** `--start`'s five-second wait for its run's pid file is still a count of seconds; the in-process path
(`decode_here_within`) stays wall clock, counted as the machine's; an oracle comparison does not yet read
`wall_clock_refusals()`.
