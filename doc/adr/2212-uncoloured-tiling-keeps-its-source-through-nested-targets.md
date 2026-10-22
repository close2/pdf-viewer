# 2212 — Uncoloured tiling keeps its source through nested targets

Status: proposed; scoped proof and cost evaluation in progress. Session 1689.
Context: ADR 2199; ISO 32000-2 §§8.7.3.3, 11.6.7 and 11.7.5.3.

An uncoloured tiling selection retained only a converted colour. Its conversion used
default rendering parameters and no black-generation pair. A retained selection
therefore could not follow the parameters of a later painting operation. The fixed
baseline reproduces RGB(.2,.4,.6) as [51,102,154], while BG(k)=1-k and UCR(k)=k/2
require CMYK(.6,.4,.2,.6), which the held device press displays as [52,62,79].

Retain the resolved underlying space and its consumed components with the immutable
selected tiling. At each use, bind that source to the mark's rendering parameters
and generation pair. Carry the binding separately in the cell's graphics state;
ordinary state cloning then owns its lifetime through q/Q, Forms and glyphs. A solid
paint or stencil resolves the binding through the existing complete Conversion in
its current target. The source is not reconstructed from inherited RGB metadata.
There is no new conversion cache, geometry cache or manual invalidation protocol.

A single converted outer colour is insufficient. An isolated Gray group inside the
cell must receive Gray quantities. Existing group admission deliberately refused
uncoloured content because its source was unavailable. Admission now distinguishes
source-backed tiling from uncoloured content without that binding. The latter,
including an ordinary Type 3 d1 glyph, retains its previous refusal. CMYK halves and
other nested targets use the same source-availability boundary.

The binding does not replace the cell's definition state. Section 11.6.7 initializes
that state from the enclosing stream's entry state and resets transparency values.
Section 11.7.5.3 separately assigns a group's exit conversion to the state at its Do.
Thus a pair introduced only after the enclosing Form began can affect direct tint
conversion without replacing the parameters of an RGB group's Do inside the cell.
The tests distinguish these two cases; an assertion that both must always match an
ordinary solid would erase the definition-state rule. Forbidden colour operators
remain forbidden inside the uncoloured definition.

Alpha remains outside source conversion, and the pattern's existing composition
applies the mark's alpha once. Soft masks retain their independently initialized
state. Profiles, spots and target identity still belong to existing Conversion;
this change does not replace that context with a smaller key.

The first correct candidate converted every elementary tint paint directly. Twelve
quiet, CPU-pinned balanced pairs rejected its cost: whole-interpretation medians for
20,000 pattern marks were 70.181→199.762 ms for one cell, 751.943→891.094 ms for many
cells, 130.386→172.443 ms for alternating tints and 142.430→149.081 ms for changed pairs.
All samples, including ordinary-solid timing variability, remain in the receipt.

The revision reuses the existing two SolidColours slots for a resolved DeviceRGB
source with exactly three components. Ordinary RGB and bound RGB tints share one
helper constructing the existing complete borrowed ConversionContext. Binding
rendering/generation and the current target/profiles/spots/separation state all
participate. Owning cached Conversion preserves identity-bearing lifetimes; alpha
and actual-paint diagnostics remain outside the memo. Non-RGB and malformed short
RGB inputs retain direct conversion. No cache field, key or invalidation rule is
added. Cell setup seeds or reuses the fill slot; an initial stroke may miss its own
slot, which changes cost rather than colour. Twelve focused controls, all-target
Clippy and formatting pass for this revision; independent code review found no
concrete cache/lifetime defect. The revised model+colour suite passes 2285 tests
with 21 skips (156-second hold, 3.91 GiB). Revised cost acceptance remains pending.
