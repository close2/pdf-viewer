# 1293 — The parsers without a fuzz target fuzzed, and what the artefacts had held

Robustness slot of batch forty-five: principle 3's fuzzing. ADRs 1423, 1424. No question.

**Stock-take.** 18 targets; `ccitt` had no `doc/verify.md` line, so `tools/fuzz.sh --list` exited 1.
No target reached `pdf_font::shaping`, the JBIG2/JPX filters (`hayro-*` inside `pdf-sandbox`),
`pdf_model::jpeg2000`, the XFDF reader, Annex F's reader and writer, or `composite_cycle`. The fuzz
workspace did not build: its gitignored lock held `hybrid-array` 0.4.13 against `brainpool_p512`.

**Toolchain.** nightly 1.99 + cargo-fuzz 0.13.2, `-s none` (ASan's shadow reservation dies under
`tools/bounded.sh`), each target behind the lock, scratch corpus first, `-rss_limit_mb=2048`.

**New targets** `shaping`, `jbig2`, `jpx`, `xfdf`, `linearize`; `sfnt` asks `composite_cycle`.
Seeds: `fuzz/seeds.sh`, `fuzz/seed_shaping.py`, `fuzz/seed_forms_data.py`, `image_codec_seeds`.

**Runs** (10 min each; execs, INITED→DONE edges, artefacts):
shaping 2.07M 827→1282 0 · jbig2 4069 2957→– 1 timeout, then fork mode 8397 →3818, 6 more ·
jpx 105k 1973→2861 0 · xfdf 51.4M 3064→3565 0 · linearize 297k 7269→7686 0 · sfnt 13.7M 383→433 0 ·
ccitt 140M 223→223 0 · page 1292, 1 oom (below); at 4096 MB 36.2k 30208→30453 0 · document 684k 3983→4249 0 · serialize 429k
4143→4393 0 · object 44M 462→553 0 · lexer 136M 226→227 0 · cmap 23.9M 368→836 0 · crypt 8.6M
837→889 0 · variable_text 58k 5985→6084 0 · forms_data 168M 270→275 0, re-seeded 11.9M 1347→2587 0 · xmp 29.5M 1584→1800 0 ·
fragment 25.3M 366→427 0 · cms 105M 352→415 0 · confined_wire 132M 3670→5987 0 · display_list
274M 105→1344 0, re-seeded 18.3M 1009→1648 0 · revocation 14.4M 437→1080 0 · x509 6.3M 1695→1875 0.

**Findings.** (1) `fuzz/artifacts/page/timeout-6af40bfb…`, six weeks unread: a string past
`CEILING` was stepped over to white space, so its words were read as content and each `(` re-lexed
a mebibyte — 59 s → 49 ms; `drop_token` now follows §7.3.4.2/§7.3.4.3 (ADR 1424 §1,
`content_window.rs`). (2) `composite_cycle` scanned the path per reference: 34.5 s → linear
(§2, `truetype.rs`). (3) seven JBIG2 inputs `hayro-jbig2` decodes for minutes; the budget is the
worker's 30 s deadline, now tested (§3, `confinement.rs`). (4) `page`'s oom is
`ContentStreamCycleType3insideType3.pdf`'s shape at `MAX_OPERATIONS`, 1.8 GB, `verify.md`'s 4096
limit; not a new defect. (5) `forms_data`'s corpus held no FDF file; `display_list`'s is stale.

Left: the main checkout's `fuzz/artifacts/` still holds the four read artefacts and two corpora
to regenerate (owner's disk). Proposed habit: read `INITED` before a campaign.
