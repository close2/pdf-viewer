# 3519 — What R16's last document and the 6.1.13 refusals at 2b are, each by its value

Session 2204. Status: **accepted** (a reading; nothing built). Context: ADRs 0816, 2638, 2858, 3393,
3463; ISO 19005-2 section 6.1.13 (paraphrased); ISO 32000-2 §7.2.3, §7.3.3, §8.5.2.1.
Evidence: a lexical scan, by §7.2.3's and §7.3's rules, of every content stream of the thirteen
files once `qpdf --qdf --decode-level=all --normalize-content=n` decoded them; its findings are the
figures below.

## R16 — `poppler-395-0-fuzzed`

Under `as-shown` at 2b and 4 it stops on one sentence: the `FreeText` whose `/DA` names `/Rufscript`,
a font the form's `/DR` does not define, is set in ADR 0112's stand-in, the loss `stand-in-face`,
which `as-shown` does not take (ADR 2858 decision 3: embedding a face for the stand-in is
`machine-face` and `layout-from-this-viewer`, which `as-shown` refuses, and the census that granted
it wrote outputs failing the font rows). Under `state-every-loss` the signature it carries stops it
first (`signatures/digest-covers-the-whole-file`). `as-if-printed` converts it at both parts. The
refusal stands by name; it is the profile's answer, not a missing build.

## 6.1.13 at 2b — the thirteen `corpora` documents

Each converts at part 4, which states no such limit; at 2b `implementation-limits/values-written-in-
content-streams` refuses each as the fence words it (ADR 3393).

- **`475121`**: forty integers of 34 digits (about 2.4e33), every one an operand of `m` or `l` in
  page 1's contents. §7.3.3 makes them integers past part 2's integer bound and far inside its real
  bound, and §8.5.2.1's path operators take any number. **A remedy that preserves what is drawn
  exists**: the same value written with a period is a real within the limit, drawn identically by
  every reader, since the number is the same number. It edits content-stream bytes, so it is a row
  inside the fence of its own (ADR 0816), with the lexer's out-of-range record (ADR 3393) as its
  input; **not built this round**, and owed next in R06b.
- **The other twelve** (`020747`, `020806`, `027613`, `033689`, `040669`, `150277`, `219789`, `268346`,
  `427669`, `477139`, `492484`, `507676`): every value past a limit is damage, not a producer's
  number — delimiters lost so digits fuse (`43.5255.752`), garbage read as strings and names longer
  than the bound, runaway `(`. In `040669` and `427669` the scan finds no value past a limit at all,
  only runs §7.3.3 spells as no number, so the row's finding there is how this tree's survey reads a
  damaged run, to be read against the survey before anything is built. For the rest the candidate is
  ADR 2638's `content-as-drawn` (the number this reader draws, written in the run's place), which
  needs per stream the proof that the drawn number is itself within the limit and the page unchanged
  in pixels. Until then **the refusal stands**, and part 4 is the target that holds them.
