# 2241 — GTK source and window pixels need an independent map

Status: diagnostic experiment; integration record follows in the batch handover.
Session 1703, 2026-10-22.

## Protocol

Use the owner's ISO_32000-2_sponsored_EC3.pdf, page 20, in an owned GTK X11 window.
Calibrate input at three points, select SinglePage, then capture fit, two discrete key
zoom-outs and fit return. Join each requested source dump to a later native paint in the
same generation. Keep rejected attempts and require no intervening refresh while xwd
captures the owned window. This is a settled key-input experiment, not a wheel-latency
or deep-zoom anchor test.

Infer the page rectangle independently from exposed white edge runs. Require a unique
matching aspect ratio and inspect alignment before comparing opaque source pixels with
the screenshot. Native/client dimensions alone do not establish a coordinate transform.
X11 backing pixels do not observe later compositor scaling or scanout.

## CPU rehearsal

Owned Xvfb/software-rendering rehearsal passes four captures. Independent visible-edge
mapping finds 609×862 at (386,130), 488×690 at (447,216), 390×552 at (496,285), and
609×862 at (386,130). Every mapping is 1:1. Entire opaque page and interior text-region
pixels match exactly: zero maximum channel difference. This rules out added presentation
blur only for these settled CPU states, not low-resolution source rendering, transient
stand-ins, hardware, fractional compositor scaling or native Wayland.

The first wrapper stopped before launching the viewer because Xvfb -displayfd did not
create the assumed lock file. The corrected owned-process/display-pipe check passes;
the failed attempt is retained. Real-display results belong to the final handover.

## Real-adapter attempt

The immutable release build passes (234 s hold, 1.91 GiB). Two finite owner-loop
trials reach AMD Radeon 890M/RADV and the correct owned X11 window, but both refuse
before settled capture: no fresh motion callback. The diagnostic second trial records
an active, unlocked Wayland session and the test window as active/focused. Requested
window-relative (560,325) does not move the reported pointer from (1849,547), window
1041, before/after the warp; the test window is 16777220 at (916,345). Its 23-pixel
GTK shadow extents explain the outer/client size difference, not the absent callback.
The screenshot shows the intended document. This establishes input delivery failure
in the experiment, not its compositor cause or any blur result. No guard was weakened.
Escape was sent only to the PID-verified test window; its timeout used bounded TERM.
Each wrapper held 13 s, .02 GiB on the agent side (not the owner's viewer memory).
Both failed trials and diagnostics remain evidence. Hardware sharpness is unjudged;
no further display trial was started before handover.
