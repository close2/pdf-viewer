# 2749 — Overprint mode 1 under a `DeviceCMYK` mark is ADR 1978's named loss where a `/DefaultCMYK` is written, and not a restatement

Session 1968, a single archive round.
Status: **accepted, not built**. It decides the construction for R05's last nine documents of
`doc/pdfa-blocking-issues.md`; the next round on R05 builds it.
Context: ADRs 1978, 2518, 2541, 2693, 2747.
Clauses: ISO 32000-2 §8.6.5.6, §8.6.5.7, §8.6.7, §11.7.4.3; ISO 19005-4 sections 6.2.4.2, 6.2.4.3.

## 1. What stops them

R05's last nine (`363_Risk…`, `E3_Mayhew_et_al_2009`, `LLMImplicationsforScience`, `NeilJairath_3`,
`carp2010-2`, `digitally_signed_3D_Portfolio`, `me450f12project10_report`, `response-to-fiber-concerns[1]`,
`freeculture`) draw `DeviceCMYK` marks with overprinting on under mode 1. A `/DefaultCMYK` over an ICC
profile (ADR 2518's press, ADR 2747's inks) makes those marks `ICCBased` (§8.6.5.6), for which section
6.2.4.2 forbids the mode, so `supplied_cmyk::prepare` writes no default (`OVERPRINT_MODE_ONE`) unless
every such mark's state is one `overprint::zeroing` already sets to 0 for the file's own `ICCBased`
marks. In these nine, none is.

## 2. Measured: the window draws them identically under either mode

Each file was written uncompressed by `qpdf --qdf`, every `/OPM 1` replaced by `/OPM 0` (same length),
and both drawn by this reader on the CPU at 72 dpi: **773 pages, every one with a maximum difference
of 0**. A control drawn twice differed by 0, and the same file with its `0 0 0 1 k` written
`0 1 1 0 k` differed on page 1 by 202 levels, so the instrument sees a change. That agrees with
§8.6.7, which says the mode does not apply where the device's colourants are not CMYK, and with
ADR 1978: this reader composites by the mode only in a page or group blending in four components
(§11.7.4.3), and `zeroing` refuses that case by name.

## 3. Decision: a named loss, not a restatement

The window draws the same, but the mode is the producer's instruction to a CMYK device: under mode 1
a `0 0 0 1 k` mark leaves the backdrop's cyan, magenta and yellow on the plate, and under mode 0 it
paints them out. ADR 1978 decided that, for a `DeviceCMYK` mark, this is `overprint-mode`, a loss
named and recorded in the history (ADR 2693), because a press would show it. That reason does not
depend on which site meets the mode, so `as-shown` may not take it as a statement here either:
identical pixels in the window are the condition for `as-shown` accepting the loss, not a reason
for it to stop being one.

The loss is already accepted by both shipped profiles: `as-shown` and `as-if-printed` answer
`graphics/no-overprint-mode-one-under-icc-cmyk` with `discard`, which authorises
`Loss::OverprintMode`. No new authorisation is decided. The construction:

1. Where the default would be written and every governed `DeviceCMYK` mark under mode 1 has its mode
   set by a graphics state that is an object stating `/OPM`, outside a group blending in four
   components, and the content walk did not stop short, the states are written with `/OPM 0` beside
   the default.
2. The `DeviceCMYK` site's decision then carries `Loss::OverprintMode` beside its own (none for the
   inks, `press-assumed` for the shipped press) and is taken only where it is authorised; every
   other profile keeps `OVERPRINT_MODE_ONE`.
3. The history entry is `overprint::zeroed_history`'s, naming the states and pages, and a packet that
   will not take it withdraws the default (ADR 2693's rule). One set of states is rewritten once
   where both sites ask for it.

## What it costs

`as-shown` and `as-if-printed` would convert the nine at part 4 with one more named loss; no pixel of
the window moves.
