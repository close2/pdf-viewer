# 2265 — Device solids compare by value, and are priced by population

Status: accepted for integration. Session 1715, 2026-10-23.
Context: ADRs 2223, 2235, 2249 (the protocol), 2264 (the decision).

## Identity

ADR 2264's `SolidSource::Gray` and `SolidSource::Cmyk` are equal to another of their variant when
their operands are equal bit for bit, as `Rgb` already was. ADR 2249's identity by allocation was
chosen because a `Function` has no identity to compare; a device family's conversion reads its
family, its operands and the context the memo's key already holds, and nothing else — no profile,
no function, no resource — so two equal selections paint alike in every context, and a value
comparison is exact rather than an approximation of one. It keeps the hit after a restore that
ADR 2249 gave up, and it costs nothing to hold: four floats ride in the enum (the variant widens
`SolidSource` from 16 to 24 bytes, two per `GraphicsState`), where an `Rc` would be an
allocation per `k`. `0.0` and `-0.0` are two keys, which costs a conversion and never a colour.
An `Indexed` entry takes its base's identity: by value over a device base, by profile and value
over `ICCBased`, by allocation over a space ADR 2248 keeps whole.

## Proof

Four omissions, each in an isolated export with its own target, fail on assertions (exit 101):
no inline device source fails four of the seven controls, the `Indexed` arm restricted to an
`ICCBased` base fails six, equality that ignores the operands fails two (the CMYK-to-CMYK restore
and the inherited-colour control), and a memo match that ignores the context fails 32 of the
file's 37 tests. The candidate passes all 37.

## Cost

Twelve balanced pairs of `black_generation_order --icc-cost`, pinned to CPU 2 after 60 s of
settling, in one `--clock` hold with the turn gate; all 24 sides exit 0 at load at most 3.54, and
every population's sample is identical across the two release binaries, whose only production
difference is `content/colour.rs`. The eight device populations are new to the instrument and
are in both binaries. Paired medians per 20 000 marks (20 interpretations of 1 000):

| Population | Before ms | After ms | Paired ratio | Delta ms | Ratio range |
|---|---:|---:|---:|---:|---:|
| gray-repeat | 4.878 | 4.954 | 1.02159 | +0.075 | 0.98523–1.05237 |
| cmyk-repeat | 4.932 | 5.003 | 1.02160 | +0.071 | 0.99275–1.14122 |
| gray-select | 34.043 | 34.636 | 1.01486 | +0.593 | 0.90239–1.04380 |
| cmyk-select | 34.998 | 35.472 | 1.00419 | +0.474 | 0.98203–1.05454 |
| gray-restore | 20.295 | 20.590 | 1.01938 | +0.295 | 0.91478–1.04528 |
| cmyk-restore | 20.878 | 21.361 | 1.02524 | +0.484 | 0.92719–1.06247 |
| indexed-cmyk-select | 6.448 | 7.244 | 1.11769 | +0.796 | 1.03883–1.16328 |
| indexed-sep-select | 9.388 | 10.826 | 1.15304 | +1.438 | 1.12972–1.19028 |
| rgb-solid-values (control) | 35.092 | 35.230 | 0.99541 | +0.138 | 0.94091–1.01871 |
| icc-solid-repeat (control) | 5.302 | 5.332 | 1.00765 | +0.030 | 0.76559–1.05823 |
| indexed-select (control) | 9.268 | 9.143 | 0.98360 | -0.124 | 0.95983–1.06556 |
| sep-select (control) | 9.301 | 9.364 | 1.00387 | +0.063 | 0.94017–1.05633 |

A repeated `g` or `k` paint costs about 4 ns a mark (+0.07 ms per 20 000, under the brief's
0.2 ms), a selection 24–30 ns (0.4–1.5 %, the memo's seed `rg` already paid), a restore
15–24 ns. An `Indexed` entry over `DeviceCMYK` is 40 ns a selection (11.8 %, `entry_of`'s vector
and the seed on a fixture whose selection is otherwise cheap), and over a `Separation` 72 ns
(15.3 %), the base space cloned into its `Rc` as ADR 2249 priced for a `Separation` selected
directly. The 35 other populations, untouched, read paired ratios between 0.97976 and 1.02806.
These are whole interpretations of generated fixtures, not conversion timings; no further cache
is justified by them.
