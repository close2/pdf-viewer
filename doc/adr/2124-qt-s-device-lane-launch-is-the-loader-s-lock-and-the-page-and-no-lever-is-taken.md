# 2124 — Qt's device-lane launch is the loader's lock and the page, and no lever is taken

Session 1644, slot 6 of batch one hundred. Status: **accepted**. Context: ADR 2071 section 1 ("about
30 ms of it is *before* the page: the first resize came at 114 ms on the device and 84 ms without
it") and section 2 ("A later round that takes the Qt figure down profiles first"); habit 59;
`CLAUDE.md` principle 2's three rules for the device on the launch path; ADR 2125, whose hold the
figures below share. It cites no clause and moves no ledger row.

## 1. What was measured, and with what

Every figure is HEAD's export (`aa74b695`) on `doc/PDF20_AN001-BPC.pdf` under `Xvfb`, pinned to the
eight fast CPUs, the device lane against `--cpu`, nine launches a lane interleaved in the one
`--clock` hold of ADR 2125 at a one-minute load of 9.1 to 10.4. Three instruments, each answering a
question the clock alone cannot:

- **Where the window's thread spends its time before the first resize**: the kernel's own
  per-thread counters (`/proc/<pid>/task/<tid>/schedstat`: time on a processor, time waiting for
  one) sampled every 0.2 ms from outside the process, and the rest of the elapsed time is time the
  thread was blocked. Medians of nine:

  | Qt, at the first resize | device lane | `--cpu` |
  |---|---|---|
  | first resize | 64.0 ms | 55.0 ms |
  | on a processor | 52.1 ms | 49.3 ms |
  | blocked | 10.6 ms | 2.5 ms |
  | the device's thread, on a processor | 25.9 ms | — |
  | first frame | 79.0 ms | 62.0 ms |

- **What it was blocked on**: `strace -f` of one launch of each lane. The window's thread waits
  10.9 ms on one futex as it asks to load Qt's platform plugin (`libqxcb.so`), while the device's
  thread holds the same address loading `libvulkan_radeon.so` and the `libLLVM.so.23.1` it links;
  23.4 ms later the device's thread waits on it in turn while Qt loads its platform theme. That
  futex is the dynamic loader's own lock, which every `dlopen` in the process takes for its whole
  map and relocation. Without the device the window's thread never waits there.
- **Whether the window's thread does more work beside the device**: callgrind, each lane, the
  window's thread counted to the first call of `resized`: 515 528 206 instructions on the device
  lane and 515 455 697 without it, 0.014 % apart. The 2.8 ms more it spends on a processor is the
  same work running slower beside the device's thread, not work the device causes.

## 2. The reading

**The brief's hypothesis does not hold.** It read the 30 ms as the adapter's request and the
surface's creation on the GUI thread; neither is there. `viewer_device::renderer::ReadBack::
bring_up` already spawns the device's own thread before Qt's application exists, the device has no
surface, and nothing on the window's thread waits for warmth. What the two threads share is the
process: **Qt's start and the device's bring-up both load large shared objects, and the loader
serialises them on one lock**. Qt pays about 8 ms of waiting and 3 ms of slower running before its
window has a size.

**The thirty milliseconds did not reproduce.** In this hold Qt's window is 4 to 9 ms later on the
device lane by the gate's minima (ADR 2125) and 9 ms by the medians above; ADR 2071's 114 against
84 ms was a single hold's medians at another moment of this machine, and the device came up in 53
to 64 ms then and 40 to 43 now. What is stated is what was measured today.

**After the window, the difference is the page** (section 3): the first frame waits for the two
pages the viewport shows, which cost the device 10.2 and 4.4 ms and the processor 2.2 each.

**GTK is not Qt here.** GTK's own start loads Mesa's EGL, which links the same `libLLVM`, so on the
device lane GTK's thread waits on the lock (blocked 25.7 ms against 7.7, medians) for a relocation
the device's thread does on its behalf, and runs 7 ms less (59.3 against 66.4): its window is not
later by the gate's minima. Its first frame differs by the page alone.

## 3. Why a small page is slower on the device

The read-back lane now says each page's stages on `--trace=frames` (`read_back.rs`'s `phases_of`,
`render_raster::FrameCost`). Medians of seven launches a window, in a second, short `--clock` hold
of its own (the stages were not written to the first hold's files):

| ms | scene | encode | transfer | execute | read back | elsewhere in the device call | outside both | in all |
|---|---|---|---|---|---|---|---|---|
| page 1, a cold device | 0.4 | 3.3–3.6 | 0.4–0.5 | 0.2 | 1.1–1.3 | 2.7 | 1.6–1.8 | 10.1–10.2 |
| page 2, warm | 0.2 | 0.9–1.0 | 0.3–0.4 | 0.0 | 0.5 | 0.5 | 2.0–2.1 | 4.3–4.5 |

**The device's own passes cost 0.2 ms; everything else is a fixed cost per page.** On a warm device
that is about 2 ms of encoding, transfer, read back and the submit-and-wait, and 2 ms outside both
of raster's stages, which on this page is where `QuorraRasterizer::rasterize` puts the read-back
page on its medium — a premultiply, the imposition and a demultiply over every pixel — on the
reading of the code rather than a stage measured. A cold device adds 2.4 ms to the encode and 2.2
ms to the submit-and-wait: its first use. The brief's second hypothesis holds in its shape — fixed
costs a 619 × 876 page cannot amortise — and not in its parts: the read-back copy is 0.5 to 1.3 ms
of it, not most.

## 4. The decision: no lever

Each lever the profile offers was priced and none is taken.

- **Start the bring-up after Qt's window has its size**, so that Qt never waits on the lock. By
  arithmetic on the gate's minima rather than a measurement: the window would come at about 68 ms,
  the device would then take at least GTK's 28 ms to come up, and the two pages 13 ms more, so the
  first frame would land near 110 ms against the 90 to 94 it lands at now. The order principle 2
  asked for, the bring-up beside the start, is the faster one: the wait it costs the window is
  smaller than the wait it would cost the page.
- **Load the driver on the window's thread before Qt starts.** The same relocation on the critical
  path, paid serially rather than waited for: nothing gained.
- **Draw page one on the processor where it is faster.** Principle 2 gives the device every frame
  predicted inside `WARN`, and ADR 1831's rule is the owner's; a 4 ms page is the device's.
- **Put the medium on the device**, which would take the 2 ms outside raster's stages off every
  page. It is `render-raster`'s and raster's construction, outside this round's files, and is left
  as that: a candidate with its figure.

The cost stays written down, as ADR 2071 required, and is now a band (ADR 2125): a regression in
the driver or the lock shows in `device_up_ms` and `device_window_ms`, one in the page in
`device_page_one_ms`.
