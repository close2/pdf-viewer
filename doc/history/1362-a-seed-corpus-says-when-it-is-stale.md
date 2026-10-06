# 1362: a seed corpus says when it is stale, and the tight-consensus page is the references'

This round had the robustness slot of batch fifty-six. It wrote ADRs 1559 and 1560, moved no
ledger row and wrote no question.

**`fuzz/seeds.sh check <target>...`** (ADR 1559) seeds a target afresh into a scratch directory
beside the build output. It then reads libFuzzer's `INITED cov` from two `-runs=0` passes: one
over the disk corpus, named second behind an empty directory so the corpus is only read, and one
over the fresh seeds, each under the limits in `doc/verify.md`. It prints **STALE** when the fresh
seeds lead by more than 2% of their own figure, and after that what a re-seed would add and where.
The binary runs by path, because `cargo fuzz run` creates `fuzz/artifacts/<t>` in the main
checkout. The main checkout's corpora were left as they were.

**Stale today:** `forms_data` (disk 528 against fresh 2072) and `display_list` (272 against 2215).
`shaping`, `find`, `embed` and `vfs_write` have no corpus on disk at all. `lexer`, `document`,
`serialize`, `page`, `fragment`, `xfdf`, `meet`, `fetched_import` and `jpeg_bands` are current.
`object` was not judged: its 45 972 fresh seeds are 4.5 GB and exceed its 2048 MB limit. Thirteen
targets were not checked, because each walks the whole population through the general stream
scan. **The population is 90 763 documents, 126 GB**, not the 974 the earlier records assume.

**`jpeg_bands`** now writes the smallest frame of each shape: marker, precision, components and
sampling, `DRI`, `DNL`. It reads only documents of 4 MiB or less that name `/DCT`, finding the
streams from the name. The result is 1069 shapes plus 56 fixture seeds, 17 MB in 47 s, where the
old seeding wrote 192 419 seeds and 2.3 GB. The fresh set reaches 3197 edges against the disk
corpus's 3508, and the disk figure includes round 1355's campaign.

**`issue7891_bc1.pdf`** (ADR 1560): §10.7.4's rules are normative and work in whole pixels. Only
the NOTEs in §10.7.1 and §11.3.7.2 expect anti-aliasing, so departure (1) stands as a departure.
The verdict, though, is the worst tile, the word at device (224, 320), where no clip edge lies.
I wrote the closed form from the file. Drawing the `/BBox` edges as the clause's set leaves 6.73.
The clause carried out throughout is 6.80 against `mupdf`, with a bound of 6.04. So the group is
`Whose::References`, and `issue4436r.pdf` is now the one page held as ours.

**Triage.** The 74 files in `fuzz/artifacts` are 2 crashes, each with its permanent test and
neither reproducing, and 72 slow units that all exit 0 under `-timeout=60`. There is no new defect.

**Gates.** rustfmt on `oracle.rs` 0. `RUSTFLAGS=-D warnings` clippy `-p pdf-model --all-targets`
0. `cargo nextest run -p pdf-model` 1785 passed. `cargo test -p conformance` 0, and its
`fuzz_workspace` 4 passed. `bash -n` and `py_compile` 0. `seeds.sh check` behind the lock: `page`
2966 s, `jpeg_bands` 1770 s, `forms_data` 5 s. `held` prints 1 / 13 / 33.
