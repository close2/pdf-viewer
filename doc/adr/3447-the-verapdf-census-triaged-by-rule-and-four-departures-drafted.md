# 3447 — The veraPDF census triaged by rule, and four departures drafted upstream

Session 2186. Status: **accepted**. Context: round 2181's census (63 failing outputs of 19
documents), `doc/pdfa-blocking-issues.md` section 1, `doc/upstream/verapdf-1.30.3-departures.md`,
ADR 2652.

Every document is placed in one of three: ours (ADRs 3445, 3446), veraPDF's, or the document's.
A departure is drafted upstream only where the clause it rests on is held in `doc/` and the reading
is plainly veraPDF's:

1. **6.2.11.3.1 test 1** compares `CIDFontSupplement <= CMapSupplement`; ISO 19005-2 6.2.11.3.1 and
   ISO 19005-4 6.2.10.3.1 require the CIDFont's Supplement to be greater than or equal to the
   CMap's (`noembed-eucjp`, `noembed-jis7`, `issue11555`). The profile text is the evidence.
2. **`re%`** read as one operator, where §7.2.3 makes `%` a delimiter (`calgray`, `calrgb`).
3. **A rectangle of indirect numbers** read as zero-sized, where §7.3.10 admits an indirect object
   as an array element (`issue7872`).
4. **A CFF encoding supplement** read other than as Technical Note #5176 section 12's SID
   (`issue3566`; code 174 is `ff`, 599.53 against `/Widths` 599).

Recorded as veraPDF's and not drafted, because the specification is not held here or the cause
inside veraPDF was not found: a Type 1 program's `lenIV -1` (`issue7769`; veraPDF's widths −20
and 0 are what decrypting unencrypted charstrings gives), a (3,1) `cmap` subtable of format 12
(`issue14881`; every code read as the 750-unit `.notdef`), a `/CharSet` naming exactly the 41
glyphs the program's `CharStrings` hold (`facct2025-final168`), and an inline image whose samples
hold `EI` (`029016`). No remedy restates a valid program to suit another reader (principle 5).
