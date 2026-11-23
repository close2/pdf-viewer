# 3393 — An integer too large for 64 bits is still an integer to the validator

Session 2173, an archive round in the `lane3` worktree. Status: **accepted** and built. Context:
ADR 3376 item 1; ISO 32000-2 §7.3.3; ISO 19005-2 section 6.1.13; `pdf_syntax::Lexer`,
`pdf_model::content::reader` (`Runs::AsWritten`), `pdf_archive::survey`.

## Context

`475121`'s content streams write 34-digit integers. §7.3.3 makes a run of digits with an optional
sign an integer whatever its length; `Token::Integer` holds an `i64`, so the lexer handed the run
over as the nearest `Token::Real`, the survey noted a real of about 1e33, inside part 2's real
bound, and `implementation-limits/values-written-in-content-streams` passed a file veraPDF fails
at 6.1.13 (40 `CosInteger`s per output).

## Decision

The token stays a real: a reader drawing the page needs a number, and every drawing caller keeps
what it had. Beside it the lexer now records the run where it spelled an integer `i64` cannot
hold (`Lexer::integer_out_of_range`, set only on `i64::from_str`'s overflow error, so a run with a
period or an exponent never sets it). The content reader's `Runs::AsWritten` mode — the
validator's, and only the validator's — hands such a run over whole, as it already hands a
salvaged run, and the survey reads its digits back as an `i128` (saturated at its bounds for a
run past 38 digits, which every part's limit is far inside). `ContentLiterals`' integer extremes
are `i128` for that reason.

No `Token` variant was added: every match on `Token` across the workspace would have had to learn
a case only the validator reads, and the side record is the shape `Lexer::salvaged` already
established for "what the token does not say".

## Consequences

`475121` is refused at 2b by the row, as the fence (`NotThisTarget`) words it: the value is the
producer's content. At part 4, which states no such integer bound in this tree's reading, it
converts and veraPDF passes it. File-level objects keep the old reading (`Object::Integer` is an
`i64` too): `implementation-limits/integer-values` would read the same record through the
parser to answer an object that states one, which no census document has shown and which is left
until one does.
