# 2998 — A threshold halftone carrying a transfer function is not replaced under a screen authorisation

Session 2078. Status: **implemented.** Amends ADR 2034. Context: review 2038 (findings B77, T168). Clauses:
ISO 32000-2 §10.5, §10.6.5.2 to §10.6.5.5 (Tables 128 to 131, `TransferFunction`); ISO 19005-2 and ISO 19005-4
section 6.2.5; ISO 19005-4 erratum 314.

ADR 2034 replaces a halftone of a type other than 1 or 5 by writing its graphics states' `/HT` as `Default`, under
`Loss::Halftone`, on the argument that nothing a screen shows moves. That holds for the screen and not for a
`TransferFunction` the halftone carries: Tables 128 to 131 make it override "the current transfer function in the
graphics state for the same component", and §10.5 applies it before a colour reaches any device. Under part 2 the
transfer requirement's own rewrite removes the function first, under its own authorisation (`Loss::TransferFunction`),
so the two losses are authorised separately (T168's control). Under part 4 erratum 314 withdraws that requirement,
nothing else removed the function, and the screen authorisation carried a colour change it does not name.

Now `halftones::defaults` refuses, by name, to replace a halftone carrying a transfer function other than
`Identity` unless the transfer requirement names the graphics state that sets it. Part 4 cannot keep the function in
the graphics state either: section 6.2.5 forbids `TR` and a `TR2` other than `Default`. A remedy that keeps it — a
type 1 halftone carrying the producer's function — would choose screen parameters nothing in the file states, and
is left for a decision of its own.

Tests: `pdf-transform` `archive` `threshold_screen_authorisation_preserves_active_transfer_in_pdf_a_four`,
`threshold_transfer_under_pdf_a_two_requires_separate_loss_authorisation`.
