# 2218 — Hidden popup pairs need independent discard consent

Session 1692, archive. Status: accepted; implemented; scoped and canonical archive gates pass.

ISO 32000-2 §12.5.3 defines annotation visibility flags; §12.5.6.2 links a
markup annotation to its popup; §§12.5.6.4 and 12.5.6.14 define their initial
open state. ISO 19005-2/-4 §6.3.2 constrains annotation flags. The existing
typed as-shown plan decides each failing annotation independently. A parent's
discard answer cannot supply its companion's missing answer.

Replace the unconditional linked-removal refusal with a private proof. Both
members must already be selected for discard, have admissible hidden-at-rest
flags, occur once on the same page, and name each other through indirect
Popup/Parent references. Both Open values must be absent/null or false.
The parent must be a supported markup subtype; Sound is excluded by the held
popup semantics. Reject widgets, additional incoming Popup/Parent/IRT edges,
structural or action relationships, malformed links, and a popup appearance.
The parent's normal appearance is allowed. Unlinked policy is unchanged.

The graph includes conforming page annotations as well as failing rows, so a
second parent or reply cannot disappear from the proof's population. No new
configuration, public API or global loss permission is introduced. Q566 stays
unresolved and unrelated font-fitting refusals retain their meaning.

A corrected corpus premise matters: F28 means Print+NoZoom+NoRotate, not hidden.
The project parser confirms ZapfDingbats objects 43/44 have F30 and a reciprocal
closed pair; its flag refusal clears, but independent font coverage still stops
whole-document conversion. PDFBox OrphanPopups pairs have F30/F28 and retain the
named linked-pair refusal. No successful PDFBox conversion is claimed.

Synthetic PDF/A-2b and PDF/A-4 outputs pass veraPDF. Positive controls preserve
kept marks and appearance bytes, exact removal identities, and raster pixels.
Negative controls cover missing consent, visible companions even when discarded,
malformed/open pairs, duplicate/cross-page membership and incoming relationships.
The same positive test fails against fixed baseline 0eb4d278 at the former refusal.
Final gate receipts belong to history 1692 and scratchpad/r1692.
