# 2187 — Black generation is reported at the paint that bypasses it

Status: accepted. Session 1675. Context: ADRs 1207, 1242, 2170; ISO 32000-2
§11.7.5.3; the owner asks for a stability review alongside the parallel rounds.

## The event is painting, not parameter selection

§11.7.5.3's first black-generation bullet:

> When painting an elementary object with a DeviceRGB colour directly into a transparency
> group whose colour space is DeviceCMYK , the functions used shall be the current
> black-generation and undercolour-removal functions in effect in the graphics state at the
> time of the painting operation.

The remaining deliberate departure is ADR 1242's: a DeviceCMYK target
characterised by its output intent converts through that profile's B2A table,
not the stated pair. An ICCBased group is outside this bullet. A monotone flag
set by `gs` or a pattern dictionary reported this even when nothing painted,
when only another colour space painted, or when `Q` retired the pair first.

`Conversion::bypasses_black_generation` holds the source, target and current-pair
condition once. Solid paints query it when consumed. Images and shadings carry
its answer with their converted colours through cache hits; the interpreter
records it only when those colours paint. A pattern selected but never used
therefore stays silent. The interpreter's monotone flag records an encountered
departure, not merely a function encountered in a dictionary.

## An image lost the pair on the way to its decoder

The new image witness failed because `conversion_under_intent` built a separate
`Conversion` without its black-generation pair. It now passes the current pair
alongside the image's own rendering intent. RGB (0.2, 0.4, 0.6) gives k=0.4;
BG(k)=1−k and UCR(k)=k/2 give CMYK (0.6, 0.4, 0.2, 0.6). The device press maps
that to RGB bytes [52, 62, 79]. The image formerly painted [51, 102, 154]; it now
matches the derived result. The regression checks the pixel, not only a report.
This is a pixel change; the combined six arms must judge its corpus effect.

A separate probe found that selecting a solid RGB colour before changing the
pair still paints the previously converted colour. That is a distinct cached
state defect, handed to the stability review with the same pixel witness.
The row is `partial` while that owed defect remains; the deliberate profile bypass
stays separately named. It is not concealed by the narrower diagnostic.
`cargo run -p pdf-model --example black_generation_order` preserves the witness:
`0.2 0.4 0.6 rg /GB gs … f` gives [51, 102, 154], while `/GB gs 0.2 0.4 0.6 rg … f`
gives [52, 62, 79]. The next round must carry original colour components or an
explicit conversion dependency to the paint and invalidate/recompute on a
changed pair, testing both orders and q/Q without weakening the arithmetic test.

## Claims reopened by the review

T.801 M.9.2.4 and its enumerated-colour definitions still require the JPX
baseline spaces. A348 says PIMA 7667 was bought and CIE 131 will not be bought.
The held PDF exists; its extraction and e-sRGB/e-sYCC implementation are owed.
The §7.4.9 note and frontier now distinguish this buildable work from CIEJab's
unheld definition. The next build must test non-neutral colours and extended
range from PIMA 7667 and Annex B, alongside declared-space precedence and the
remaining CIEJab fallback. No JPX colour implementation changes here.

§12.2 is partial again: the owner's RFC 0010 instruction adds WinUI alongside
the existing applications, and its executable does not consume chrome or
placement records yet. The three-host evidence cannot establish the new host's
coverage. The note names the Windows work and keeps the existing deliberate
GTK/Qt menu and GTK positioning decisions distinct from that work.
