# 1559 — A seed corpus says when it is stale, and `jpeg_bands` is seeded by frame shape

Session 1362. Status: **accepted**. Context: trap 107; ADRs 0742, 0747, 1439, 1433, 1481; round
1355's record; `fuzz/seeds.sh`, `fuzz/seed_streams.py`, `doc/verify.md`'s fuzz lines,
`tools/conformance/tests/fuzz_workspace.rs`; `doc/questions/Q227`.

## 1. `fuzz/seeds.sh check <target>...`

A corpus on disk is a fact about one checkout, and it goes stale with nothing failing: round 1355
spent two twenty-minute campaigns on `forms_data` and `display_list` corpora whose coverage before
a single mutation was a fifth and a ninth of what fresh seeds gave. `check` makes that comparison a
command. It runs the target's own `case` arm into a scratch directory beside the build output,
then asks libFuzzer for `INITED cov` twice, a `-runs=0` pass over the disk corpus and one over the
fresh seeds, each under the memory, timeout and length limits of the target's `doc/verify.md` line
and without its run length or fork count (a fork-mode parent prints no `INITED`).

- **Stale** is the fresh figure leading the disk figure by more than 2 per cent of the fresh
  figure. A fuzzed corpus is expected to *lead*, since it holds what campaigns found; it trails
  only when it lacks what the seeders now produce. Repeated passes over one corpus print one figure
  for a single-threaded target (three passes of `forms_data`: 2072 each time); a target that
  iterates a randomly seeded `HashMap` or splits work over threads can move a few edges, and the
  gaps trap 107 is about are several times, not per cent.
- **The owner's corpus is read and never written.** libFuzzer keeps what it finds in the first
  directory it is given, so each pass names an empty scratch directory first and the corpus
  second, and sends artefacts to scratch. The binary is run by its path after `cargo fuzz build`,
  because `cargo fuzz run` creates `fuzz/artifacts/<target>` in the main checkout whatever
  prefix it is given. Then `check` prints what a re-seed would add (the fresh
  seeds whose bytes the disk corpus lacks) and where the link resolves, and removes nothing. The
  re-seed is a separate command, run on purpose.
- **Not judged** is printed when the fresh seeds give no figure under the line's limits. That says
  something about the seeder's population, not about the corpus (section 3).

The logic sits in `fuzz/seeds.sh`, so a `tools/state.sh` section can call it. Like every recipe
it is a census, so it runs behind the lock.

## 2. `jpeg_bands` takes one frame per shape

`seeds.sh jpeg_bands` wrote every `DCTDecode` stream up to 64 KiB, which came to 192 419 seeds and
2.3 GB written into the main checkout through the `fuzz/corpus` link. What decides which branch a
band plan takes (ADRs 1433, 1481) is the frame's shape, not its picture: the start-of-frame marker,
precision, component count and each component's sampling factors, the restart interval and whether
`DNL` states the height. `seed_streams.py`'s `frame_shape` reads those from the marker segments,
and the smallest stream of each shape is written. A stream whose markers do not parse is a shape
of its own, so the decoder's refusals keep a seed.

## 3. Measured

`check` was run behind the lock on 2026-10-05 and 2026-10-06 over 16 of the 29 targets:

| target | disk seeds, `INITED cov` | fresh seeds, `INITED cov` | verdict | wall |
|---|---|---|---|---|
| `forms_data` | 1344, 528 | 21, 2072 | **stale** | 5 s |
| `display_list` | 2019, 272 | 748, 2215 | **stale** | 222 s |
| `shaping`, `find`, `embed`, `vfs_write` | none | 1317, 920, 3540, 4010 seeds; 1542, 1158, 2181, 9900 | **stale**: no corpus on disk | 1 to 5 s |
| `lexer` | 4907, 334 | 45972, 321 | current | 415 s |
| `document` | 15386, 6242 | 45972, 5765 | current | 216 s |
| `serialize` | 4778, 6552 | 45972, 6130 | current | 635 s |
| `page` | 10826, 55890 | 45998, 55910 | current (20 inside a margin of 1118) | 2966 s |
| `fragment`, `xfdf`, `meet`, `fetched_import` | | | current | 1 to 3 s |
| `jpeg_bands` | 192419, 3508 | 1125, 3197 | current | 1770 s |
| `object` | 5905, 748 | 45972, out of memory | not judged | 78 s |

- **The document population is 90 763 files and 126 GB now**, not 974, because `corpus-cache`
  grew. Every whole-document recipe writes some 46 000 seeds and 4.5 GB. Under `object`'s default
  2048 MB limit that set cannot be loaded at all, so a re-seed of `object` in an empty directory
  would leave a campaign with nothing it can load. This is the seeders' population and is named
  here; bounding it is a later round's.
- **`jpeg_bands` seeding is 47 s and 17 MB**: 1069 frame shapes and 56 fixture seeds, against
  192 419 seeds and 2.3 GB. It costs coverage, and the cost is measured: the fresh set reaches 3197
  edges, the disk corpus 3508. That 311 includes what round 1355's 1206 s campaign added, so the
  check calls the disk corpus current. Loading the disk corpus took about 1700 s of the 1770, which
  is what every campaign on it pays before its first mutation. The time comes from two things in
  `seed_streams.py`: a memory-mapped search for `/DCT` that skips every document naming none, and
  a stream search that starts from each `/DCT` instead of running regular expressions over every
  object. Over the 974 pdf.js documents the second finds the same 117 streams the general scan
  does. Documents over 4 MiB, which are 7% of the files and 65% of the bytes, are not read for
  this target, and the cost is any shape only such a document states.
- Not checked: `cmap`, `crypt`, `variable_text`, `sfnt`, `xmp`, `ccitt`, `cms`, `revocation`,
  `x509`, `jbig2`, `jpx`, `linearize`, `confined_wire`. Each walks the 126 GB through the general
  stream scan (`cmap`'s seeding alone ran past ten minutes), and the lock is shared by six rounds.

The re-seeds are the owner's to run in the main checkout:
`flock /home/AI/heavy-walk.lock fuzz/seeds.sh fuzz/corpus forms_data display_list shaping find embed vfs_write`.
This round wrote nothing there.

## 4. `fuzz/artifacts`, triaged

There are 74 files. Each was replayed against this tree's fuzz binaries:

- `serialize/crash-3a47ca5d…` and `x509/crash-60910642…` exit 0. Each already has its permanent
  test, `pdf-syntax/tests/serialize.rs`'s
  `a_root_that_reaches_no_dictionary_is_refused_rather_than_written` and `pdf-signature`'s
  `the_fuzzers_order_four_ed448_key_is_refused`, and each test names its artefact.
- `document`'s two slow units run in 55 and 100 ms.
- `page`'s 70 slow units, all dated 2026-09-01, exit 0 under `-timeout=60`. The slowest three
  took 74.5, 42.7 and 39.5 s of wall time. The first 32 were replayed outside the lock at a load
  average of 20, and the 74.5 s is one of them.
- There is no `jpeg_bands` file. Round 1355's `zune-jpeg` crash (Q227) was not kept on this disk.

So nothing here is a defect without a regression test, and there is no new defect.
