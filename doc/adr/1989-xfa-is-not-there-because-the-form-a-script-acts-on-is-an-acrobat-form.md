# 1989 — `xfa` is not there, because the form a script acts on here is an Acrobat form

Session 1576, slot 6 of batch ninety-one, a script round; `doc/todo/70` item 5's residue. Status:
**accepted** and **built** in `crates/pdf-script/src/surface.rs`. Context: RFC 0008 §4.3 (Tier 2,
and how an excluded call fails), ADR 1957 §2 (a refused property throws on read), ADR 1975 §2 (the
eight property rows read against the reference), `CLAUDE.md`'s XFA exclusion. Supersedes RFC 0008
§4.3's row `XFA`, `xfa`, `dynamicXFAForm` for `xfa` alone. Clauses: §K.1, §K.2. No ledger row moves.

## 1. What the reference says

The *JavaScript for Acrobat API Reference*, at RFC 0008's pinned commit
(`adobe/dc-acrobat-sdk-docs` `ab3b42a7…`, `JS_API_AcroJS/XFA.htm`, 9 108 bytes, SHA-256
`eccaeca0…6ceddb`), gives one example on its XFA page, and it is a test for which kind of form a
document is: the comment "The xfa object is undefined in an Acrobat form", then
`if ( typeof xfa == "object" )` for an XML form, `else` "This is an Acrobat form." Here that read
threw `NotAllowedError`, so the reference's own test could not be asked.

## 2. The decision

**`xfa` is neither refused nor carried: a script's read finds the name not there**, and
`surface::ABSENT` lists it with this reason so that no later reading restores the refusal. `XFA`
and `this.dynamicXFAForm` stay refused as RFC 0008 has them.

The argument is the reference's sentence and Annex K's two. This processor takes §K.1's permission
not to implement XFA, and what it shows and fills is the document's AcroForm, which §K.2 requires,
in a hybrid file, to be consistent with the XFA resource. So the document a script here acts on is
an Acrobat form, and the reference's answer for an Acrobat form is that `xfa` is undefined. A
refusal tells the script it stands in an XML form whose object it may not reach — the one thing not
true of it here — and sends the reference's own branch for this case unasked.

RFC 0008 §4.3 says an excluded call fails "never as `undefined` and never in silence". `xfa` is
not an excluded call failing: it is a name the surface the reference describes does not have, which
is how `surface`'s module comment says the surface ends. Nothing is silent either. A document that
carries an XFA resource, or states `/NeedsRendering`, is said once where the viewer opens it
(`viewer_core::notes::xfa_form`, ADR 1932), whatever its scripts do; and a script that *uses* the
name rather than asking about it meets the `ReferenceError` an Acrobat form would give it, which the
outcome reports as any uncaught throw.

## 3. What is held

`tests/fixtures.rs`: every `ABSENT` name answers `typeof` with `undefined`, a use throws
`ReferenceError`, and no refusal is recorded — planted back into `EXCLUDED`, the test fails. The
reference's example, run through the code: verbatim it stops at its first line, which reads
`this.documentFileName` (`REFUSED`, ADR 1724); from its second statement it prints " This is an
Acrobat form."

**Gates.** `doc/history/1576` gives the tests and both script corpus walks.
