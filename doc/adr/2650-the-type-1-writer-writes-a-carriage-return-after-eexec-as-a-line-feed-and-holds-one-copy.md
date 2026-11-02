# 2650 — The Type 1 writer writes a carriage return after `eexec` as a line feed, bounds a charstring at 65 535 bytes, and holds one copy of what it writes

Status: implemented and measured (section 4). Session 1908 (ADR 2620 section 3's two open items).
Context: Adobe Type 1 Font Format section 7.2 (pages 63–64) and chapter 10 (page 83); `read-fonts` 0.44
`src/ps/type1.rs` `find_eexec_data` (lines 487–529); ADRs 2620, 2578.
Code: `crates/pdf-font/src/type1_glyphs_given.rs` (`Sections::header_written`, `MOST_CHARSTRING_BYTES`, `Written`,
`body_of`, `encrypt_in_place`, `section_with` and `read_back` over the one buffer) and its tests; `type1.rs` (the
re-export); `fuzz/fuzz_targets/type1_write.rs` (the header property).

## 1. Where the section starts

Section 7.2 lists, among the restrictions on `eexec`'s input, that "[t]he first ciphertext byte must not be an ASCII
white space character code (blank, tab, carriage return or line feed)", and that in binary form "every byte is
considered part of the ciphertext". Every white space byte after `eexec` therefore comes before the section, and a
carriage return there and a line feed there read the same to the standard. `read-fonts` follows FreeType's rule
instead: it skips a line feed, but a carriage return only where a line feed occurs somewhere at or after it in the
data. For a header ending `eexec` and a carriage return, where `read-fonts` starts the section depends on whether the
ciphertext happens to hold a byte 10 — and the writer re-encrypts the section, so its output may hold none where its
input held one. That is `type1_write`'s five findings of ADR 2620: the read-back of a program the writer wrote twice
refused it.

**Decision**: the writer copies the header byte for byte but for a carriage return in the white space after `eexec`,
which it writes as a line feed. The standard reads both the same, `read-fonts` reads a line feed whatever follows, and
the substitution keeps the header's length, so `/Length1` is unchanged. Refusing the shape by name was the other
choice; it would refuse a program the standard reads without question for a reader's leniency rule, and the
normalisation decides nothing a reader sees. The viewer's own `Program::parse` keeps `read-fonts`' rule for a program
read as stored; that is named in the round's record, not changed here.

The test is the finding `defcb9b3`, minimised by `cargo fuzz tmin` to 370 bytes and stated as its header and its
decrypted section: its ciphertext holds a line feed, the program written from the program written from it holds none.

## 2. The charstring bound

Chapter 10 states that "[i]ndividual tokens and charstrings may not exceed 65,535 characters in length".
`MOST_CHARSTRING_BYTES` is that figure, checked per given glyph with its `lenIV` leading bytes before anything is
written; a `.notdef` past it is refused with the figure in the refusal. The given glyphs therefore add at most
`MOST_GLYPHS_GIVEN` × 65 535 bytes, 16 MiB; `MOST_BYTES_WRITTEN` (64 MiB) stays the cap on the whole program.

## 3. One copy

ADR 2620's writer held each given glyph's body, the plaintext section, its encrypted copy, the program, and the
read-back's decrypted copy beside `read-fonts`' own. Now a given glyph is held as its name, advance and outline choice
(`Written`) and its body is rebuilt by `body_of` where it is written and where it is read back; the section is written
straight into the program and encrypted where it lies; the read-back's decrypted copy is dropped before `read-fonts`
parses. What stays is the program, the input's plaintext, and `read-fonts`' working set: measured alone at the bound
(section 4), about three and a half times the program written. The given glyphs add 16 MiB at most, so the writer
reaches 200 MiB only for a program already some 40 MiB long, where the cost is the input's own size and not anything
the writer amplifies; `MOST_BYTES_WRITTEN` still refuses past 64 MiB.

## 4. Measured

Peaks are `getrusage`'s `ru_maxrss` of the child, beside HEAD's release binary, on a machine at load 22–30.

- **The hostile probe** (ADR 2620's shape: a 250 933-byte file whose `.notdef` is 246 006 bytes and whose 255 shown
  codes reach it), `quorra-transform archive --to 2b`: HEAD converts it at a peak of 1 179.7 MiB in 7.66 s and, again,
  856.9 MiB in 11.25 s, writing a 62 947 541-byte file; here it is refused by name at 12.3 MiB in 0.02 s (exit 4, the
  charstring bound).
- **At the bound** (the same shape, a 64 998-byte `.notdef`, 64 998 + 4 bytes under the bound): HEAD 248.6 and
  248.7 MiB, 1.95 and 1.61 s; here 237.8 MiB, 1.77 s; both write 16 516 570 bytes. Of that, the writer's own share,
  measured as the bound test alone (256 copies of a 65 531-byte charstring, a 16.8 MB program) under the same probe, is
  70.5 MiB against 12.3 MiB for a test that writes nothing: about 58 MiB for 16.8 MB written. The rest is the
  conversion's handling of a 16.5 MB font stream after the writer has returned, not this ADR's.
- **Fuzz** `type1_write`, fork mode with 4 jobs and `-ignore_crashes=1`, 614 s from 76 seeds of `fuzz/seeds.sh`, the
  five findings and their minimised form: 2 560 616 executions, 1 736 edges, 8 417 features, 2 056 inputs, 0 out of
  memory, 0 timeouts, 0 crashes, peak 0.27 GiB. The five `eexec\r` findings replay clean; the sixth, a header opening a
  procedure it never closes (`c8f91bdb`), still fails the written-again property and is not this ADR's.
- **The plant**: `header_written` copying the white space unchanged turned the carriage-return test and all five
  findings red, each on the written program written again (`read-fonts` reading no section); restored, green.
- **Census**, without the pack, against HEAD's: no document moved at any line, and no refusal names the bound.
