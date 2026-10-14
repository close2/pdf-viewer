Status: complete
Given: 2026-10-08, in conversation — transcribed by the round
Owes: none — "until a launcher exists nothing is owed", and the launcher is not this tree's to ship yet

> For everything else I agree with your suggestions, please answer them.

Reading: the suggestion adopted is the question's own, quoted and attributed: *"Accept the cost
in the binary, and set the variable in the launcher when the tree ships one."* The sentences
adopted with it:

- the binary stays as it is; lavapipe's ~3.5 ms of device bring-up is the driver stack's cost,
  and ADR 1658's node-wake keeps what it recovered;
- `VK_LOADER_DRIVERS_DISABLE=*lvp*` is a line for a desktop file's `Exec=`, written the day the
  tree ships a launcher;
- the GPU-less fallback is measured on a machine without one before that line ships;
- the launch gate goes on measuring the unfiltered launch, so the published band is a claim
  about the binary and not about the launcher.

Nothing in the tree changes on this answer; it records the decision the packaging round needs.