# ADR 0607 — The device a confinement cannot hold, and the boundary that was already half chosen

Status: accepted, 2026-08-25. Session 724. Settles `doc/todo/34` §2, which `doc/todo/15`'s road B
has been waiting on since the seven-hundred-and-nineteenth session priced the tier change and left
the question unargued. Cites no clause: this is `CLAUDE.md` principle 3 against principle 2, and
the ledger is untouched.

## The question, and why it is not a transport question

`viewer-ui` is a tier-2 host: it hands back `Rendered::Presented` and draws on the graphics device.
A confined process draws on the processor and hands back a raster. Putting the window on the
confined boundary is a change of *tier*, and `doc/todo/34` §2 named two ways out and argued
neither:

- the host keeps the device and the confined process ships **display lists** rather than pixels;
- or the confined process is given a window handle and **drives the device itself**.

The second reads as the cheaper one — nothing crosses per frame at all — and it is the one this
round expected to have to argue *against* on performance grounds. It does not get that far.

## Finding 1 — the second option does not exist under this confinement, at any ordering

`crates/render-quorra/examples/device_under_confinement.rs` brings up a real headless device on
this machine's Radeon 890M, draws page 7 of ISO 32000-2, and asks what
`pdf_sandbox::lockdown::apply_for(Profile::Interpreter)` — the confinement `pdf-view-worker`
already runs under — makes of it. Each stage is a child process, because the seccomp action is
`KillProcess` and the parent has to survive to report:

| stage | outcome |
|---|---|
| `warm` — bring up, draw, draw again, no confinement | drew |
| `descriptors` — bring up, draw, confine, report what was reached | confined, **landlock unavailable** |
| `confine-then-draw` — bring up, confine, then draw | **killed by signal 31 (SIGSYS)** |
| `draw-then-confine` — bring up, draw once so every pipeline exists, confine, draw the same frame again | **killed by signal 31 (SIGSYS)** |

The third row is the option's own shape and the fourth is its *best* case — a device so warm that
the frame about to be drawn has already been drawn once. Both die. `strace` names the call: the
first system call after `seccomp(SECCOMP_SET_MODE_FILTER, …TSYNC)` returns is

```text
ioctl(5, DRM_IOCTL_AMDGPU_GEM_CREATE, …)
```

— the driver allocating a buffer object on the DRM render node. **A graphics device is not a
resource a process holds; it is a conversation with a kernel driver**, and this confinement ends
every conversation a process can have. There is no ordering that fixes it, because there is no
point after which the driver stops needing `ioctl`.

**And the second row is a cost the first three obscure.** A process holding a device holds **9
descriptors against the confinement's ceiling of 8**, so `landlock_create_ruleset` fails with
`EMFILE` and Landlock is `Unavailable`. The depth layer is lost *before* the filter is reached, on
a limit whose own comment says it "leaves room for the runtime to do something ordinary without
leaving room to do something interesting".

## Finding 2 — what admitting a device would cost, counted rather than asserted

`doc/todo/34` §2 said only "a large surface, and drivers open files". `strace -f -c` over a device
bring-up (`examples/bring_up vulkan`) says how large. The bring-up issues **55 distinct system
calls, 35 of them off the interpreter's 28-call allow-list**:

| off the list | calls |
|---|---|
| `openat` | 314 (122 failing) |
| `fstat` | 234 |
| `ioctl` | 174 |
| `readlink` | 153 |
| `access` | 140 |
| `lseek`, `getdents64`, `sched_setaffinity` | 73, 52, 52 |
| `socket`, `connect`, `recvfrom`, `shutdown`, `getpeername` | 2, 2, 5, 2, 2 |
| `memfd_create`, `ftruncate`, `mkdir`, `unlink`, `flock`, `prctl`, `poll`, `sysinfo`, … | the remainder |

What it opens is the argument, not the count:

- **`/dev/dri/renderD128`**, and 174 `ioctl`s on it — **563 for a bring-up and two frames**, so
  roughly 190 per frame, across **25 distinct DRM request numbers** including `AMDGPU_CS`, which
  is command submission.
- **`~/.cache/mesa_shader_cache` and `~/.cache/radv_builtin_shaders`, read *and written***, with a
  `mkdir` and an `unlink` among them.
