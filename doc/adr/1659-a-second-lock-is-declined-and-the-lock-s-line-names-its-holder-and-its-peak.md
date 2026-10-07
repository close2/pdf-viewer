# 1659 — A second lock is declined, and the lock's line names its holder and its peak

Session 1411. Status: **accepted** and **built**. Answers the question ADR 1646 section 1 left
open ("whether a second lock is wanted is the review's question, re-asked on these lines"), on the
first batch of lines. Supersedes nothing; section 4 is a defect in ADR 1646 section 1 found and
left open, with its cost.
Context: `doc/reviews/1401-are-we-making-progress-and-what-a-round-reads.md` (the "two heavy-walk
locks" row, declined for want of a count); ADRs 0798, 1612, 1646, 1662; traps 110 and 116.
Code: `tools/bounded.sh` (`lock_holder`, `lock_take`, `lock_record`, self-test case 7).

## 1. What batch sixty-three paid, and behind what

Batch sixty-three ran from its open at 14:42 to its merge's last gate at 17:18, about 2.6 h.
`/home/AI/heavy-walk.log` holds eight lines for it, all round 1405's, because only that round's
wrapper had `--lock`; the others took the lock with a bare `flock`, which writes no line.

| asked | queued | held | behind |
|---|---|---|---|
| 14:42:28 | 159.6 s | 5.5 s | not on the log (the open's arms export was queued behind it too, 59 s by its README) |
| 14:45:56 | 1 762.3 s | 81.8 s | the open's arms export: granted at 15:15:18, the second its README says it ended |
| 15:20:16 | 1 678.7 s | 457.5 s | not on the log |
| 15:58:22 | 447.8 s | 19.9 s | not on the log |
| 16:06:34, 16:32:12, 16:32:31 | 0 s | 1 514.9, 18.8, 3.3 s | — |
| 16:33:06 | 460.0 s | 18.8 s | not on the log (the round's own previous run ended at 16:32:34) |

So round 1405 queued 4 508.4 s and held 2 120.5 s: 1 762.3 s of the queue behind the export and
2 746.1 s behind holders no line names. The records add what the log cannot: round 1402 about
1 080 s and about 1 560 s, round 1406 261 s; 1403, 1404 and 1407 state no queue. Batch
sixty-three's known queue is therefore about **7 410 s, by three of six rounds**.

`tools/state.sh gates-cost` printed 2 586.5 s for 1405, not 4 508.4: the first two lines say
`batch=HEAD`, because that round called a private export's own wrapper, and `lock_cost` reads the
last batch's branch only. That is `tools/state.sh`'s, reported to its owner rather than changed here.

## 2. Whether a second lock for walks under 6 GiB would have been safe

**On memory, yes, if the lane's ceiling is a kill rather than a promise.** The walks' recorded peaks
over the process tree: the transform corpora 1.50 to 3.33 GiB (record 1009), `pdf-model --test
corpus` 2.14 (1167), `render-raster --test corpus` 5.10 at scale 1 (1329) and 10.7 with
`issue19517` compared whole (1318), the oracle at scale 4 6.54 (1070), `foreign_corpus` 6.70 (1009),
a crawl 8.4 (1375); this round's own bring-up runs under the lock 0.01 to 0.07. The incidents:
2026-09-01, eight shards of about 12.9 GB each; 2026-09-02, the slice at 61.09 GB of 61.9 with one
walk allowed 32 GiB beside builds, so about 29 GB was everything else; 2026-10-06, 52 259 tasks, a
count no memory bound sees. A 12 GiB lane and a 6 GiB lane are 18 GiB of walks; beside 2026-09-02's
29 GB of everything else that is about 47 GB, under the machine's 61.9. It holds only under
`--tree 6`, which kills the tree at the ceiling. Tasks are a separate budget: the agent's cgroup's
`pids.peak` reads 8 161 against the 8 192 that `ulimit -u` allows, so a second walk's threads make
a refused fork more likely somewhere. That fails one command, and it does not take the machine down.

**On the clock, no, and that decides it.** Every one of 1405's queued runs was a clock run: the
launch gate's children with `PDFVIEWER_LAUNCH_CLOCKS`, ten gate runs, an interleaved A/B. A clock
run beside another walk measures the busy end of trap 110. So under two lanes such a run must take
both, and `flock` gives a waiting exclusive holder no priority over a stream of small ones. Of the
7 410 s known, a second lane could have shortened at most the 2 900 s of rounds 1402 and 1406, if
their walks were under 6 GiB and their holders were not clock runs. It could not have shortened
1405's 4 508 s at all, and it would have made every measuring and pixels round's wait longer.

## 3. Decision

**No second lock.** What would justify one is a count the log could not give until now: the queue
of runs that peaked under 6 GiB, were not clock runs, and queued behind a holder that was not one
either. **The line now carries both facts it lacked**:

    <asked> batch= round= wait= hold= exit= peak=<GiB> behind=<holder> cmd=<first 200 bytes>

- **`peak=`** is the wrapper's own process-tree peak, the figure its last line already printed.
- **`behind=`** is the run found holding the lock when this one queued, as one word, or `-` when
  it did not queue. A bare `flock <lock> <command>` keeps `flock` alive as the command's parent, and
  `/proc/locks` names that process. A `--lock` wrapper's lock was taken by a `flock -n <fd>` that
  has exited, and `/proc/locks` keeps that dead pid, so the wrapper writes `<pid> round=<N> <command>`
  to `<lock>.holder` once it holds and removes the file before it lets go. The live pid is read
  first, then the file, whose pid must still be running. The lookup runs only on a queue, so a free
  lock costs nothing.

The fields go between `exit=` and `cmd=`, so `lock_cost`'s fields one to six and its `cmd=` stay
where they are. Its first live line: this round queued 116.3 s `behind=round=1410_…/r1410/arms/…`.

**Calibrated** (trap 13): with `lock_holder` returning `unknown`, case 7's shape check fails on the
bare `flock` holder; with the holder file never written, the sub-case queued behind a `--lock`
holder fails. The case's runtime grows by about 2 s.

**Re-ask**: once a batch has every walk on the log (ADR 1662 moves `tools/batch.sh` and the rule
line to `--lock`), sum the queue described above. Over **3 600 s in a batch**, ten minutes a round,
a second lane is worth building. It would be a separate lock under `--tree 6`, with clock runs taking
both. That threshold is a choice: below it, a round loses less waiting than a starved measuring
round loses.

## 4. A daemon a walk starts can hold the lock after the walk: found, not fixed

ADR 1646 passes the lock's descriptor to the command, as `flock <lock> <command>` does, so that the
lock stays held while anything the walk started still runs. During this batch that held the whole
machine's lock for a cache server. Round 1412's `gates.sh` was granted the lock at 19:17:32. Its
`cargo` found no `sccache` server running and started one that second, and the server kept
descriptor 10 on `/home/AI/heavy-walk.lock`. The wrapper ended at 19:18:39 and the lock stayed
held. `/proc/locks` named a dead pid, and the only open descriptors were the server's and the
waiters'. Rounds 1408 and 1411 had been queued since 18:09 and 18:35, and they waited until the
server idled out at about 19:37, about eighteen minutes. A bare `flock` behaves the same; its `-o`
exists for exactly this. The cache server outlives the walk by design, so any build inside the lock
can start one, and trap 109's rebuild inside the lock is such a build.

**The fix was built and taken out again.** The command's subshell closed the descriptor, and a
signalled wrapper stopped its command's tree first, so the lock was still held exactly as long as
the command ran. Both were calibrated: a `sleep` the command left running no longer held the lock,
and a signalled wrapper's command no longer outlived it. But ADR 1662's `arms-held`, built in the
same batch, proves it runs under the lock by finding the lock's descriptor in its own `/proc/$$/fd`.
The fix removes exactly that descriptor, so `batch.rs`'s
`open_returns_while_the_warm_build_and_the_arms_export_run_detached` failed. Two siblings' files
cannot both be right about the descriptor, so the wrapper is left as ADR 1646 built it.

**Owed, to `tools/bounded.sh` and `tools/batch.sh` together.** The wrapper should export a marker,
for example its own pid as `HEAVY_WALK_HELD_BY`. `holds_the_lock` should accept that marker when
the pid is an ancestor that has the lock's descriptor open. Then the command's subshell can close
the descriptor. Until then, a round that finds the lock held by no running wrapper should check
whether the holder is a daemon (`ls -l /proc/*/fd 2>/dev/null | grep heavy-walk.lock`) and say so.
It should not kill a shared server to free the lock.
