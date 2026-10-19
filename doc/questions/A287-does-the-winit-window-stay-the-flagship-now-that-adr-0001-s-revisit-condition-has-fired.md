Status: complete
Given: 2026-10-19, in conversation — transcribed by the round
Owes: the superseding record for ADR 0001 — its fired revisit condition answered, the Qt dependence stated, the "little conventional GUI" premise retired, "do not depend on Qt" turned into a rule about the reference's own dependencies; the Windows-native .NET host on the `viewer-core` boundary, beside the established windows and the C ABI; and a GPU rendering path in `quorra-gtk`, `quorra-qt` and the .NET host wherever its platform admits one (acting rounds)

> The reference stays winit, but we will add a windows native (dotnet) version in addition to the other applications. If possible gtk, qt, and dotnet should provide a way to still render on the GPU.  We will also introduce a macos native version, so the modifications should keep this in mind.  However: we do not implement the macos version right now.

Reading: **The reference stays winit.** The flagship question is answered in the shape's favour:
nothing is re-licensed, and ADR 0001's revisit condition — "revisit only if conventional widget
requirements appear" — is answered by the record it is owed, which the owed work writes: the Qt
dependence stated, the retired premises retired, and the decision turned into a rule about the
reference's own dependencies alone.

**The application set grows.** A Windows-native host, written in .NET, joins the established
windows and the C ABI on the same boundary — one more consumer of a vocabulary that already
carries three toolkits, and the level-hosts rule and `viewer-host`'s shared decisions exist to
be matched by it.

**The GPU is not given up with the toolkit.** Where a native host's platform still admits
device rendering, the host keeps a path to it: `quorra-gtk`, `quorra-qt` and the .NET host
should provide a way to still render on the GPU. The known limit stands where it stood — a GTK
4 widget has no surface GSK hands out (ADR 0604) — and "if possible" is the owner's own word for
exactly that class; where a platform admits no device, the processor remains the lane.

**A macOS host is in the plan, and is not to be built.** Every host-side modification keeps it
in mind — the boundary's messages, the host abstractions and the GPU-path hooks must not bake in
what only Linux and Windows offer — but nothing implements it until the owner says otherwise.