# ADR 0221 — A flag that opens no driver, and a backend a person can name

Status: accepted, 2026-08-07 (session 384).

## Context

The project owner ran `pdf-viewer` on a Windows machine with Intel graphics and **it crashed
inside the Vulkan driver**, and `--cpu` crashed too. Two documents came out of that report in the
three-hundred-and-eighty-third session, one per owner: `doc/todo/12` for the half that is ours,
and `doc/QUORRA_FEEDBACK.md` §12 for the half that is the renderer's.

**The crash itself is nobody's code in either tree.** What made it a report is that the machine
has two driver stacks and this program could reach only one of them, and that the flag a person
reaches for after a driver crash was the one flag that could not help them.

`--cpu` reached exactly one place — `App::draw`, where it turned the graphics device's attempt
into a refusal so the frame fell to `CpuRasterizer`. It chose **which rasteriser drew the page**
and nothing else. `main` spawned `QuorraPresenter::instance` unconditionally, which is where the
Vulkan loader is opened; `App::resumed` built a presenter unconditionally, which is surface,
adapter and device. And it could not simply be skipped, because the processor's raster was
presented *through the quorra surface* as one image: a working device was the only path pixels
took to the screen.

So this round has three parts, and only the middle one is a matter of judgement.

## Decision

### 1. `--cpu` creates no graphics instance, no adapter and no device

`spawn_instancing(processor, backend)` answers `None` under the flag, and `App::bring_up` takes
the software path before it would have joined a thread that was never spawned. A function rather
than three lines in `main` because the promise needs a test:
`cpu_creates_no_graphics_instance`, with `without_cpu_an_instance_is_created` as the control that
stops it passing on a function that never spawns.

**Demonstrated rather than argued**, on Linux, which is what `doc/todo/12` said this environment
could do. `strace -f -e trace=openat` over a `--cpu` run against the *previous* binary and this
one:

| | shared objects opened | Vulkan among them |
|---|---|---|
| before | 56 | `libvulkan.so.1`, `libvulkan_lvp.so`, `libvulkan_radeon.so`, and both ICD manifests under `/usr/share/vulkan/icd.d/` |
| after | 17 | **none** — and no `icd.d`, no `libEGL`, no `libdrm` |

### 2. A software present path, and it is a dependency decision

Item 1 is only half a fix on its own: a window with no device had nothing to present *with*, so
`--cpu` would have shown a blank window, which is worse than what it did before. `softbuffer`
is the crate that puts a raster on a window without a device — `rust-windowing`'s, beside
`winit`, sharing its `raw-window-handle` 0.6 — and `viewer_ui::software` is the twenty lines of
compositing around it.

**What it costs**, in ADR 0186's and ADR 0214's shape:

- **Four packages**: `softbuffer`, `tiny-xlib`, `ctor`, `dtor`. All `MIT OR Apache-2.0` except
  `tiny-xlib`, which is `MIT OR Apache-2.0 OR Zlib`; every one of those is already in
  `deny.toml`'s allow list, and `cargo deny` is clean on all four checks. Nothing is
  redistributed as data, so `NOTICE` does not change.
- **`kms` is off**, and `drm` and `gbm` with it: a document viewer draws into a window, not onto
  a bare console framebuffer. The rest of the feature set is winit's own — X11 and Wayland, both
  dlopened.
- **One cost that is not the crate's own and is the honest argument against it.** softbuffer's
  X11 backend reaches Xlib through `tiny-xlib`, which loads `libX11` in a `ctor` — a static
  initialiser, run by the loader **before `main`**, on every launch, whether or not a software
  surface is ever made. That is exactly what `CLAUDE.md` principle 2 forbids ("nothing eager"),
  so it is measured rather than waved at: `pdf-viewer --licences`, which never opens a window,
  40 runs a batch, three batches, built both ways.

  | | per launch |
  |---|---|
  | with `x11` (this build) | **1.34, 1.36, 1.39 ms** |
  | with `x11` off, Wayland only | **0.98, 0.91, 0.92 ms** |

  **About 0.4 ms**, on every launch, of a launch that is 105 ms. `strace` confirms the cause
  rather than assuming it: the Wayland-only build opens `libX11` zero times under `--licences`
  and this one opens it twice.

  **Taken, and the alternative is worse.** Turning `x11` off makes `--cpu` useless on every X11
  machine — including this one, where the demonstration above would not exist — and there is no
  third option: the ctor is how `tiny-xlib` reads Xlib's default error hook, which it must do
  before anything else installs one. 0.4% of the launch to make a flag honest is a trade this
  file records rather than hides.

