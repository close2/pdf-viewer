Status: complete
Given: 2026-11-13, in conversation — transcribed by the round
Owes: restore ADR 2841's construction so the producer's /RO Form XObject is drawn over the emptied region strictly after physical removal of the region's content, read the authoring exclusion's "overlay" as /OverlayText (with /IC's fill kept excluded), and update redaction_projection_spec_audit.rs::a_redaction_ro_is_reported_while_removal_proceeds_without_composing_the_overlay to expect /RO drawn after removal (a transform round)

> Q842: agreed

Reading: adopts the recommendation — after `pdf_transform::redact` has physically removed and destroyed all text, path, and image content in the redaction region (`redaction_is_removal.rs`), draw the producer's `/RO` Form XObject over the empty region as §12.5.6.23 Table 195 specifies; read `CLAUDE.md`'s redaction overlay exclusion as `/OverlayText` (keeping `/IC` excluded), and update the audit test in `redaction_projection_spec_audit.rs` to assert that `/RO` is drawn after removal.
