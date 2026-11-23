# 3494 — The ASCII data restatement is named in `xmpMM:History`

Session 2198. Status: **accepted**; built. Context: ADR 3473 item 3 (the open item), ADR 3462,
ADR 2689 (`undecodable-stream`, the construction followed). Clauses: ISO 19005-2 section 6.6.6,
ISO 19005-4 section 6.7.5.

## Decision

The streams `ascii_data::restate` writes as the bytes decoded are prepared beside the streams no
reader decodes, before the packet, and their report sentences, joined, are one `converted` entry
of `xmpMM:History` after `undecodable-stream`'s. As for that rewrite, the entry is a condition: a
restatement the packet cannot record is not written (`ascii_data::NO_PLACE_TO_RECORD`), and the
packet is written whenever the restatement is, as for a removed transfer function (ADR 1964). The
preparation only moved earlier; it reads the document and the validator's report and nothing the
packet's preparation writes, so no other output changes (the census's kept outputs, re-diffed).

Test: `each_stream_restated_is_named_in_the_files_own_history` in
`crates/pdf-transform/tests/archive/ascii_data_as_decoded.rs`.
