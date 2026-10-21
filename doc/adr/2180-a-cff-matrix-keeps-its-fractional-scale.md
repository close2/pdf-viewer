# 2180 — A CFF matrix keeps its fractional scale

Session 1672. Status: **accepted**, in `crates/pdf-font/src/cff.rs`. Context: ADR 2163 section 3,
ISO 32000-2 §9.9 and Adobe Technical Note 5176, Table 9.

The CFF reader normalizes the top matrix and returns an integer units-per-em value. A PDF font
does not require its reciprocal scale to be an integer. Taking that value rounded both outlines
and program advances before the archive validator compared them with the PDF's widths.

Read the original Top DICT through the dependency's dictionary reader, before its normalization.
Use its dynamic scale divided by the magnitude of its Y factor, matching the outline reader's
normalization axis, and retain that quotient as a floating-point value. The normalized outline
still carries the matrix's horizontal factor. The same units-per-em value normalizes outlines
and charstring advances; neither the validator nor the converter gets a separate correction.

[Technical Note 5176 Table 9](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5176.CFF.pdf#page=15)
defines the matrix operator and its default. With no matrix, the default remains 1000 units per
em. The test states a 649-unit advance under a matrix of 0.00049: the result is 0.31801 text-space
units. A 100-unit line is 0.049. A nonuniform matrix doubles the horizontal component and keeps
the vertical normalization, so the two dimensions cannot accidentally be conflated.

This changes the reading, not the tolerance in ISO 19005-2 section 6.2.11.5. ADR 2163's example
differs from its PDF width by 0.00101, which remains outside that clause's thousandth.

Found, not fixed: `type1.rs::units_per_em` also takes an integer from its reader. Whether a
fractional Type 1 matrix reaches the same rounding needs its own witness and rendering comparison;
this decision covers bare CFF programs.

The real-file cross-check is `doc/corpora/format-corpus/jhove-errors/PDF-HUL-133/wp315.pdf`,
font object 111 (`F4`). Its code 49 states 0.317 in the PDF and now reads 0.31801 from the
embedded program; code 50 states 0.58 and reads 0.58212. The archive diagnostic confirms that
the corrected program scale exposes the PDF/program disagreement instead of rounding it away.
