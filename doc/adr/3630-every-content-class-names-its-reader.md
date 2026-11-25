# 3630 — Every class of repair logged inside content names who reads it, and a gate holds it

Session 2232. Status: **accepted and built**. Context: ADR 3614, ADR 3629; the trap that a class
the interpreter alone logs reaches the repair log after the converter has decided.

## Decision

1. **`pdf_model::content::grammar::COVERAGE`** lists every class `pdf-model`'s content code and
   `pdf-syntax`'s lexer log inside content, each with its `Reading`: `Grammar` (the token walk,
   with the walk's resolution of a `cs` space or a `BDC` property list), `Reader` (the content
   reader, its inline-object reader, the inline-image scan or the lexer — code both walks run), or
   `Interpreter(reason)`.
2. **Left to the interpreter, with the reason in the table**: the five classes of a graphics state
   parameter dictionary (`graphics-state-entry-of-the-wrong-type`, `transfer-function-unreadable`,
   `alpha-constant-out-of-range`, `halftone-unreadable`, `halftone-transfer-function-unreadable`)
   and the three of a form `XObject`'s dictionary (`form-matrix-malformed`, `form-bbox-unusable`,
   `form-resources-not-a-dictionary`): each is a fact about the object a name resolves to, read
   where `gs` applies it or `Do` invokes it, and no token states it.
3. **`tools/conformance/tests/content_classes.rs`** parses the sources: every class logged in
   `crates/pdf-model/src/content/`, `inline_image.rs` or the lexer is in the table and every entry
   is logged; a `Grammar` class is answered in `grammar.rs`, a `Reader` class in the reader or the
   lexer, an `Interpreter` reason is a non-empty constant; and the survey still calls what the
   table credits it with. A new class cannot reopen the trap unannounced.

## Consequences

The eight interpreter-only classes can still appear after the fence. They are resource readings,
not tokens; reading them in the survey would mean applying each `gs` and `Do` dictionary, which is
the next step if a document shows one late.
