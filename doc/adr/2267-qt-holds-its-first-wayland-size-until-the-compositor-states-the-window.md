# 2267 — Qt holds its first Wayland size until the compositor states the window

Status: accepted. Session 1716, 2026-10-23.

## Context

On Wayland Qt states a size before the compositor's first configure: the size it asked for, at the
output's scale rounded up. On the owner's panel at 1.6 that is ratio 2, and the fractional scale
arrives with the configure 29 to 45 ms later. `PageArea::resizeEvent` told the core each size as it
came, so the core asked for page one at ratio 2, the device lane drew it — 1191×1684 where the
window wanted 953×1348 — and a draw in flight cannot be taken back (ADR 0725), so the page at the
right ratio waited behind it. On the owner's session (ADR 2266's loop run) every one of seven
Wayland launches drew page 20 at ratio 2 first, for 26.8 to 88.4 ms of the device's or the processor's time, and the
first picture shown was that raster resampled to 1.6. ADR 2251's report had priced this at one
layout under 5 ms; that did not hold.

## Decision

On the `wayland` platform only, `PageArea` holds the first non-empty size and tells the core
nothing until one of three things says what the window is: a later size or ratio that differs
(the configure), Qt's `DevicePixelRatioChange` (the fractional scale with no change of logical
size), or the first paint, which follows the configure and so says the held size stood; the held
size is then told after that paint returns. X11 states the window's own size first and is told at
once, as before. `PageArea::viewportChanged` is the one place the viewport is told, and it sends
nothing where the device size and ratio repeat what the core already holds. The same
`DevicePixelRatioChange` now re-measures a settled window taken to an output of another ratio,
which no resize states — the Qt half of ADR 2250's `notify::scale`.

Measured on a nested KWin at 1.6 on the owner's adapter (ADR 2251's construction, `NODEVICE_SELECT=1`
so that Mesa's device-selection layer, about a second on the nested compositor and 135 to 227 ms in
all on the owner's session, does not hide the page), four launches each, interleaved: the first
paint at 1.6 at its own scale comes 118 to 153 ms after the first size (median 134) against HEAD's
171 to 200 ms (median 179); no page is drawn at ratio 2; and the first picture is the page itself,
not a resampled one. At whole-number scales nothing is drawn twice where the configure keeps the size,
and the first frame read 307 against 321 ms at 2 and 349 against 378 ms at 1 (three launches
each), inside the run's spread.

## Costs

Where a Wayland compositor's configure keeps both the size and the ratio, the core hears of the
viewport at the first paint rather than at the first size: on the nested KWin at scale 1 that is
31 to 92 ms later, which the same launches did not show as a later first frame because the device
was not up before it. The `toolkit-launch` gate runs on Xvfb, where nothing changed, and cannot see
either side; this ADR's figures are the Wayland side's only measure.

On X11 the hold does not act, and the gate read the tree's `device_page_one_ms` at 5.8 to 6.0 ms
against its floor of 5.9 while HEAD read 6.0 to 6.2 in the same holds. It is the address layout and
not this change: under callgrind the device's thread executed 142.80 million instructions in the
tree against 142.15 million in HEAD's build for the same launch, and with `setarch -R` in one
`--clock` hold (habit 121) the tree read 6.9 and 7.2 ms against HEAD's 6.9 and 7.0, every figure
inside its band. The band is left as it stands.
