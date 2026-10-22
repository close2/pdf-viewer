# 2210 — Annotation flags are answered per object

Session 1688, archive. Status: accepted; implemented and scoped gates passed.

ISO 19005-2 and ISO 19005-4 §6.3.2 require conforming annotation flags.
ISO 32000-2 §12.5.3 Table 167 distinguishes screen visibility from printing.
A visible annotation without Print has marks the as-shown profile must retain;
an annotation with Hidden or NoView has no marks on the page at rest.
One site-wide answer cannot preserve the former and discard the latter in a
mixed document. The former as-shown answer therefore stopped both.

The profile now names two shapes at annotations/printable-and-visible:
hidden-at-rest answers Hidden or NoView with Invisible and ToggleNoView clear;
visible-nonprinting answers missing Print with all four forbidden flags clear.
It discards the first and preserves the second by setting Print in place.
Negative flags and the Invisible or ToggleNoView combinations are not classified.
Invisible has subtype-dependent semantics; ToggleNoView describes interaction.
Neither is an instruction to guess the page's state. Malformed noninteger flags
also refuse under this policy. Absent or null F retains Table 166's default zero
and the separate flags-entry-present answer; no implicit permission is added.

The existing shape parser previously selected one answered row for a whole site.
AnnotationFlagAnswers instead retains the two answers through Configuration,
Contribution and ArchivePlan until the preparation phase sees actual objects.
No shape rows means the former site-wide behavior, including its authorisations.
A partial shape policy inherits its other answer from the unqualified row or Stop.
Unsupported remedies, options and fallbacks fail configuration validation.
An object outside the shapes refuses even alongside a global removal authorisation.
The new optional Rust plan field is a source API addition; all workspace callers
are compiled. It does not change the native ABI or configuration of existing profiles.

Focused controls cover both targets, mixed objects, table order, absent policy,
partial answers, malformed values and misspelled shapes. The shipped profile is
transported through the same Contribution used by command-line and host callers.
