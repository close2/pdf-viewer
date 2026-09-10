# 949 — The licence the standard states, and the condition it carries

Date: 2026-09-10. ADR: 0949.
Files: `crates/pdf-model/src/xmp.rs`, `crates/pdf-transform/src/archive.rs`, `tests/archive.rs`.

The DeviceN `/DefaultCMYK`, which ISO 19005-2 section 6.2.4.3's NOTE 2 makes device independent
and which ISO 32000-2 §10.4.2.5 supplies the tint transform for, so nothing about it is invented.

**The round corrected its own brief's arithmetic before writing anything.** The brief said 32
documents; only 13 were ever reachable, because part 2 admits "a device independent `DefaultCMYK`
**or a DeviceN-based** one" and ISO 19005-4 states the same sentence with that half absent. The 19
part-4 documents stay refused, and that is the standard's difference rather than the converter's
gap. Twelve of the thirteen convert; the thirteenth already holds a **GRAY** destination profile
where §10.4.2.5 produces RGB, so the alternate space cannot be that profile — refused with that
sentence.

**`A48`'s permission and its condition are one operation, enforced by data flow.** `Prepared::of`
builds the default and then the packet; if the `xmpMM:History` event cannot be written, **the
default is withdrawn**. A permission that could land without its condition would be ADR 0927's
argument reduced to a hope about callers.

The tint transform is a type 4 PostScript calculator function whose program is §10.4.2.5's
arithmetic, and the test **evaluates the written stream back** through `pdf_model::function` at
five points — so the file is checked as a function rather than as bytes. §7.10.5.2's operator set
has no `min`, which is why the clamp is written the way it is.

A supplied CMYK profile takes the output intent and **no default is written at all**, which is
§10.4.2.1's own ranking of the ICC route above the approximation, applied to a conversion.
