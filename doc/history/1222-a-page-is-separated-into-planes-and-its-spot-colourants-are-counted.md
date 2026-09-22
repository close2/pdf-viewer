# 1222 — A page is separated into planes, and its spot colourants are counted first

Date: 2026-09-22. Branch: `batch-1219-1224`, worktree shared with five sibling rounds.
ADR: [1281](../adr/1281-a-page-is-separated-into-planes-and-its-spot-colourants-are-counted-first.md).

## §10.8.3 — stage one of the spot plane

`pdf_model::colourants::spot_colourants` is step a)'s "[t]he PDF processor determines what process
colours and possible spot colours the simulated device is to have": every `Separation` and
`DeviceN` colourant the page's resources reach — patterns, shadings, forms, images, Type 3 fonts,
annotation appearances — less `All`, `None`, the four reserved process names and an `NChannel`
space's Table 71 components, never through a soft mask's group (§11.7.3). Depth-first in document
order, once per object and role, from a heap stack. `plane_count` is `2 + ceil(S / 3)`. Twelve
tests, five of them the examples §8.6.6.4 and §8.6.6.5 print.

`Half` is `Plane`, with `Plane::PROCESS` and `Plane::COLOURANTS`, and `content::in_planes`
interprets the page as a sequence of planes. No spot plane is made, so nothing draws differently.

A report naming the colourants without a plane was built under the preference and taken out: the
condition "names a spot colourant" is far wider than "draws differently for want of a plane"
(trap 11), and ADR 1229's own fixtures would have read as incomplete. The precise condition is per
mark, and stage two has it.

## The measurement

callgrind, two binaries from `HEAD` and `HEAD` plus this round's hunks, in one sitting. Page 101 ×
50 (device path, no new code runs): 1 250 037 757 / 1 250 038 551 → 1 250 082 480. A 3000-mark
`/DeviceCMYK` page: 86 772 513 → 87 132 711, +0.41%. Every function of this tree executes the same
instructions in both; the whole delta is glibc `memmove` under `Document::get_key`, same 254 110
calls, 22.05 → 23.46 instructions each — alignment moving with frame layout, which the untouched
device page shows too. An iterator carrying each run's tuple was suspected first, replaced by
a loop, and measured again: no change, so the suspicion was wrong and the claim was not written.

## Gates

`raster_golden` held 974, moved 0. `render-raster --test corpus` and `pdf-model --test corpus`
passed. Tier 1 on `pdf-colour`, `pdf-model`, `render-cpu`: clippy clean, 1847 tests pass;
`cargo test -p conformance` passes, the frontier map included. Stages two to four and the census
sizing `S` are priced in `doc/todo/23`.
