# ADR 0185 — The graphics instance is made before there is a window

Status: accepted, 2026-08-04 (session 288).

## Context

ADR 0182 put the document on a thread of its own and left the launch's largest step untouched:
bringing the graphics device up, 33 to 45 ms of a 145 ms launch. ADR 0179 had already ruled out
the obvious lever — restricting the backend set moves the cost between two steps and does not
reduce it — and named the real one: **a `wgpu::Instance` needs no window, no surface and no event
loop**, so it can be made before any of them exist. `bring_up overlap` measured that at ~20 ms.

It was not this tree's to pull. `quorra_gpu::Device::for_surface` created the instance itself, and
the one number it reported — `adapter_enumeration` — was measured from before `Instance::new`, so
a host could not even see the shares. Both were written up in `doc/QUORRA_FEEDBACK.md` §8, with
the measurement, and a third thing was written up as explicitly *not* asked for.

**quorra answered at `7d5dafb`** (its ADR 0014): five startup fields where there were three,
`create_instance` beside `headless_with_instance` and `for_surface_with_instance`, and no backend
knob — with our own §8.3 measurement quoted as the reason that silence is deliberate.

## Decision

**`main` spawns two threads at its first line, and `resumed` joins them on either side of the
device.**

```text
main            spawn(document)   spawn(instance)   EventLoop::new   run_app
resumed         join(instance) → build the presenter → join(document) → receive → Resize
```

The order is the whole design and it is not symmetric:

- **The device needs the instance**, so that join comes first. It costs 0.006 to 2.6 ms, because
  `EventLoop::new` on this machine takes 20 to 45 ms and the instance is finished inside it.
- **The device does not need the document**, so that join comes last — after the presenter exists,
  which is ADR 0182's argument unchanged.

**Two threads rather than one**, and the first attempt used one: the document and the instance on
the same thread, joined together before the presenter. It works and it costs 15 to 26 ms at the
join, because the instance's consumer had to wait for the document's work as well. Splitting them
puts each join next to the thing that needs it. This machine has 24 cores; contention was the
argument for one thread and it was not an argument at all.

**The instance comes from `QuorraPresenter::instance`, not from `wgpu::Instance::new`.** The
descriptor has to be the one quorra's own constructors use, and a host that guessed it would find
out at `create_surface`. So the re-export is the interface: `render-quorra` names it, the host
calls it, and nothing in `viewer-ui` knows what an `InstanceDescriptor` is.

## What it is worth

`pdf-viewer --trace` on ISO 32000-2 — 1023 pages, 101 318 objects — under `Xvfb` with `lavapipe`,
three runs each:

| step | before | after |
|---|---|---|
| graphics instance | inside bring-up | **+0.006 to +2.6 ms** |
| graphics device | +33.4 to +45.1 ms | **+13.2 to +19.2 ms** |
| document joined | +3.0 to +5.6 ms | +5.2 to +5.6 ms |
| **process start → first frame** | 145 / 152 / 145 ms | **110 / 112 / 119** |

And the trace line a person reads is legible for the first time:

```text
trace: device up in 13.7ms — instance None, surface 23.8µs, adapter 5.34ms, device 8.00ms, pipelines 5.36ms
```

`instance None` is the point: the step happened on another thread, and quorra reports `None`
rather than zero for work it did not do.

**Our numbers and quorra's disagree about the share, and both are right.** Its ADR measures
adapter selection at 3.2–4.4 ms headless; ours is 5.3–6.8 ms with a `compatible_surface` under a
virtual X server. That is the field split earning itself: one number could not have shown it.

## The lesson

**A lever that is not yours is still worth measuring.** Nothing in `doc/QUORRA_FEEDBACK.md` §8
could have been written without `bring_up`, and the measurement is what made the request specific
enough to answer — including the part that said *do not add a backend knob*, which is the only
paragraph in that section that asked for nothing and is the one the library's ADR quotes back.
The corollary is the ordinary one for this project: **the request was a number, not a preference.**
