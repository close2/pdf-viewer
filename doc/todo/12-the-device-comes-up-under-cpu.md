# `--cpu` brings a graphics device up anyway

Status: **reported by the project owner from a Windows machine in the three-hundred-and-eighty-third
session**, and diagnosed here by reading the code. The crash itself is an Intel Vulkan driver's; the
part that is ours is that `--cpu` did not avoid it.
Priority: 12 — a defect: the program does not start at all, and the flag a person would reach for
does not help
Corpus: —, this is a property of the host
Clauses: —, this is `CLAUDE.md` principle 2's other half: what the launch path *does*
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs` (`main`, `App::resumed`, `App::draw`)
Beside it: `doc/QUORRA_FEEDBACK.md` §12, which is the half that is quorra's and is **not** a todo

## What was reported

> I have tested the pdf-viewer on windows. On the windows machine the viewer crashes in the vulkan
> (intel) driver. Interestingly `--cpu` still let the app crash.

The owner's question — *do we still try to initialize the GPU even with `--cpu`?* — is answered by
the code, and the answer is yes, twice over.

## What `--cpu` actually does today

`Arguments::processor` reaches exactly one place: `App::draw`, where it turns the graphics device's
attempt into `Err("was not asked, because --cpu")` so that the frame falls to `CpuRasterizer`. It
changes **which rasteriser draws the page** and nothing else. In particular:

- `main` spawns `std::thread::spawn(QuorraPresenter::instance)` **unconditionally**, before the
  window exists. A `wgpu::Instance` is the driver loader — this is where the Vulkan ICD is opened
  and where quorra measured roughly 80% of bring-up (ADR 0185). Under `--cpu` that thread runs
  anyway and its result is joined anyway.
- `App::resumed` builds a `QuorraPresenter` **unconditionally**: surface, adapter selection,
  device. Under `--cpu` this is a device nothing will draw with.
- And it is not dead weight that could simply be skipped: **the CPU raster is presented through the
  quorra surface** — `present(PresentFrame { raster: Some(..), .. })` — because a working window is
  the only path pixels take to the screen. `App::draw` returns `None` when `self.state` is absent,
  so with no presenter there is no drawing at all.

So a driver that faults during instance creation, adapter enumeration or device creation takes the
process down whether or not `--cpu` was given, and the flag a person reaches for after a driver
crash is the one flag that cannot help them.

**And the failure is a panic rather than a report**: `.expect("presenter creation")` in `resumed`.
Where the driver returns an error instead of faulting, this program aborts with a Rust panic
message rather than saying which stage failed and what could be done about it — which is
`CLAUDE.md` principle 1's rule about `unwrap` in the one place where a person most needs a sentence.

## What this owes, and the order matters

### 1. `--cpu` must mean *no graphics device*, which needs a second way to put pixels on the screen

This is the real work, and it is a decision rather than a patch. Presenting a CPU raster without a
device means a software present path — `softbuffer` is the usual answer beside `winit`, and it is a
dependency decision in ADR 0186's and ADR 0214's shape (what it costs, what it is confined to, what
the licence is). The alternatives are worse and should be written down as rejected rather than left
unconsidered:

- **Keep presenting through quorra under `--cpu`.** Then `--cpu` cannot be the answer to a broken
  driver, and the flag's own documentation ("if a page appears under `--cpu` and not without it,
  the difference is the device") is only true of *drawing*, never of bring-up. Honest, but it
  leaves the reported failure with no way out at all.
- **Bring the device up lazily and let `--cpu` never touch it.** This is half the fix and worth
  having on its own: nothing under `--cpu` needs an instance, so the thread should not be spawned
  and `resumed` should not build a presenter. It stops short only because the window still has
  nothing to present *with*.

**Do the second first**: it is small, it removes the launch cost from every `--cpu` run, and it
makes the remaining question precisely "how does a raster reach a window with no device", which is
one question rather than two.

### 2. The panic becomes a sentence

A device that will not come up is a fact about the machine, not a bug in this program, and it
should print which stage failed, what the adapter list looked like, and what to try — the shape
`Confinement::shortfall` already uses for a confinement that could not be applied.

### 3. `--backend`, when quorra can honour it

The flag is trivial *given the parameter*, and the parameter does not exist: `quorra_gpu::create_instance()`
takes none, `Options` has no backend field, and the descriptor is built without `.with_env()` so
`WGPU_BACKEND` is not read either. **That ask is `doc/QUORRA_FEEDBACK.md` §12 and is deliberately
not a todo here** — a flag this program cannot honour would be worse than no flag. When it lands,
`--backend vulkan|dx12|gl|metal` is one `match` and one argument, and the Windows default becomes a
choice this project makes rather than one wgpu's hub order makes for it.

## What cannot be measured here

**No machine in this environment runs Windows, has an Intel adapter, or has DX12.** The cross-target
check builds `x86_64-pc-windows-msvc` and that is all it proves. Everything above is read out of the
source; the crash is the owner's observation. A round taking this can verify item 1 on Linux — that
`--cpu` opens no instance and creates no device is testable here, with `--trace` and with `strace`
on the ICD — and cannot verify that it fixes the Windows crash. Say which is which.
