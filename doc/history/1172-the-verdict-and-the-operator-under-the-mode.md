# 1172 — The verdict is a command, and the mode is an operator somebody already has

Date: 2026-09-22. Branch: `batch-1171-1176`, worktree shared with five siblings. ADRs
[1181](../adr/1181-the-verdict-is-a-command-and-not-a-decision.md) and
[1182](../adr/1182-two-compositing-operators-chosen-per-channel.md); question
[Q76](../questions/Q76-who-builds-the-seventeenth-mode-in-quorra.md).

**The flag two backends refuse on is settled against the commands.** `DisplayList::overprints`
was set where §11.7.4.3's mode is *chosen*, and three shapes separate choosing from emitting: a
painting operator asks for both of §8.6.7's parameters before it knows which part marks the page;
text rendering modes 3 and 7 and a hidden layer mark nothing; and a group that gives up its own
colour space runs its content again with the first run thrown away. `settle_overprinting` walks
the finished list — commands, nested groups, a `Shaped`'s object, `GroupBlending`'s black half and
every soft mask's elements — gated on the old flag as a hint, beside `noninvertible_marks`, whose
comment already gives this reason. Re-run over the same 65 944 documents: the census's own
verdict-without-a-mark column is **0**, was 177 of 10 040; documents the mode reaches 1826 → 1788.
`raster_golden` moved **0** digests, so nothing drawn changed.

**What the mode is.** Substituting §11.7.4.3's two values of `B` into §11.3.6's formula collapses
it: a kept component composites Porter-Duff **destination-over**, every other **source-over**, one
union alpha — not a seventeenth blend function but two compositing operators chosen per channel.
Held against `render-cpu`'s own `composite`, calibrated so the two differ by more than eight-bit
rounding in every channel.

**Neither backend can be given it from here, and the ask got smaller.** `raster_scene::Compose`
has `SrcOver`, `Src`, `DestOut` and `Plus`, and the staged pair cannot make destination-over — its
missing factor is the destination's alpha per pixel. Vello has `Compose::DestOver` but composites
a layer whole, so a proper subset of channels is out of reach there too; and
`render-gpu` refuses every four-component page and group before the overprint test anyway, so what
its refusal costs is one soft-mask position, not the 1788 documents ADR 1178 charged to both. The
census now counts the *shape* of each kept set: of 27 435 261 marks, 13 772 602 keep all three
channels, 13 650 173 keep none and **12 486 keep a proper subset** — so **9734 of 9863 pages and
1711 of 1788 documents need nothing but `Compose::DestOver`**. `QUORRA_FEEDBACK.md` section 49 is
restated in those terms. `raster/` is another project with its own team whose documents do not
mention overprinting at all; Q76 asks the owner who builds it.

Files: `pdf-render/src/display_list.rs`, `pdf-model/src/content.rs`,
`pdf-model/tests/overprint.rs`, `pdf-model/examples/overprint_ink_group_census.rs`,
`render-cpu/src/blend.rs`, `render-raster/tests/corpus.rs`, `doc/QUORRA_FEEDBACK.md`,
`doc/conformance/ledger.toml` (§11.7.4, §11.7.4.3, §11.7.4.5), `doc/todo/23`, `doc/verify.md`.
