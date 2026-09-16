# 1148 — §11.7.5.2's carrier is a channel beside the list, and both backends apply it

2026-09-16. Files: `pdf-render/src/{transfer_channel.rs (new),display_list,lib}.rs`, `pdf-model/src/
content{.rs,/marked,/path,/text,/image,/pattern,/transparency}.rs`, `render-{cpu,raster,gpu}/src/lib.rs`,
`viewer-confined/src/protocol/display_list.rs`, both `tests/transfer_edge.rs`, `pdf-model/tests/
{transfer_functions.rs,raster_golden.tsv}`, ADR 1125, `doc/todo/13`, ledger, this file. Every other
change in the tree — `viewer-*`, `pdf-model/src/{variable_text,view,file_spec,…}.rs` — is a sibling's.

## The carrier

Measured first: **272** `Command::Fill`/`Image` occurrences over 11 crates, so no field on the mark;
a side-table keyed by position cannot reach a mark inside a group (trap 5). What reaches both is a
**channel on the `DisplayList`**, the shape `set_blending`'s companion list already has.
`TransferBuilder` collects every elementary mark at leaf creation — `Interpreter::draw_mark`, the one
route every mark takes, nesting included — as an opaque *shape* with the function in force, grouping
consecutive marks sharing one into **runs** whose numbers rise with painting order, so the topmost
object covering a pixel is the one in the highest run covering it. `pdf_render::resolve_transfers`
walks runs top down, takes the first whose coverage is **nonzero** (the clause's own words, so an
antialiased edge belongs to the object whose edge it is) and maps the finished pixel once (§11.7.5.3's
NOTE). Both backends call it with a closure rasterising one shape list — one statement of the rule
(trap 2), every mark rasterised once at most, the builder inert until one carries a function.

Both fixtures assert the clause's **0.375** where they asserted the pipeline's 0.875, and that the
two backends agree; the no-transfer control is unchanged. `issue6931_reduced.pdf` is the only corpus
**raster** that moves — ink **3.45863** against poppler 2.96392, hayro 3.44317, ghostscript 3.48165,
closer to the three that apply §10.5 than before, its samples now resampled raw and mapped at the device
pixel. The other 965 golden rows are *list only*: `DisplayList`'s `Debug` gained a field. Cost:
on ISO 32000-2 p101 (no transfer) the channel executes **no** instruction; on `issue6931_reduced`
`encode_in_strips` is 15.95% over four calls (two draws × page + shape run) — ~26 M Ir of a ~330 M draw.

## What remains: two paints

A *shading*'s ramp is sampled under the function where its colours are made (ADR 0479) and a *tiling*
cell is interpreted once and copied to every site (ADR 0430), so one shape cannot stand for its
marks: both keep §10.5's pre-composite application, `Interpreter::tile` records the finished tiling as
occluders so nothing beneath is mapped twice, and `Unsupported::TransferFunction` is narrowed to those
two. **ADR 0479's reason does not survive the channel** — a raw ramp simplified and mapped per
device pixel has no chord — so a later round can move a shading across; an amendment to 0479, not taken
here. `render-gpu` refuses such a list by name and `viewer-confined` crosses it as pixels the CPU
backend drew. §11.7.5.2 stays `partial` for the two.
