# 3238 — A Table 349 value holding a character XML 1.0 excludes is answered by the rows' second step

Session 2134, an archive round in the `lane3` worktree. Status: **accepted** and built. Answers
`doc/pdfa-blocking-issues.md`'s R07 entry "a stream whose file's packet cannot name its removal"
(`poppler-742-0-fuzzed`). Context: ADR 1269 (the dictionary moves into the packet), ADR 2681 (a
carried value the packet cannot hold is removed and stated), ADR 2689 (a removal recorded in
`xmpMM:History`), ADR 3141; `crates/pdf-transform/src/archive/prepare/metadata.rs` (`xml_admits`,
`not_admitted`). Clauses: ISO 19005-4 sections 6.1.3 and 6.7.2.1; ISO 32000-2 §14.3.3, Table 349.

## Context

The R07 refusal said the document's packet "will not take that entry". Read at the refusal, the
packet was this conversion's own fresh one, and the step that broke it came first: section 6.1.3
moves `/Title` into `dc:title`, and the fuzzed `/Title` is UTF-16BE text holding U+0000. XML 1.0's
`Char` production admits no U+0000, even as a reference, so the supplement wrote a packet that is
not well-formed, the history entry could not parse it, and the whole metadata step refused. ADR
2681 already answers this for a key Table 349 does not define (`xml_carries`); a key it defines
was written unchecked.

## Decision

A Table 349 value whose text holds a character the `Char` production excludes (a C0 control other
than tab, line feed and carriage return, U+FFFE, U+FFFF) is answered as ADR 2681 answers a carried
value: `on-failure = "append-statement"` removes it and states the key, type and length on the
appended page; `discard`, as the second step or as `unreadable` — both shipped profiles — drops it
and the report names the key; a profile naming neither keeps a refusal by name. The predicate is
narrower than `xml_carries`: a carriage return in a Table 349 value has always been written, and
whether that reading changes is not this decision's.

## Result

`poppler-742-0-fuzzed` converts at part 4 under both shipped profiles (2b already converted);
veraPDF 1.30.3 passes all four outputs; page 1 at 150 dpi is pixel-identical to the source's.
