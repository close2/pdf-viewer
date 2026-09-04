# 922 — Principle 2's four numbers get an instrument, and a band needs a machine under it

2026-09-04. Argued in
[ADR 0884](../adr/0884-the-launch-path-is-a-gate-and-a-wall-clock-band-needs-a-machine-under-it.md)
(the gate) and
[ADR 0885](../adr/0885-what-the-launch-path-costs-and-which-of-principle-2s-claims-hold.md)
(what it found). On its own branch, beside rounds 919 and 920.

`CLAUDE.md` principle 2 names four numbers — cold open, time-to-first-page, page-turn latency,
memory high-water — and a fifth, cold graphics bring-up, that it makes a gate of its own. Nine
hundred rounds in, **no command in this tree printed any of them.** This round built the one that
does, and then read the principle's own claims off it.

Touched: `crates/viewer-ui/tests/launch_path.rs` and `doc/checks/launch-path.toml` (both new);
`tools/state.sh` (a `launch` section), `doc/todo/02-every-round.md` (§2's sequence and its map),
`doc/performance.md`, `doc/habits.md` (*Measuring*), `doc/environment.md` (the machine),
`doc/PLAN.md` (a phase-4 line that contradicted principle 2), `doc/todo/42`; two ADRs, this file.
**No crate source changed**, which is the second finding: nothing on the launch path needed
fixing, and what was missing was the instrument.

## 1. The machine has two classes of core, and every duration here was a lottery

This is the finding that made the rest possible, and it was met by accident. The calibration probe
— a fixed, serial, in-memory piece of this tree's own work — measured **0.75 ms in some processes
and 1.50 in others, on a machine whose load average was under two**. `cpuinfo_max_freq` says why:
an AMD Ryzen AI 9 HX 370 has four Zen 5 cores at 5.16 GHz and eight Zen 5c at 3.29.

Every wall-clock figure this project has ever taken was drawn from that lottery, `doc/performance.md`'s
launch timelines included, and nobody knew it was one. Pinned with `taskset` to a core list
*derived* from `/sys` — never written down — and taking the minimum of nine fresh processes, the
gate's spread fell from **100–400% to 0.6–22%**. That is the whole difference between an
instrument and a gate, and it is now in `doc/habits.md`'s *Measuring* section and
`doc/environment.md`'s machine list.

## 2. What makes a wall-clock gate believable, checked against the defect

`doc/todo/02` §2 records three false failures from neighbours' load, so the gate had to answer
that before it could be trusted. Five answers, in ADR 0884: the minimum of nine fresh processes;
pinning every one of them to the machine's fastest cores, on a list derived from `/sys`; a
calibration probe with a band of its own that decides whether the *clock* figures are judged at
all; two figures with no clock in them, judged always; and a check that the profile is `release`,
because `[profile.gates]` costs `Document::open` 4.06% to 12.30% and that is wider than the
bands.

Verified against the defect rather than assumed (trap 13): with eight busy threads pinned onto the
gate's own eight CPUs — deliberately confined there so the neighbouring rounds kept the other
sixteen — the probe read **0.955 ms against a band of 0.62 .. 0.82**, the gate printed `NOT
JUDGED`, and **twelve of twenty-eight figures were outside their bands**. Every one would have
been a false failure. Not one of the deterministic figures moved. The failure path was checked in
the other direction too, by narrowing two bands by hand and watching the gate name the figure, the
value and the band.

**And one figure was demoted by its own evidence, which is the round's third finding.** The
memory high-water of a process that has brought a graphics device up was identical across all
forty-four derivation runs, so it was banded as tightly as the two figures with no clock in them —
and an hour later, same tree, same binary, idle machine, all four rows had fallen together by
about 12% and the gate failed on all four. The driver's allocation had moved; the memory figure
with *no* device in it had not moved by a kilobyte. "Deterministic" turned out to be a claim about
a population of runs, and forty-four consecutive ones is not a wide one — so that figure is now
judged like a duration, with a band spanning what has been observed, and the fine memory figure is
the one with none of the driver in it.

The cold page cache principle 2 asks for by name is `dd … oflag=nocache`, which is
`posix_fadvise(POSIX_FADV_DONTNEED)` and is available to an unprivileged user, on **a copy** the
gate makes under the build directory — never on a file in `doc/`, because dropping a file's cache
means opening it for writing. It was checked before it was believed: 19 MB read in 16–26 ms cold
against 3–7 ms warm.

## 3. Which of principle 2's claims hold

Four of five hold. *Nothing eager* holds for configuration, recent files, thumbnails and the page
tree — `strace` over an open of the 1023-page specification finds **eleven `openat` calls in the
whole process, one of them not a shared library: the document**. *No parsed data at startup* holds:
612 `static` key tables in `.rodata`. *Incremental parsing* holds, and it is the one claim that
*changed* — session 881's on-disk reader (ADR 0809) made it true of the bytes where it had been
true only of the parse. *Nothing waits for warmth* holds, and `--trace` says so in its own words.

Two do not hold as written, and both are the owner's to word:

- **"No system font enumeration"** is false without a condition. A page naming a font it does not
  embed walks the machine's font directories on the launch path and roughly doubles time to first
  page. It is *needed to show page one*, so it is not eager by the rule's own definition — but the
  sentence has no such qualifier. The gate carries such a document as its fourth row, so the cost
  has a band rather than a sentence.
- **"A 500-page document must open no slower than a 5-page one"** is false of the open by about
  thirty times and true of the launch, because `document joined` and `device up` are the same
  figure in every single run: the document thread finishes before the graphics device does.

Round 921 has already filed principle 2's wording as a question for the owner; these numbers
belong beside it, and this round therefore opened no second `Q` file and amended no principle.

## 4. What the numbers are

They are not written here. `tools/state.sh launch` prints them, `doc/checks/launch-path.toml`
holds what they may be, and ADR 0885 holds the readings a command cannot produce. One of them is
worth knowing without running anything, because it points at where any future work belongs:
**the memory high-water of this program is the graphics driver, not the PDF** — opening the largest
document this project owns costs a fraction of what bringing a device up does.