**Rejected, and written down rather than left unconsidered** (both are `doc/todo/12`'s):

- *Keep presenting through quorra under `--cpu`.* Then the flag cannot be the answer to a broken
  driver at all, and its own documentation is true only of drawing. This is what the tree did,
  and it is what the report was about.
- *Bring the device up lazily and let `--cpu` never touch it, and stop there.* Half the fix, and
  the half that removes the cost; it stops short because the window still has nothing to present
  with. The todo file said to do it first because it makes the remaining question exactly one
  question. It did, and this round answered it in the same commit rather than shipping the blank
  window in between.

### 3. `--backend vulkan|dx12|metal|gl`

quorra answered §12 at `2531f447` with **exactly the parameter that was asked for** —
`create_instance_with(backends)`, `create_instance()` unchanged and now that function with
`Backends::all()` — plus `Device::adapter_names_on(&instance)`, which was not asked for and
closes a trap the parameter would have opened: a host that restricted its backends and then
listed adapters with the all-backends enumerator would offer a choice its own constructors could
not honour. They also decided the question §12 left to them and did not read `WGPU_BACKEND`
(their ADR 0017), on the grounds that a library rendering through a different driver because a
variable was exported has a failure mode that reproduces nowhere.

So the flag can be honest now, and `doc/todo/12` said it would not be added until it could.

- **Four values**, not six. `BROWSER_WEBGPU` needs a `wasm32` target this program has none of,
  and `NOOP` is compiled only under a wgpu feature this build does not enable — naming either
  would offer a choice that cannot be honoured, which is the shape `adapter_names_on` exists to
  prevent one level down.
- **A word that is not one of them is refused at parse**, with the list, exit 2. A flag carried
  as a string and ignored later is worse than no flag.
- **A named backend with no adapter is a refusal, not a fallback**: the stage that failed, what
  *this instance* could see, what the machine has by every route, and what to try. Exit 1.
  Starting on the stack the person was avoiding is precisely what they used the flag to stop.
- **`--trace` prints two lines about one choice**, because they answer different questions:
  `backend asked for: dx12 (--backend)` is a fact about the command line, and
  `rendering with llvmpipe (LLVM 22.1.8, 256 bits) (Cpu, Vulkan)` ends in the backend that was
  actually *chosen*, which quorra's adapter description has always carried.

Measured here, on the backends this machine has:

```text
$ pdf-viewer --backend dx12 --trace doc/PDF20_AN001-BPC.pdf        # exit 1
the graphics device could not be brought up: surface creation failed: …
  asked for: dx12 (--backend)
  adapters behind it: none — this machine has no adapter for that backend
  adapters on this machine: AMD Radeon 890M Graphics (RADV STRIX1), llvmpipe …, radeonsi …

$ pdf-viewer --backend gl --trace doc/PDF20_AN001-BPC.pdf          # exit 1
the graphics device could not be brought up: no adapter matched None; adapters present: […]
  asked for: gl (--backend)
  adapters behind it: AMD Radeon 890M Graphics (radeonsi, …)
```

The two are the distinction the message was built for: a backend this machine has **no adapter
for**, and a backend whose adapter exists but cannot present to this surface.

### 4. The Windows default is DX12, and this project now owns that choice

`DEFAULT_BACKEND` is `Some(Backend::Dx12)` under `#[cfg(windows)]` and `None` everywhere else.

**The argument is not "DX12 is better".** It is that today's answer is not a choice this project
made at all: with no restriction, `request_adapter` with `PowerPreference::HighPerformance`
breaks ties among adapters of equal device type in **wgpu's own hub order**, where Vulkan
precedes DX12. That ordering is an implementation detail of a dependency; it decided which driver
this program handed a Windows machine to, and it is how the owner's machine reached the one that
crashed. Making it a decision is the change, and having made it, the reasons to make it DX12
rather than Vulkan are:

- **DX12 is the platform's own stack.** Every Windows GPU driver that ships is validated against
  it by the vendor and by Microsoft's certification; Vulkan on Windows is a vendor extra, and its
  quality varies by vendor far more than DX12's does. The report is one data point in exactly
  that direction.
- **It is present wherever this program can run.** DX12 needs Windows 10 and a WDDM 2.0 driver,
  which is every machine that will run a 2026 Rust binary. Vulkan on Windows can be absent
  outright — there is no Vulkan ICD in a clean Windows install without a vendor driver.
- **It costs nothing measurable.** quorra's ADR 0014 §3, from our own §8.3 measurement:
  restricting the instance to one backend halves `Instance::new` and gives every millisecond of
  it back in `request_adapter`. The total is the invariant. This is an escape hatch, not a knob.

**And it is a default rather than a requirement**, which is the part that matters most given
what cannot be tested. A machine with no DX12 adapter gets a note and a second attempt with every
backend, rather than a refusal; a backend a *person* names is refused, because that is an answer
to a question they asked. The rule in one line: **a default gives way, a flag does not.**

### 5. The panic became a sentence

`.expect("presenter creation")` in `resumed` is gone (`doc/todo/12` item 2). A device that will
not come up is a fact about the machine, and it now prints the stage, the adapters behind the
instance, the adapters on the machine, and what to try — `Confinement::shortfall`'s shape — and
then draws the page on the processor. Exercised here by pointing `VK_DRIVER_FILES` at a file that
does not exist: five lines of report, and the page and the sidebar on the window.

### 6. One prefix that was a lie, found on the way

`QuorraRasterError::Device` read `resource upload refused: {0}`. That is true of three of
`quorra_gpu::DeviceError`'s seven variants and false of the four about *construction*, so the
first refusal this round produced said `resource upload refused: surface creation failed` — two
claims, one of them invented. Every variant already names what happened, so the prefix is gone
rather than corrected. Trap 11's shape, in an error message.

## The measurement

`pdf-viewer --trace` on ISO 32000-2, under `Xvfb` with `lavapipe`, three runs each, release.

| | before | after |
|---|---|---|
| `--cpu`: graphics instance | +2.3 to +5.6 ms | **not created** |
| `--cpu`: graphics device | +15.4 to +16.1 ms | **not created** |
| `--cpu`: software surface | — | **+0.159 to +0.194 ms** |
| `--cpu`: process start → first present | 128.2 / 133.1 / 134.7 ms | **67.7 / 58.9 / 57.0 ms** |
| device: process start → first present | 125.9 / 111.0 / 108.3 ms | 108.4 / 103.3 / 104.7 ms |

**Half the launch, and the shape of what is left is the interesting part.** Under `--cpu` the
document's thread is now the thing being waited for — `document joined` +17.0 to +21.6 ms, where
before it was +3.7 to +5.6 — because the launch thread no longer spends twenty of its own
milliseconds on a device. That is the overlap comment in `main` predicting itself: the two costs
are the longer of the pair, and the pair just got shorter on one side.

The device path is unchanged within its own spread; nothing this round does is on it except the
0.4 ms of `ctor` measured in §2, which lands before the timeline starts and is therefore *not*
visible in the table above. Said here so that nobody reads the table as saying it is free.

## What could not be measured, and must not be reported as fixed

**No machine in this project runs Windows, has an Intel adapter, or has DX12.** Everything about
the crash itself stays argued: that DX12 avoids it is a hypothesis about somebody else's driver,
and this round did not test it. What the round is certain of is smaller and worth stating exactly:

- Before it, there was **no way to ask** for the other stack. Now there is one.
- Before it, `--cpu` opened the driver. Now it does not, and that is **demonstrated on Linux**
  with `strace` rather than argued.

The person on that machine finds out by running `pdf-viewer --cpu`, which should now start
whatever the Vulkan driver does, and `pdf-viewer --backend dx12 --trace`, which should print
`backend asked for: dx12 (--backend)` and an adapter description ending `(…, Dx12)`. If DX12 is
the default on their build already, `--backend vulkan` is what reproduces the crash on purpose.

## Revisit when

A platform arrives where the backend cannot be decided before the window exists, which would
break the instance-level premise the flag rests on; or when a Windows machine is reachable from
this project, at which point §4's default stops being reasoned and becomes measured.
