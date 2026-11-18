# 2994 — The transparency survey answers only for what it saw, and only for what Annex Q counts

Session 2078. Status: **implemented.** Amends ADR 1461. Context: review 2038 (findings B35, B37, D24, T50).
Clauses: ISO 32000-2 Annex Q.2 and Q.3; Table 166 (`/CA`); ISO 19005-2 and ISO 19005-4 section 6.2.9 (the
transparency rows `graphics/a-transparent-page-has-a-blending-space` and
`graphics/a-transparent-page-has-a-blending-space-or-an-output-intent`).

Three readings of the `Survey` walk change.

1. **A walk a bound stopped does not certify a page.** `Survey` stops at its depth, stream and token bounds and
   says so (`stopped_short`). The two transparency rows asked only `page_is_transparent`, so a page whose
   transparent content lay past a bound was reported Met. Now a stopped walk returns each page it did not see as
   transparent to the rows as *unknown*, and a page with no blending space and no output intent is recorded as a
   finding that says the walk stopped. The bounds stay; the verdict stops claiming what they hid.
2. **An annotation's `/CA` counts only where it has no appearance stream.** Table 166: "The specified value shall
   not be used if the annotation has an appearance stream (12.5.5, "Appearance streams"); in that case, the
   appearance stream shall specify any transparency." The streams are walked as forms (Q.2's last paragraph), so
   their own states decide. `/BM` is unchanged: Errata Collection 3 Issue #51 makes it transparency with an
   appearance as well.
3. **A soft mask's group does not itself make a page transparent.** Q.2 counts "each graphical element to be
   rendered on the page"; the mask's `/G` is evaluated for the elements painted under the mask and is not one. Each
   of those is transparent by Q.2's first condition (`/SMask` a dictionary), which the walk already records at the
   paint, so the group's own marks are redundant where something is painted and wrong where nothing is. The group
   is still walked, because ISO 19005's colour rules reach the colours it paints (ADR 2263).

Tests: `pdf-archive` `incomplete_transparency_spec_audit` (four), `transparency_detection_spec_audit`
`annotation_ca_does_not_override_an_explicit_opaque_appearance`.
