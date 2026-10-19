# 2025 — A start returns at once, and the wait is the next command

Session 1594, slot 3 of batch ninety-four. Status: **accepted** and **built** in `tools/wait.sh`.
Amends ADR 1976 section 4, under which `--start` started the run and then waited for it. Context:
ADRs 1976, 1977 and 2011; round 1590's report ("`wait.sh --start` around `bounded.sh --lock` did
not return twice").

## 1. What happened, read in the round's transcript

Round 1590's two calls that "did not return" were both moved to the background by the harness:

- `wait.sh --start <log> -- bounded.sh --lock … cargo build …; sleep 5; tail -3 <log>`, with no
  harness timeout, so the harness's default of 120 000 ms against `--start`'s own wait of 240 s;
- `wait.sh --start <log> -- bounded.sh --lock … drive-windows.sh …; wait.sh <log>`, under 270 000 ms,
  where `--start` waited its 240 s and the wait after it 240 s more.

`--start` waited a whole wall after starting, so the form the round's contract writes — `--start`,
then `tools/wait.sh <log>` — was two walls when the two went into one call, as they naturally do.
A locked run is the one that shows it, because a run queued for the lock outlives the first wall.
Reproduced here at scale: a start and a wait of 20 s each, a locked `sleep 60`, a 30 000 ms harness
timeout — the call was backgrounded, and its output shows the two 20 s walls one after the other.

**The brief's hypothesis did not hold**: the detached run does not hold the caller's output. Its
descriptors read from `/proc/<pid>/fd` are standard input from `/dev/null` and output and error to
the log, the harness's output is a file and not a pipe, and the same start under `| cat` closed
the pipe as soon as the caller ended.

## 2. Decision

- **`--start` returns as soon as the run has started**, whatever the command, with exit 0 and the
  line `wait: started pid … into <log>; wait for it with tools/wait.sh <log>`. The wait is the next
  call or the next command of the same call, and one call then holds one wall.
- **A wait's option beside `--start` is refused** (`--wall`, `--tail`, `--until`), on one line with
  exit 64 and nothing started, as ADR 2011 refuses every other misuse: `--start <log> --wall 20 --`
  was the old way to bound the start's wait, and accepting it silently would be a request ignored.
- **The self-test holds the case**: a locked run queued behind another locked run on a scratch
  lock — the holder taken through `tools/bounded.sh --lock` too, as `conformance`'s `bounded` test
  requires of every lock under `tools/` — is started inside two seconds, is still queued at the
  wait after it (75), and ends with its command's status once the holder lets go; and a start and a
  one-second wait in one call take one wall. Planted with the old waiting start, six cases fail, and
  the holder's own start does not return until its command gives up a minute later.

The detached subshell `( … & )` round 1590 used instead still works and is no longer needed.
