# 2212 — Uncoloured tiling keeps its source through nested targets

Status: accepted with measured cost; final combined corpus/launch checks remain due. Session 1689.
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
with 21 skips (156-second hold, 3.91 GiB).

The revised comparison accepts a bounded correctness cost. Twelve balanced pairs
pinned to CPU 2, with the same fixed baseline and identical instrumentation, pass
all 48 unchanged quiet guards (maximum load 2.82 and mean GPU activity 1%). The
exclusive nice-0 hold takes 394 seconds, 499 including queue, and 0.07 GiB. Pattern
whole-interpretation medians before→after are 70.661→71.387 ms for one cell,
745.647→754.007 ms for many cells, 130.442→134.246 ms for alternating tints and
143.097→148.880 ms for changed pairs, each over 20,000 marks. Paired median ratios
are 1.01723, 1.01093, 1.02955 and 1.04436; paired median deltas are 1.209, 8.1575,
3.874 and 6.267 ms, or approximately 60–408 ns per mark across these cases.

All samples remain included. Changed-pair ratios span 1.020–1.453, including the
214.685-ms final candidate sample. Ordinary-solid controls have paired medians
0.94371, 0.98609, 0.98291 and 1.00658; their variability, especially the bimodal RGB
case, does not establish a speedup. This is not a performance-neutral claim or an
isolated conversion microbenchmark. Document opening and rasterization are outside
the timer; content parsing and lazy interpretation work are inside. Existing-cache
reuse removes the rejected large regression without introducing another state key.

The revised immutable binary produces all four saved PNGs byte-identically to the
correct uncached candidate. Inspected tint pictures match the independent arithmetic
above. Exact source/binary manifests, all samples and guards are retained under
scratchpad/r1689/{cost-before,cost-after,cost-cached,paired,paired-cached}; final
combined corpus and launch gates remain separate integration obligations.
