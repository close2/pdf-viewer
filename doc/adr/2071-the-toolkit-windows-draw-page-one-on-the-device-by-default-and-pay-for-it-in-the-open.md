# 2071 — The toolkit windows draw page one on the device by default, and pay for it in the open

Session 1617, slot 3 of batch ninety-seven. Status: **accepted**. Context: ADR 2049 section 4, item
2 ("Whether the device is the toolkits' *default* lane is for G1's round to decide, on the toolkit
launch figures with the bring-up in them. This record recommends the device, for principle 2's
reason"); ADR 2070, which built the lane; `CLAUDE.md` principle 2, *Startup time is a first-class
requirement*; ADR 1831 and `doc/questions/A393`.

## 1. The figures

One `--clock` hold, the machine otherwise idle, five rounds interleaved per window and lane, each a
cold process on `doc/PDF20_AN001-BPC.pdf` under Xvfb at 1400×1100 with `--trace=launch,frames`
(`scratchpad/r1617/launches.sh`). The device line in every device launch read "AMD Radeon 890M
Graphics (RADV STRIX1) (IntegratedGpu, Vulkan)": the read-back device has no surface, so it takes the
real adapter under Xvfb where `quorra`'s surface binds llvmpipe (habit 113).

| window | first frame, device | first frame, `--cpu` | page one drawn, device | page one drawn, `--cpu` | device up |
|---|---|---|---|---|---|
| `quorra-gtk` | 143–153 ms, median 146 | 128–154 ms, median 138 | 13.0–13.5 ms | 3.5–4.0 ms | 36–49 ms |
| `quorra-qt` | 132–142 ms, median 135 | 95–107 ms, median 98 | 11.9–12.6 ms | 3.3–6.8 ms | 53–64 ms |

The device costs GTK about 8 ms of time-to-first-page and Qt about 37. **Where it goes is two
different places.** In GTK it is the page: the device's first page is 13 ms against the
processor's 3.5–4, and the window was built at the same moment in both lanes. The device's second
page is 6.0–6.6 ms against the processor's 3.5–4.4, so on a page this small the device, read back,
is the slower lane warm as well as cold — ADR 2049's 596×842 row measured the reverse, headless and
without a toolkit beside it, which is a difference this record states and does not explain.
In Qt about 30 ms of it is *before* the page: the first resize came at 114 ms on the device and 84
ms without it, so the bring-up on its own thread is contending with Qt's own start-up, and the page
adds the rest.

## 2. The decision

**The device is the toolkits' default lane, and `--cpu` is the word that takes it away**, as it is
`quorra`'s. Principle 2 is the owner's, and it is explicit: page one goes to the graphics device by
default, its bring-up is on the critical path by choice, and the processor takes page one only where
it is predicted the faster lane — which ADR 1831 and A393 put past `WARN`. A small page that the
processor draws in four milliseconds is inside `WARN` on either lane, so the rule gives it to the
device, and this record does not write a second rule for the toolkits.

What principle 2 turns into an obligation is the figure, and it is written down rather than hidden:
**8 ms in GTK and 37 ms in Qt on a small page, on this machine.** A later round that takes the Qt
figure down profiles first (habit 59): whether the 30 ms before the resize is the Vulkan loader,
RADV's own start or processor contention, and whether starting the bring-up after Qt's first resize
moves the page later by less than it gives back. That is not taken here, because the bring-up beside
the read is what ADR 2049 asked for and moving it is a decision about a measurement nobody has made.

## 3. What is not gated

Cold bring-up is its own gate for `quorra` and the toolkits have none: `launch_path` measures
`quorra` alone. The table above is a round's measurement, not a band, and `--cpu` beside it is how a
later round repeats it.
