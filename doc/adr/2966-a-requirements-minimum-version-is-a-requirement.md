# 2966 — A requirement's minimum version is part of the requirement

Session 2073. Status: **implemented.** Context: review 2038 (findings B38, D27), ADRs 1167, 1220. Clauses: ISO
32000-2 §12.11.1 (Table 273), §12.11.4, §12.11.6.

Table 273's `/V` is "[t]he minimum version level of support needed to satisfy the requirement", and "the value
shall represent the PDF version" unless Table 275 says otherwise, which it does for `U3D` and `PRC` alone.
`requirements::unmet` judged the feature and kept `/V` only as a string, so a `/V /2.1` was met by this PDF 2.0
reader and cost no penalty. `Requirement::unmet` now judges the feature and then the version: a PDF version later
than `pdf_syntax::Version::SUPPORTED` is unmet, is named in the report and counts in `penalty_total`, which is what
`Operation::Process` decides from. A `/V` that is not §12.11.4's dotted number (the extensions-dictionary form,
kept as its prefix) states nothing comparable and the feature alone decides; `U3D` and `PRC` are unmet as features.

The reader's level for processing (ADR 1167) decides what is done with the total. It never decided which
requirements count, and `notes.rs` now says so beside the report.
