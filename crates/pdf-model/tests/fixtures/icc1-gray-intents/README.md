# Gray display intent fixture

Generated for audit2038, not measured from hardware. ICC.1:2022 v4.4 monochrome
Display profile, 776bytes. SHA256:
`16de5308d92f234fca71ae05883ff2af647ec7d8c119be270af1b078dd0da8aa`.

The deterministic generator emits required common/gray tags plus three optional
A2B lut16 transforms (§8.2, §8.4.4, Table25, AnnexG TableG.6). The separately
written derivation reads quantized knots and computes screen-output RGB literals.
ICC iccDEV2.3.2.5 reports ValidationStatus.OK with an empty report. That checks
profile conformance; it does not certify a real device or the PDF consumer.
The audit evidence records validator provenance and limits. No validator package
is needed to run the Rust regression.
