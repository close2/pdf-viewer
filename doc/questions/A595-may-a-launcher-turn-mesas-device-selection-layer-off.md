Status: complete
Given: 2026-10-26, in conversation — transcribed by the round
Owes: the launcher line, written the day a launcher ships: `NODEVICE_SELECT=1` set only where the
person has set neither `DRI_PRIME` nor `MESA_VK_DEVICE_SELECT`, placed beside A270's
`VK_LOADER_DRIVERS_DISABLE=*lvp*` line, and tried on a machine with two GPUs before it lands
(acting round, when a launcher exists — A270: until one does, nothing is owed)

> agreed on all

Reading: one word answered the three questions presented together, and it adopts each
recommendation as presented. This one is Q595's own, quoted and attributed: *"Accept the one
asking in the binary, and let a launcher set the variable only where the person has set neither
`DRI_PRIME` nor `MESA_VK_DEVICE_SELECT`."*

What the answer settles:

- **The binary keeps the one asking.** ADR 2298 already took the launch from two enumerations
  to one; the remaining ask-the-display-server step (3.7–5.8 ms of it in the adapter check
  without the layer, against 12.1–21.0 with) stays in the binary, and the launch gate goes on
  measuring with the layer on — so the saving is never claimed at the cost of the measurement.
- **A person's GPU choice outranks the program's speed.** The variable is set only where the
  person has not already spoken through `DRI_PRIME` or `MESA_VK_DEVICE_SELECT`, so both keep
  reaching this program; the condition is checked by the launcher, not the desktop entry, and
  the two-GPU trial comes before the line is trusted.
- **The 8–15 ms is on the ledger the owner's own startup rule keeps**: launch latency is a
  first-class requirement, and this is one of its items bought without an `unsafe` in a crate
  that touches PDF bytes — the loader reads the variable from the environment, so it belongs to
  a launcher, not to the binary.