# 2227 — A protocol move proves tags, errors and bodies unchanged

Session 1696, docs. Status: implemented; scoped gates pass; final conformance pending.

The protocol split starts from a hashed source snapshot. A syn parser identifies
exactly six function items, including attributes and documentation; source spans
preserve their original body bytes, including ordinary comments. The proof
compares complete function token streams after normalising only the declared
pub(super) visibility and two named trailing signature commas added by rustfmt
when that visibility makes read_script_answer and write_published wrap.
No body token or punctuation is normalised away.

The four dialogue functions preserve 54 literals including documentation; the
two published-policy functions preserve 29. The remaining root's tokens match
the original with only those function removals and explicit private module/import
scaffolding. Consequently original tests, routers, tags and error branches stay
in the comparison, rather than being regenerated from the candidate codec.

Three scratch controls independently change a tag, change an error string, and
omit a helper. Each fails the proof with exit 101. The positive proof passes.
These lexical controls complement existing roundtrip/refusal tests, compilation,
all-target clippy and private rustdoc; they do not replace those gates.
Proof sources, snapshot and receipts are under scratchpad/r1696.
