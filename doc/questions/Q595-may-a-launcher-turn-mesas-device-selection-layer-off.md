# Q595 — May the program's launcher set `NODEVICE_SELECT=1`, or is the device-selection layer's probe accepted?

Asked by round 1732, which made the launch enumerate the machine's graphics adapters once instead
of twice (ADR 2298) and measured what is left on the owner's session (ADR 2299).

## What the cost is

Mesa installs an implicit Vulkan layer, `VK_LAYER_MESA_device_select`, which every Vulkan program
loads. It reorders the physical devices so that the GPU the display server uses comes first, and to
learn which one that is it asks the display server on each enumeration. Before ADR 2298 a launch
enumerated twice and the two askings were 13 to 25 ms of the device's bring-up on the owner's Wayland
session (ADR 2282). The launch now enumerates once, and the one asking that is left is in the
adapter check. Measured on the owner's session in one loop run, the launch gate's bring-up child and
`quorra` interleaved, five and four of each (ADR 2299):

| | layer on | `NODEVICE_SELECT=1` |
|---|---|---|
| adapter check, gate child, ms | 12.1 to 21.0 | 3.7 to 5.8 |
| bring-up, gate child, ms | 34.4 to 52.2, median 37.5 | 30.0 to 42.3, median 31.1 |
| `quorra`'s first present, ms | 63.1 to 78.0 | 53.6 to 66.3 |

So the variable would take a further 8 to 15 ms off the device's bring-up, at one-minute loads of 4.1
to 12.4.

## What setting the variable gives up

`NODEVICE_SELECT=1` is the layer's own `disable_environment` (its manifest under
`/usr/share/vulkan/implicit_layer.d/`): the Vulkan loader reads it when the instance is made and
leaves the layer out of the process. The layer is also how a person chooses a
GPU for a Vulkan program: `DRI_PRIME=1` and `MESA_VK_DEVICE_SELECT=<vendor:device>` are read by it.
With the variable set, both stop reaching this program. What that loses is narrower than it sounds:
this program ranks adapters by type before order — discrete, integrated, other, virtual, processor,
as `request_adapter` does under `HighPerformance` (ADR 2298) — so the layer's order already decides
only between two adapters of one type, two discrete GPUs, and a `DRI_PRIME` that names an integrated
GPU beside a discrete one is passed over today with the layer on. The other loss is
`MESA_VK_DEVICE_SELECT_FORCE_DEFAULT_DEVICE`, which hides every device but the chosen one.

A desktop entry is written once for every machine, so "where one adapter is known" cannot be decided
by the entry itself. A wrapper script could count the machine's render nodes, or set the variable
only when the person has set neither `DRI_PRIME` nor `MESA_VK_DEVICE_SELECT`.

## Why the binary cannot do it

As for Q270: the loader reads the variable from the process's environment, and
setting it from inside the process is `std::env::set_var`, which is `unsafe` in this edition and
which `CLAUDE.md` principle 3 forbids in a crate that touches PDF bytes (ADR 2283). The tree also
ships no desktop entry and no launcher yet (A270: "until a launcher exists nothing is owed").

## The question

When the tree ships a launcher, may it set `NODEVICE_SELECT=1`? And if so: always, only where the
person has set neither `DRI_PRIME` nor `MESA_VK_DEVICE_SELECT`, or only where the machine has one
GPU?

## Recommendation

**Accept the one asking in the binary, and let a launcher set the variable only where the person
has set neither `DRI_PRIME` nor `MESA_VK_DEVICE_SELECT`.** That keeps a person's own choice of GPU
working and takes the asking off every other launch. It belongs beside A270's
`VK_LOADER_DRIVERS_DISABLE=*lvp*` line, written the day the tree ships a launcher, and should be
tried on a machine with two GPUs first. The launch gate goes on measuring with the layer on.

**What the tree does meanwhile:** nothing changes. The launch enumerates once (ADR 2298), and the
toolkit windows and the launch gate print the asking's share as the adapter check's step.