- The vendor driver itself: 37 opens under `/usr/lib`, `dlopen`ed on the strength of **56 JSON
  manifests parsed** out of `/usr/share/vulkan/{icd.d,implicit_layer.d,explicit_layer.d}`.
- An `AF_UNIX` socket **connected to `/tmp/.X11-unix/X0`** — headless, on a run that asked for no
  window at all.

`lockdown_linux.rs`'s own comment on the allow-list reads: "Notably absent: `openat`, `socket`,
`connect`, `execve`, `clone`, `ptrace`, `prctl`, `ioctl`. There is no path from decoding an image
to any of them." Option B's requirement is exactly that sentence deleted. And `ioctl` cannot be
narrowed the way a path can: seccomp-BPF filters scalar arguments and cannot dereference the
pointer a DRM ioctl carries, so permitting the 25 request numbers permits **whatever they can be
made to say**, to a C driver in the kernel, from the one process in this program that holds
untrusted bytes. That is the inversion principle 3 exists to prevent.

**B's only viable form is A with an extra hop.** A separate, differently confined graphics process
would work — it is what a browser does — but the untrusted document would still be interpreted in
the confined process and the marks would still have to cross to the process holding the device. It
is option A with the host replaced by a third program, and it inherits every question below.

## Finding 3 — what a display list costs beside its raster, on 958 real first pages

This is the number that decides A, and it had never been taken.
`crates/viewer-confined/examples/list_against_raster.rs` walks a display list and sums what an
encoder **must** write — a tag per command, four bytes per index and per `f32`, and the payload of
everything a receiver cannot reconstruct — against `TargetSpec::for_page`'s raster at the same
scale. Both figures are byte counts, so the run is load-immune; the corpus is `doc/pdf.js`, first
page of each document, with `pdf-sandbox-worker` built beside it (trap 10).

The aggregate is dominated by one 212-megapixel page, so the **distribution** is the answer:

| scale | median list/raster | p90 | p99 | worst | pages whose list exceeds its raster |
|---|---|---|---|---|---|
| 1.0 (72 dpi) | 0.034 | 1.000 | 17.3 | 1101× | 153 of 958 |
| **1.333** (a page fitted at 96 dpi) | **0.019** | 0.562 | 9.75 | 659× | **41 of 957** |
| 2.0 | 0.008 | 0.250 | 4.33 | 289× | 23 of 957 |
| 4.0 | 0.002 | 0.063 | 1.09 | 73× | 12 of 957 |

**A display list is scale-invariant and a raster is quadratic in the scale**, which is why every
column improves to the right and why the table's leftmost column — 72 dpi, smaller than any window
asks for — is the display list's worst case rather than its typical one.

The tail is real and it is one population: **a scanned page's decoded samples *are* its display
list.** `scan-bad.pdf` is one `Command::Image` and 33 660 053 bytes; `issue12841_reduced.pdf` is
two marks and 80 087 263. Those pages ship 9× to 659× more bytes as a list than as pixels.

The two documents the seven-hundred-and-nineteenth session priced, at 1.333:

| | list | raster | ratio |
|---|---|---|---|
| ISO 32000-2, page 1 | 1 014 292 B | 3 566 648 B | **0.284** |
| `PDF20_AN001-BPC.pdf`, page 1 | 36 265 B | 3 566 648 B | **0.010** |

Against this transport's measured rate — `examples/confined_page` on `PDF20_AN001-BPC.pdf`, load
average 12, a 849×1200 raster of 4 075 200 B crossing in **3.915 ms**, so **1.04 GB/s** end to end
— those are about **0.035 ms and 0.97 ms** against **3.9 ms**. 719's "4.0 ms raster back" is the
figure being removed.

**And the codec has a hard requirement with a number on it.** Counted flat, with each occurrence
of a shared `Arc<Path>`, `Arc<[u8]>` or `Arc<Shading>` written out again, the same corpus is
**1.90 GB against a 2.19 GB raster — 0.91**. An encoder that does not preserve the sharing buys
nothing at all. Per page the flat/shared ratio is 1.16 at the median, **3.01 at p90 and 30.3 at
worst**, and `pdf_render::Command`'s own documentation says where it comes from: "3005 fill
commands on a dense specification page carried 101 320 path segments between them".

## Finding 4 — what neither option removes, which is most of the cost

