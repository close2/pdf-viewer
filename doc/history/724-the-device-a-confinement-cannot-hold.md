# 724 — The device a confinement cannot hold

`doc/todo/34` §2, settled: **display lists cross, and the raster payload stays**, chosen per page
by size. The alternative — `wgpu` inside the confinement — was not out-priced, it was found not to
exist. ADR 0607. Date 2026-08-25.

## What the round was sent to argue, and what it measured instead

The briefing expected a performance argument against putting a device in the confined process. The
argument never got that far. `render-quorra`'s new `examples/device_under_confinement` brings a
real Radeon 890M device up and then confines the process holding it, each stage in a child because
the seccomp action is `KillProcess`:

| stage | outcome |
|---|---|
| bring up, draw, draw again, unconfined | drew |
| bring up, draw, confine, report | **confined, landlock unavailable** |
| bring up, confine, then draw | **SIGSYS** |
| bring up, draw once, confine, draw the *same* frame | **SIGSYS** |

The killing call is the first after `seccomp(SECCOMP_SET_MODE_FILTER, …TSYNC)`:
`ioctl(5, DRM_IOCTL_AMDGPU_GEM_CREATE, …)`. No ordering helps — a device is a conversation with a
kernel driver rather than a resource a process holds. And the second row is the cost the others
hide: **9 descriptors held against the confinement's ceiling of 8**, so `landlock_create_ruleset`
fails `EMFILE` and the depth layer is lost before the filter is reached.

`doc/todo/34` §2's "a large surface, and drivers open files" now has counts: **55 distinct system
calls in a bring-up, 35 of them off the interpreter's 28-call allow-list** — `openat` 314, `ioctl`
174, `readlink` 153, `access` 140 — `/dev/dri/renderD128` across **25 distinct DRM request
numbers** at about 190 ioctls a frame, the shader cache read *and written*, 56 driver manifests
parsed out of `/usr/share/vulkan`, and an `AF_UNIX` socket connected to `/tmp/.X11-unix/X0` on a
run that asked for no window.

## The number that decided the other option

`viewer-confined`'s new `examples/list_against_raster` sums what an encoder must write for a
display list against `TargetSpec::for_page`'s raster. Byte counts, so the run is load-immune; 958
first pages of `doc/pdf.js`, with `pdf-sandbox-worker` built beside it. A display list is
scale-invariant and a raster is quadratic, so 72 dpi is the list's *worst* case:

| scale | median list/raster | p90 | p99 | worst | lists exceeding their raster |
|---|---|---|---|---|---|
| 1.0 | 0.034 | 1.000 | 17.3 | 1101× | 153 of 958 |
| **1.333** | **0.019** | 0.562 | 9.75 | 659× | **41 of 957** |
| 2.0 | 0.008 | 0.250 | 4.33 | 289× | 23 of 957 |

The tail is one population and it is not exotic: **a scanned page's decoded samples *are* its
display list** — `scan-bad.pdf` is one `Command::Image` and 33.7 MB. Hence a per-page choice
rather than a tier. Two constraints came with it: the encoder **must** preserve `Arc` identity
(flat, the same corpus is 0.91 of its raster instead of 0.37, and 30× worse than shared at the
extreme), and `ImageSource::AtDeviceScale` and `ShadingKind::Sampled` carry trait objects that
cannot cross — 4 of 958 pages, which the raster arm already covers.

## What neither option removes

The document. 19.2 MB of ISO 32000-2 was **74.2 ms** of this round's 167.5 ms to page one, at load
average 15. `doc/todo/34` §5 owns it, and a round choosing between the two options on the strength
of the *frame* is optimising the smaller half.

## The gates

The core, plus the quorra gate — owed because a dev-dependency was added to `render-quorra`, which
is the feature-unification shape trap 16 is about. Both workers built first. Load average 7–15
from three parallel rounds throughout; every figure this round decides on is a byte count.

`cargo test -p conformance` caught a `CLAUDE.md §2` in a new example, which a `§` checker reads as
an ISO 32000-2 clause — the same catch as the six-hundred-and-eighty-third session's.

## Ledger

Untouched. This is `CLAUDE.md` principle 3 against principle 2 and cites no clause.
