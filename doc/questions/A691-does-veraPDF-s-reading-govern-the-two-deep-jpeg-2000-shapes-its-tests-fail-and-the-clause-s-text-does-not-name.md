Status: complete
Given: 2026-10-27, in conversation — transcribed by the round
Owes: the clause's text governs — remove the two refusals (`DEEP_OPACITY_AT_ANOTHER_DEPTH` and `DEEP_GREY_WITH_OPACITY`) in `pdf-transform`, keep `pdf-archive` following ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3's colour-channel wording, and record/prepare an upstream issue report for veraPDF's `org.verapdf.external.JPEG2000.parseHeader` (an archive round)

> Please investigate if vera PDFs behavior for q691 is intentional. the spec is what counts unless we find a good reason (maybe the reason why verapdf diverged). if it seems unintentional, shouldn't we open an issue with verapdf?

Reading:
- **Investigation of `verapdf 1.30.3` (`org.verapdf.external.JPEG2000.parseHeader`)**: decompiling `/usr/share/verapdf/bin/cli-1.30.3.jar` shows that veraPDF's failure on both shapes is **unintentional** — a parser shortcut in `JPEG2000.java`:
  1. `parseHeader` only inspects `ihdr`, `bpcc`, and `colr` inside `jp2h`; it never parses the `cdef` (Channel Definition, ISO/IEC 15444-1 Annex I.5.3.6) box.
  2. At `ihdr` (bytecode offsets 221–269), it reads the 2-byte `NC` field (total number of components, logged as `"Can not read number of components"`) and passes it straight to `Builder.nrColorChannels(NC)`. That conflates total components with colour channels: 3 colour + 1 opacity (`NC = 4`) passes test 1 only by accident because `4` matches CMYK's `4`, whereas 1 grey + 1 opacity (`NC = 2`) or 4 CMYK + 1 opacity (`NC = 5`) fails `nrColorChannels == 1 || 3 || 4`.
  3. At `bpcc` (offsets 331–356), it calls `skipBytes` over the entire `bpcc` box without inspecting which channels differ and sets `bpccBoxPresent = true`, while test 5 checks `bpccBoxPresent == false`.
  4. (Additionally, at offsets 302–317, `ihdr`'s `BPC` byte is sign-extended as a signed Java `byte` + 1 without masking bit 7's sign flag `& 0x7F`, so a signed component yields a negative `bitDepth`.)
- **Ruling**: overrules the round's recommendation. Because veraPDF's divergence is an unintentional parser bug rather than a deliberate interpretation, **the specification's text governs**: take off `DEEP_OPACITY_AT_ANOTHER_DEPTH` and `DEEP_GREY_WITH_OPACITY`, keep `pdf-archive` checking the colour channels as the clause states, and file/record an upstream bug report against veraPDF (`org.verapdf.external.JPEG2000`).
