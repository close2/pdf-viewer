# 2203 — A move keeps literal tokens as well as statements

Status: accepted verification rule. Session 1684. Context: ADR 2202; todo 74.

Whitespace removal is not a sufficient proof of a Rust method move: whitespace
inside a string is data. Path replacement over a whole source file can likewise
rewrite a literal while leaving signatures and line counts plausible.

For this split the comparison retains complete normal, byte, character and raw
literal tokens, including escaped newlines. It checks all 58 named methods and
all root declarations outside the moved implementation. Only known module-depth
visibility/path changes and optional trailing argument commas are normalized.
A changed character inside a literal fails the comparison. An independent check
compares the original literal sequence directly, without path normalization.

The first scratch lexer mishandled a backslash-newline string and its method-boundary
check failed before compilation. Correcting it and re-extracting from the untouched
baseline snapshot restored all 58 streams. This is why the comparator is calibrated
and has a structural completeness check, rather than merely counting moved methods.
Scratch evidence is `scratchpad/r1684/`: original snapshot, split script, method
manifest and verifier; the integration report retains the result before scratch closes.

Token identity does not prove imports, privacy or documentation links resolve.
Warnings-denied checks, tests and rustdoc must establish those separately. No runtime
bug repair belongs in a move; a defect found while reading gets its own future contract.
