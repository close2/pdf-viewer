# 2299 — The bring-up band stands, and the device-selection layer is a launcher's question

Status: accepted — the band is not re-taken; whether a launcher may set `NODEVICE_SELECT=1` is asked
in `doc/questions/Q595`. Session 1732, 2026-10-23.

## Context

ADR 2298 made a launch enumerate the graphics adapters once. Batch one hundred and twelve's brief
asked for `doc/checks/launch-path.toml`'s bring-up band to be re-taken by ADR 1969's rule only if the
headless figure moved, and for the layer that the remaining enumeration still pays to be put to the
owner as Q270's question was, or for this ADR to say why not.

## The band

The change moves one step of the headless bring-up: the adapter step, 0.228 to 0.344 ms on HEAD and
0.001 ms on the tree over twelve interleaved children each (ADR 2298). That is under a twentieth of
the band's 13.6 to 20.6 width, and under the spread of instance creation, a step the change does not
touch, which read 12.6 to 21.6 ms across the same 24 children. ADR 1969's rule re-takes a band whose
figure moved; this one moved by less than its own noise, so the band and its reason stand, and its
header now says what a second enumeration's return would read.

The two gates ran once per arm in the same `--clock` hold, at loads of 4.9 to 6.2:

- `t2-launch_path` exited 101 on both: HEAD on two warm opens and the tree on two others, each a
  sub-millisecond figure whose open executed the same instructions in both arms (3 972.9 against
  3 972.6 thousand on `PDF20_AN001-BPC.pdf`, 3 370.2 against 3 369.8 on `bug1815476.pdf`) — the
  machine's, by habit 101. Bring-up read 22.6 ms on HEAD, not judged for the carve-out, and 19.1 on
  the tree, in band.
- `t2-toolkit_launch` exited 0 on both. `device_up_ms` read 28.4 and 29.2 on GTK, 42.0 and 44.8 on Qt,
  HEAD first; the harness's display is its own Xvfb, and nothing on these rows is claimed for the
  change either way.

## The layer

Mesa's `VK_LAYER_MESA_device_select` asks the display server on every enumeration, and the one the
launch still makes is the adapter check, which a launch cannot skip (ADR 1532 decides the backend set
on it). On the owner's session the check read 12.1 to 21.0 ms with the layer and 3.7 to 5.8 ms under
`NODEVICE_SELECT=1`; `quorra`'s first present read 63.1 to 78.0 against 53.6 to 66.3 ms.

The variable is the layer's own `disable_environment`, read by the Vulkan loader from the process's
environment, so the binary cannot set it without `std::env::set_var`, which is `unsafe` (ADR 2283);
it is a launcher's, as A270's `VK_LOADER_DRIVERS_DISABLE` is. It is not a free line, because the
layer is how `DRI_PRIME` and `MESA_VK_DEVICE_SELECT` reach a Vulkan program, so Q595 asks the owner
whether a launcher may set it and under which condition, and recommends setting it only where the
person has set neither. The tree ships no launcher, so nothing changes meanwhile.

## Costs

The band stays a claim about one enumeration's process without a display, and the display's share is
judged by no gate; the printed steps are what show it. Q595's figures are from one adapter and one
compositor.
