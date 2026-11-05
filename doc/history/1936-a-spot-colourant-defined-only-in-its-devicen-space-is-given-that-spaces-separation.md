# 1936 — A spot colourant defined only in its DeviceN space is given that space's Separation

Session 1936, a single archive round. ADR 2690; no question; ADR 2691 not used.

**The premise.** It held for six of R12's seven documents and not for the seventh. Six use a Type 4
tint transform: `colorspace_atan`, `_cos` and `_sin` (`[/X /Y /Z]` over `DeviceRGB`), `issue17065`
(`[/L /A /B]` over `CalRGB`, written twice), `issue9940` (`[/IBM /None /None /None]`, with its
attributes an object of its own) and `postscript_type4_many_outputs` (nine colourants over
`DeviceCMYK`). `devicen.pdf` uses a sampled one (FunctionType 0). None is `NChannel`. The census
names R12 as the only refusal of all seven under both shipped profiles at both parts.

**The construction (ADR 2690).** `archive/spot_separation.rs` writes the `Separation` from the
space: same colourant, same alternate, and a Type 4 stream holding the producer's program with the
other inputs pushed after its brace as reals, each zero clipped to its `Domain`, and `k+1 -1 roll`
to move the input into place. A one-component space's own transform is the entry. There is one
array per colourant across the file. An indirect attributes dictionary is copied into the space.
A page inheriting a node's direct resources gets the node's entries in its `PageResources` copy;
`issue17065` needed this. No loss, so it stays `Mechanical`, and `no-data-loss` takes it.

**Refusals kept, by name**: a sampled transform; a Type 2 or 3 transform over more than one input;
an `NChannel` space, where the entry changes what is drawn; two spaces that would derive different
arrays; a program not opening with `{`.

**Tests.** `archive/spot_separations.rs` (six): a 41-step grid per colourant over a program with a
domain excluding zero, at both parts; a one-component space; sampled; `NChannel`; shared and
differing spaces; an indirect attributes dictionary. Planting `1 roll` for `-1 roll` failed the grid.

**Census.** Against round 1935's runs. Shipped: 24 new outputs (the six, both profiles, both
parts), none gone, the rest identical once masked; pdf.js moves 900→906, 910→916, 835→841,
839→845. `rank.py --table` puts R12 at 1 / 1 (`devicen.pdf`). `issue9940` at 2b under
`no-data-loss` and `state-every-loss` is refused by `metadata/extension-schemas-embedded`. Each
output renders the same with and without its `Colorants` entries; its differences from the source
come from the output intent.

**Gates.** fmt (`pdf-transform`, `pdf-archive`): exit 0. clippy `-D warnings --all-targets`
(`pdf-transform`): exit 0. nextest of `pdf-transform` and `pdf-archive`: 1381 passed, exit 0.
Archive tier (`corpus`, `archive_corpus`, `witnesses`): exit 0 each; both censuses exit 0.
veraPDF 1.30.3: the 24 new census outputs and the 46 outputs of every profile pass. `cargo test
-p conformance` after `git add -N`: exit 0. `state.sh docs`: exit 0.
