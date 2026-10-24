# 2298 — The launch enumerates the adapters once, and the constructor chooses among the check's list

Status: implemented. Session 1732, 2026-10-23. Reopens ADR 2283's deferred lever and builds it.

## Context

A launch enumerated the machine's physical devices twice: `raster_gpu::create_launch_instance_timed`'s
adapter check, to learn whether a primary backend has an adapter on hardware (ADR 1532), and then
`request_adapter` inside the constructor. Under a display each enumeration is a question Mesa's
`VK_LAYER_MESA_device_select` puts to the display server, and ADR 2282 measured the second asking at
5.5 to 13.0 ms of a bring-up on the owner's Wayland session against 0.2 to 0.35 ms where no display
is named. ADR 2283 priced handing the check's adapter to the constructor at 5 to 9 ms of `quorra`'s
first present, and deferred it because no gate could see it. Batch one hundred and twelve's brief
owns both crates and asked for it built and measured.

## Decision

1. **`raster_gpu::LaunchInstance` is the instance and the adapters its check enumerated.**
   `create_launch_instance_timed` keeps the list where it returns the primary instance and carries
   none where its fallback replaced the instance. `From<wgpu::Instance>` makes one that carries
   none, for an instance made any other way.
2. **Two constructors take it**, `Device::headless_on_launch` and `Device::for_surface_on_launch`;
   the four that existed are unchanged and pass an empty list. With a list, the
   `Options::adapter` filter reads it in place of `enumerate_adapters`, and without a filter
   `preferred` chooses the way `wgpu-core`'s `Instance::request_adapter` does under
   `HighPerformance`: the adapters that can present to the surface (`is_surface_supported`, the same
   `get_capabilities` call), stably ordered discrete, integrated, other, virtual, processor, the
   first taken. Where none can present, the constructor asks `request_adapter` as before, so the
   refusal is `wgpu`'s own. In `wgpu` 30.0.0 every backend's `enumerate_adapters` ignores the
   surface hint, and `enumerate_adapters` applies the same limit, feature and flag filters as
   `request_adapter`, so the two lists are the same list.
3. **`render-raster` hands the launch instance through**: `QuorraWindowRenderer::instance`,
   `instance_timed` and `instance_with` return it, `with_instance` and `new_headless` build on it,
   and `QuorraRasterizer::with_options` — the launch gate's child, the toolkit windows and the corpus
   arms — does too. `quorra` and `quorra-confined` carry it on their instance thread.
4. **The restated order is held by a test**, `the_carried_choice_is_request_adapters_choice`, which
   asks both on one instance of the machine the tests run on; planted with the processor ranked
   first, it failed naming lavapipe against RADV.

## What was measured

Headless, HEAD's export against the tree's, in one `--clock` hold at one-minute loads of 4.9 to 8.3:
twelve gate bring-up children each, interleaved, no display variables. The adapter step read 0.228 to
0.344 ms on HEAD and 0.001 ms on the tree, the adapter the same RADV in all 24; the bring-up figure
itself, 26.6 to 33.3 against 24.9 to 35.3, is instance creation's spread (15.1 to 21.6 against 12.6
to 17.6), which the change does not touch. Under callgrind a bring-up child executes 682.30 and
682.37 M instructions on HEAD and 680.66 and 680.65 M on the tree. `t2-launch_path` and
`t2-toolkit_launch` ran once per arm in the same hold (ADR 2299 reads them).

On the owner's adapter, the Radeon 890M through RADV on the owner's Wayland session, in one loop run inside
one `--clock` hold, staged binaries built by the agent user, every process ending itself under a
timeout, no capture and no key sent, at one-minute loads of 3.1 to 12.4 falling through the run:

| interleaved, HEAD against the tree | load | HEAD, ms | tree, ms |
|---|---|---|---|
| the gate's bring-up child, five each: adapter step | 9.3 to 12.4 | 6.29 to 14.05 | 0.001 |
| the same: bring-up | 9.3 to 12.4 | 45.5 to 61.4, median 52.4 | 34.4 to 52.2, median 37.5 |
| `quorra`, four each: adapter step of `device up in` | 4.1 to 8.6 | 7.7 to 14.7 | 0.9 to 2.4 |
| the same: `device up in` | 4.1 to 8.6 | 18.7 to 23.1 | 11.4 to 13.8 |
| the same: first present | 4.1 to 8.6 | 74.9 to 90.3 | 63.1 to 78.0 |
| `quorra-qt`, two each: adapter step of `graphics device up in` | 3.3 to 3.7 | 5.8 and 8.1 | 0.0 and 0.0 |

The adapter, RADV, was the same in every process of both arms. `quorra`'s remaining 0.9 to 2.4 ms is
the surface's capability query, which reads 1.1 to 2.2 ms with the layer off too. Its device step is
6.2 to 11.1 ms shorter pair by pair, about the hypothesis's 5 to 9; the first present moved by more,
10.9 to 22.8 ms pair by pair, but the load fell through each triple, so the figure this ADR claims is
the device step's. `quorra-qt`'s device is up before its window's first size (ADR 2282), so its first
frame is not claimed either way.

## Costs

- **The preference is restated, not called.** A `wgpu` upgrade that changes `request_adapter`'s order
  would leave `preferred` on the old one; the test holds the two together only on the adapters of the
  machine it runs on, which here are one integrated GPU and one processor.
- **What ADR 2283 objected to is answered by a printed step, not a band.** No gate judges the
  display's share; the gate prints `device_adapter_ms` beside every bring-up, and a second
  enumeration's return reads there as 0.2 to 0.35 ms instead of 0.001 headless, and on a window's
  `device up in` line as its 5 to 13 ms.
- A `LaunchInstance` holds the adapters it did not choose until it is dropped; they are handles.
