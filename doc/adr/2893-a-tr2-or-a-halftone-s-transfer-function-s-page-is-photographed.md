# 2893 — A `/TR2` or a halftone's transfer function's page is photographed

Session 2038, an archive round in the audit worktree, on ADR 1964's "Owed" lines: `/TR2` other than `Default`
and a halftone's `TransferFunction`. Status: accepted and built. In `pdf-transform`: `archive/fenced.rs`
(`SECOND_TRANSFER`, `HALFTONE_TRANSFER`, `transfer_function_of`, `state_applies`, `halftone_applies`, the reach
walk following a state's `HT` and a type 5 halftone's components); `doc/profiles/as-if-printed.toml` and
`as-shown.toml` (comments). Tests: `tests/archive/finer_remedies.rs` (three).
Context: ADRs 0816, 1964, 1978 section 6, 2007, 2394, 2516, 2889; A694 item 1.
Clauses: ISO 32000-2 §8.4.5 Table 57, §10.5, §10.6.5.6; ISO 19005-2 section 6.2.5; ISO 19005-4 section 6.2.5
(whose halftone provision `pdf_archive::errata` issue #314 withdraws).

## Decision

ADR 2889's argument holds word for word. Table 57 makes `TR2` "\[s]ame as TR except that the value may also be
the name Default", and §10.5's second bullet has a halftone's function "override the corresponding one specified
by the current transfer function parameter in the graphics state"; §10.5 has every processor apply either. So a
print carries them, and under `as-if-printed` (A694: as if printed and scanned) the page this reader draws with
the function applied is the answer. The two rows join `fenced::ROWS`, and their pages are found as the `/TR`
row's: the pages whose resources reach an object stating a function other than the name `Identity`. For the
halftone row that object is the finding's holder — the graphics state whose `HT` it is, or the type 5 halftone
whose component it is — so the walk now follows a state's `HT` and a type 5 halftone's entries. A component that
omits the function §10.6.5.6 requires is answered the same way: the photograph is what the window draws, and it
claims nothing about the function the file does not state.

Where every finding is `Identity`, ADR 1978 section 6 and ADR 2007's statement answers the row and nothing is
photographed. An `Identity` reached from a page not photographed, an object an annotation's appearance reaches,
or a finding naming no object keeps the refusal. `as-shown` keeps ADR 2394's gate. Writing the function into the
colours stays behind ADR 0816's fence (ADR 1964).

## Left

`stated_where_free` reads `TransferFunctions::costs_nothing` across the three rows, so an `Identity` `TR2` or
halftone function in a document whose `/TR` is photographed elsewhere is refused rather than stated. No corpus
document holds either row; the fixtures are written from the clauses.
