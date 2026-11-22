# 3376 — Why this validator passed the veraPDF failures this round left standing

Session 2168. Status: accepted (a reading; each item names the work it needs). Context: round
2164's veraPDF triage (`doc/pdfa-blocking-issues.md` section 1); ISO 19005-2 sections 6.1.13,
6.2.2, 6.2.11.2, 6.6.2.3.1; ISO 32000-2 §7.3.3, §7.3.5; ADR 1021 (`pdf_archive::reach`).

The brief asked, for each veraPDF failure that is ours, why this tree's validator did not catch it.
ADRs 3374 and 3375 fixed three readings; these are the ones diagnosed and not yet fixed.

1. **`475121`, 34-digit integers in a content stream (6.1.13).** The row exists
   (`implementation-limits/values-written-in-content-streams`), but `pdf_syntax::Lexer` reads a digit
   run that overflows `i64` as `Token::Real`, so the survey notes a real of about 1e33, inside the
   real limit. §7.3.3 makes a run without a period an integer whatever its size. The fix is in
   `pdf-syntax` or `pdf-model`'s content reader (the token has to say it was spelled as an integer),
   then the fence's answer: the value is inside the producer's content stream.
2. **`PDFBOX-4532-reduced`, undescribed `illustrator` and `xmpTPg` properties (6.6.2.3.1).** The row
   finds all five (a probe of `extension_schemas_embedded` on the output reports five places), and
   the report withdraws them: the packet is the `/Metadata` of an object nothing reaches (the
   output's `<< /Metadata 11 0 R >>`), and `pdf_archive::reach`'s exemption narrows findings at
   unreached objects. veraPDF checks every indirect object. Section 6.2.2's exemption is about a
   named resource no content stream uses, which an orphan dictionary is not; either the exemption
   is narrowed to resources, or the writer stops carrying objects nothing reaches. Q-worthy only if
   both are refused.
3. **`issue1350`, `/BaseFont` written as a string (6.2.11.2).** No row judges it:
   `fonts/font-programs-conform-to-their-own-specifications` is `Unchecked`. A row asking Table 109's
   "name" is the first half; the value is a UTF-16 string with a NUL between letters, which no name
   may hold (§7.3.5), so the second half is ADR 2846's name taken from the embedded program.
4. **`issue7901`, `bug920426`, `T02-04-01_005`, `issue17679_2`, `bug1068432`, `bug1308536`,
   `extgstate`, `bug866395`**: not reached this round, in the order `doc/pdfa-blocking-issues.md`
   section 1 lists them.
