# 1041 — a tint is interpolated before it is converted, and a patch is asked once

Date: 2026-09-14. ADR 1058. §9.8's and §8.7's twelve `partial` rows, each read against the clause in
`doc/md/` and not against its note. Five rows moved: three by code, two by reading.

**§8.7.4.4 — `partial` → `implemented`.** A mesh in a CIE-based, `Separation` or `DeviceN` space now
keeps its stated components through the subdivision and converts *after* interpolating — "all
gradient fill calculations shall be performed in that space"; an `Indexed` space is "immediately
converted to the base colour space" and takes the base's rule. What makes that true of every point
on a display list of device colours is a subdivision measured against §10.7.3's tolerance — a triangle
is split until a rasteriser's plane between its converted corners is within the tolerance of the
conversion of the interpolated components — with two bounds and one report,
`max_mesh_refinement`. Three fixtures in `tests/shadings.rs`: the triangle draws 65 where 129 was
drawn, the Coons patch 63 where 127, the stepping tint reports; each was planted back and failed.
**The corpus cannot show it**: `examples/mesh_census`, now over `doc/corpora` too, finds three
documents the rule reaches, every conversion in them linear — no triangle refines, no pixel moves at
1× or 4× (trap 8, said in the row). **The first shape's cost is the lesson**: converting every grid
vertex and midpoint took `personwithdog.pdf` from 92 ms to 2.63 s for nothing, so a patch is asked
once at nine grid vertices whether its conversion is linear; the page is at 96.8 ms. ADR 1058.

**§9.8.2 — `partial` → `implemented`.** "[S]hall be an unsigned 32-bit integer" had two readers that
disagreed on a word outside the range; `metrics::flag` is both now and reads such a word as no entry.
`examples/font_flags_census` counts the population: of 3628 descriptors in 1450 documents none writes
one, seventeen set a reserved bit, four break Table 121's Symbolic/Nonsymbolic sentence. The three
unread bits bind no reader; ForceBold's is a `may` this device is inside either way.

**§9.8.1 and §8.7.4.1 — `partial` → `implemented` by reading.** §9.8.1's four reader-side `shall`s
are each executed and the ten unread entries "provide information that enables" a synthesis this
tree does not attempt — an entry that enables is not one that is owed (§11.4.1's reading of a
permission); what leaving them unread costs is ADR 0267's, the oracle's denominator. §8.7.4.1 was
`partial` for entries "not this clause's to fix", and a departure another row records is that row's.
**§9.8 narrowed** to `/FD`. Untouched: §8.7.3, §8.7.3.1 (ADR 1031), §8.7.4.5.7/8, §9.8.3, §9.8.3.1, §9.8.3.3.

Siblings had the shared worktree mid-edit most of the round, so the gates ran in a private worktree
at the same HEAD with only this diff, every file then mirrored back byte for byte:
fmt 0; clippy `-D warnings` 0; `nextest --workspace` 4600/4600 (a first pass failed one conformance
test on a sibling's in-flight rename served stale from the shared target directory; the re-run and
`nextest -p conformance` 267/267 are the tree's answer); doc 0; fuzz fmt and clippy 0; conformance 0;
`raster_golden` **held 974, moved 0**; `pdf-model --test corpus` 0 with no `max_mesh_refinement`
anywhere; oracle `our_rendering_agrees_with_the_reference_consensus_across_the_corpus` ok, 3/3.
