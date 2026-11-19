# 3209 — A descriptor written inside its font is copied into the descriptor the substitution writes

Session 2127, an archive round in the `lane3` worktree. Status: **accepted** and built. Answers
`doc/pdfa-blocking-issues.md`'s R10 entry "a descriptor written inside a font that has no program"
(`SimFang-variant`, `XiaoBiaoSong`). Context: ADR 1209 (the shipped face for a font the file does not
embed), ADR 2162 (a font dictionary written inside its resource dictionary), ADR 2394 and A694 item 1
(photographing around a refused font); `crates/pdf-transform/src/archive/fonts.rs`
(`embed_pass`, `embed_at_descriptor`, `photographable`), `fonts/empty.rs`, `fonts/bystanders.rs`.
Clauses: ISO 32000-2 §9.6.2.1 Table 109, §7.3.10; ISO 19005-2 section 6.2.11.4.1.

## Context

Both documents show a simple `TrueType` font that embeds no program and writes its `/FontDescriptor`
as a direct dictionary. Table 109 makes that entry "( Required; ... shall be an indirect reference )",
so the file is at fault, and the substitution refused the font by a sentence of its own: a program
is embedded "by adding a key to the descriptor object", and there was no object. The refusal was
also kept from the photograph (`photographable`), as R10's to answer.

## Decision

The premise was wrong. `embed_at_descriptor` never adds a key to the producer's descriptor: it
copies the descriptor, writes the program into the copy, and writes the copy as a new object of its
own that the font's `/FontDescriptor` names by reference. A descriptor written inside the font is
copied the same way, so the route reads the descriptor resolved, whichever way the file wrote it,
and the output states the indirect reference Table 109 asks for. Every reading the route makes of
the descriptor (its `/Flags` for a font made symbolic, the proof of empty glyphs) reads the same
copy. What is left refused is a `/FontDescriptor` that is no dictionary at all
(`DESCRIPTOR_NOT_A_DICTIONARY`), and that refusal may be photographed around like any other.

## Consequences

Nothing a reader draws changes: the font was drawn from the face the window chose, and the face
embedded is that face (the machine's under `as-shown`, the shipped one under `as-if-printed` with
the loss `machine-face`, ADRs 2773 and 2774). Behind this refusal stood two more, which ADR 3210
answers: the font's name is no UTF-8 (section 6.1.8), and the symbolic font states
`/WinAnsiEncoding` (R11, section 6.2.11.6). Test:
`crates/pdf-transform/tests/archive/descriptor_inside_the_font.rs`.
