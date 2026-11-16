# 2889 — A Private Use ActualText is removed, and a transfer function's page is photographed

Session 2036, an archive round in the audit worktree, on `doc/pdfa-blocking-issues.md` R03's second
and third finer remedies. Status: accepted and built. In `pdf-transform`: `archive/actual_text.rs`
(new), `Loss::ActualTextPrivateUse` (`actual-text-private-use`), `Rewrite::ActualTextRemoved`,
`Conversion::removed_actual_texts`; `fenced::transfer_pages` and the graphics states the reach walk now
enqueues; `doc/profiles/as-shown.toml` and `as-if-printed.toml`.
Tests: `crates/pdf-transform/tests/archive/finer_remedies.rs` (four).
Context: ADR 0816 (the fence), ADR 1964 (the transfer function a loss), ADRs 2394, 2516, 2847 (the
photograph and its gate), A694 item 1.
Clauses: ISO 19005-4 sections 6.2.5 and 6.2.10.8, ISO 19005-2 section 6.2.5; ISO 32000-2 §10.5,
§14.9.4.

## 1. The ActualText

Part 4's section 6.2.10.8 forbids a Private Use value in an `/ActualText`. ISO 32000-2 §14.9.4 makes
the entry optional and its value "a character substitution for the content", "equivalent to what a
person would see"; a Private Use value is that only under the producer's private agreement. Of the two
rewrites that meet part 4, taking the private characters out writes a substitution the producer did
not write (U+F0B7 alone becomes an empty string, which says the content reads as nothing), and taking
the entry off claims nothing and leaves the content read as its own text. The second is built, behind
a loss of its own the report names per object with the value as `U+XXXX`. Both shipped profiles
authorise it: neither a window nor a print shows an `/ActualText`. `state-every-loss` keeps `stop`,
since its statement page has no sentence for the loss yet. Only an entry an object states is removed;
one inline in a `BDC` operand is inside the producer's stream and stays the photograph's.

## 2. The transfer function

§10.5: "the PDF processor shall apply the transfer function after performing any needed conversions
between colour spaces". ADR 1964 drew the consequence that a print carries the function, so its
removal is a loss no shipped profile authorises. The same sentence answers `as-if-printed` (owner
answer A694: as if printed and scanned): the page this reader draws with the function applied is what
a scan of the print shows. So the row joins the fenced rows the photograph takes: the pages are those
whose resources reach a graphics state whose `/TR` is not `Identity`. Where every state names
`Identity`, ADR 1952's free removal answers the row and nothing is photographed; where an `Identity`
state is reached from another page, or an annotation's appearance reaches a state, or a place names
no object, the row keeps its refusal. Writing the function into the colours instead would edit the
producer's operands and samples, in a device space §10.5 places after colour conversion — the far
side of ADR 0816's fence. `as-shown` keeps ADR 2394's gate.

## Evidence

`smaskdim` (9 entries, U+F0B7) and `sakula` (6, U+F062 and U+F06D) convert at 4 under both profiles;
`issue6931_reduced` converts at 2b and 4 under `as-if-printed`, its logo red in the photograph as in the
window. veraPDF 1.30.3 calls every output compliant.
