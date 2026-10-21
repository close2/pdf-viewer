# 2191 — The comment-history gates read C++ too

Status: **accepted**. Session 1677, 2026-10-21.

The Qt host is production source read by people, but the two history sweeps selected Rust,
C and headers and omitted `.cpp`. Two comments in `window.cpp` retained historical narratives
that no gate could see. Extending a source population must also extend the language reading:
C++ raw strings can contain quotes and comment markers without containing a comment.

Both gates now include `.cpp` and assert that a C++ implementation file was actually visited.
The numbered-round gate uses non-nesting C++ comments and skips raw strings through their
matching delimiter, preserving line numbers across embedded newlines. A planted fixture
includes quoted comment markers in raw strings, a prefixed raw string, real line comments
and a decorated block comment. The ordinal gate deliberately reads every line, as its existing
contract does for other languages.

Calibration before cleanup: the expanded ordinal gate fails on the two Qt comments. They now
state only why an editable combo or multiple-selection list is configured that way, retaining
the normative Table 233 text and the relevant ADR. No widget behavior changes.

The resulting focused tests pass. This is a population correction, not a broader relaxation
of what a comment may say or a new held allowance for existing debt.