The **document** still crosses. On ISO 32000-2 that is 19 206 210 bytes and, in this round's own
run at load average 15, **74.2 ms** of the 167.5 ms the confined path takes to reach page one —
719 measured 41–66 ms on a quieter machine. Neither display lists nor a device in the child touches
it; `doc/todo/34` §5 owns it, and a round choosing between A and B on the strength of the *frame*
is optimising the smaller half.

What A removes that B also removes: the raster back, per frame. What A removes that this round did
not expect to be the largest item: **the round trip on a zoom or a scroll disappears entirely.**
The host holds the `Arc<DisplayList>` and re-rasterises without asking the child anything — the
property `zooming_rasterises_again_without_interpreting_again` already asserts in process. Under
today's confined path a smooth zoom is a 4 MB raster across the pipe *per frame*: at 60 fps, 245
MB/s of a 1.04 GB/s transport and 3.9 ms of added latency on every frame. Under A it is nothing.

## The decision

**Display lists cross, and the raster payload stays.** The confined process chooses per page, by
comparing two sizes it can both compute before it sends either: the encoded list, and the target's
pixels. On the corpus at a window's scale that sends a list for **96%** of first pages and pixels
for the other 4%, and it sends pixels for exactly the pages where pixels are smaller.

This is not a hedge and it does not resurrect the two-protocol design `doc/ui-boundary.md` was glad
to be rid of. `Rendered::{Raster, Presented}` is already a payload choice on this boundary; this
adds a third arm to a choice that exists, made by measurement rather than by tier.

### What it costs, written down

- **A `DisplayList` codec on both sides, and a fuzz target beside `confined_wire`.** The host gains
  a parser of input the confined process produced. That is a real new surface and it is the right
  one: Rust under `#![forbid(unsafe_code)]`, in a crate that already speaks a fuzzed wire protocol
  — against a kernel graphics driver's ioctl table written in C, which is what option B offered.
- **The codec must preserve `Arc` identity**, and the number above is what happens if it does not.
- **Two deferred producers cannot cross as they stand**, and this is the one structural blocker.
  `ImageSource::AtDeviceScale` and `ShadingKind::Sampled` carry `Arc<dyn ImageAtDeviceScale>` and
  `Arc<dyn ColoursAtDeviceScale>` — *producers*, invoked by the backend once it knows how many
  device pixels the mark covers (ADR 0210). They are **self-contained data behind a trait object**
  rather than closures over the document — `pdf_model::shading::FunctionColours` holds its
  functions, its colour space, its conversion, its domain and §10.5's transfer; `MaskedAtDeviceScale`
  holds an image and a soft mask — so they *can* be encoded, at the price of putting §7.10's four
  function types and `pdf-model`'s colour conversion on the wire. **4 of 958 first pages carry
  one**, and the raster arm already covers them exactly. So this is deferred with a reason rather
  than by attrition, and the reason is that the fall-back is not a workaround: for a page whose
  colours are a function of position at the device's own resolution, pixels are what the boundary
  is *for*.
- **The host must rasterise a list it did not interpret.** It already does: `render-quorra` is that
  translator and `viewer-ui` is already on it.

### What it costs the launch path — nothing this path does not already pay

`CLAUDE.md` §2 puts page one on the graphics device and forbids waiting for warmth. Under A the
host's device is created on the launch path exactly as it is today, warm before the first list
arrives; the child's spawn and confinement is **1.415 ms** (measured this round on
`PDF20_AN001-BPC.pdf`) and overlaps it. Under B the device moves *behind* a process spawn into a
process that then cannot be confined — bring-up measured at **26.7 to 34.4 ms** over five
processes at load average 7–10, which is time the launch path would be paying somewhere it can no
longer overlap the host's own work.

**So `CLAUDE.md` §2 does not choose between the two options. It forbids one of them**, and finding
1 forbids it a second time for a different reason. That the security argument and the startup
argument agree here is worth saying out loud, because they usually do not.

## What would change this decision

- A page population where the list is routinely larger than the raster. The instrument is
  `examples/list_against_raster`; the corpus's answer today is 4% at a window's scale.
- A confinement that can hold a device without admitting `ioctl` — a kernel interface that does not
  exist today and that `doc/todo/35`'s non-Linux platforms would each answer differently.
- The document crossing (finding 4) being removed, which would make the frame the dominant cost and
  raise the value of B's zero-per-frame conversation. It would not make B safe.
