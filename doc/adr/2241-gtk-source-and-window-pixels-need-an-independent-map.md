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
