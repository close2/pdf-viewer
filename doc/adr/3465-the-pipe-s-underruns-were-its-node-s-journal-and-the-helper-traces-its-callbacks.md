# 3465 — The pipe's underruns were its node's journal, and the helper can trace its callbacks

Session 2191. Status: accepted. Context: review 2185 and its round-2189 section ("run 6 spent about
2.2 s a minute in underruns at every load from 15 to 49 … not explained"); ADRs 3441, 3457, 3459;
RFC 0015 section 9.1.

## Decision

1. **The helper traces its callbacks on request.** Where `QUORRA_SOUND_TRACE` names a directory,
   each pass's stream records every callback — when it began on the wall clock, the gap since the
   last one ended, how long reading the sound took, how long the sound's lock took to take, the
   frames asked and given, the frames of the sound arrived, and the delay `cpal` stated — into a
   buffer allocated before the first callback, and writes it there as `pass-<pid>-<ns>.tsv` when
   the stream is dropped. Unset, no trace exists and the callback reads no clock for one.
   `AUDIO_IN_USE_TRACE=1 tools/audio-in-use.sh` sets it, and the test writes the device's
   underruns and its reader's own late reads beside it as `device.tsv`, on the same wall clock.
2. **The stand-in's named pipe is made on a memory file system** (`$XDG_RUNTIME_DIR`, else
   `/dev/shm`), never beside the readings on the scratch disk.
3. **A run is copied into its shared slice before `Held`'s lock is taken**, not under it.

## What the trace showed

Three traced pipe runs of a 60 s sound played four times (2026-11-23, loads 13–55):

- With the pipe's node on the scratch disk (ext4 on a rotating disk), every long underrun had a gap in
  the helper's callbacks beside it, the underrun plus 15.7 ms (the pipe's 10.7 ms and a 5 ms period)
  long, while every callback itself took microseconds (99.9th percentile 38 µs). Run 1: 65 of 73
  underruns, 9.5 of 9.6 s, beside such a gap; the longest 655 ms beside a gap of 670 ms.
- A millisecond sampler of the stream thread's `/proc` state named where it was: in state `D` at
  `do_get_write_access` — ext4's journal — for 6.8 s of the gaps over 20 ms in run 2, against 0.9 s
  in `pipe_write`. A write into a named pipe updates its node's times; on a journalled file system
  the writer then waits for the journal's commit, which comes every five seconds — the gaps came
  about five seconds apart — and lasts as long as the disk takes under the other lanes' load. That
  is why the figure followed the disk rather than the load average, and why 2185 saw less of it.
  The slice's memory events and pressure in the same gaps were nil.
- With the node on a memory file system (run 3, load 21–56), the long sound's underruns fell from
  1.0–2.2 s a minute to 0.1–0.7 s, the longest from 655 ms to 26.5 ms. What remains is the
  scheduler's: the pipe is one page, which the writer may refill only once the reader has emptied
  it, so each 10.7 ms the device's buffer is empty and a wakeup 2 ms late is an underrun; 345 of
  534 had no helper gap over 12.7 ms beside them. The test's own reader woke more than 2 ms late
  10 147 times in the same run.
- The one long lock wait seen (150 ms) was the trace's own reading of `Held` while the sound
  arrived; the copy under the lock is removed all the same, since the callback takes that lock
  whenever it reaches the sound's arriving end.

## Consequences

- The pipe's earlier underrun figures (2185's and 2189's) measure the scratch disk's journal as
  much as the helper; RFC 0015 section 9.1 says so beside them.
- The trace costs nothing unset; set, 9 MiB a pass and a file per pass.
