# 1825 — A deep JPEG 2000 image outside the baseline is written back as JPEG 2000, and the census finds none

Slot 3 of batch one hundred and twenty-three, 2026-10-27, an archive slot (Sonnet, high); ADRs 2484
and 2485 used, Q691 raised (ADR 2485 is proposed on it). I worked in `scratchpad/r1825/` (two sessions of mine, a handover between). No
ledger status moves (ISO 19005 has no ledger rows).
**Premise** (the refusal's instances are few) held more strongly than stated: **none**. A scan of every
image a JPX-site finding names, opened with the shipped decoder: pdf.js 3 images (all 8-bit, a stated
`/ColorSpace`, 963 of 974 documents opened), `doc/corpora` 0 (487 of 503), the veraPDF corpus 4 (all
8-bit, none states a `/ColorSpace`; 2908 of 2908); deeper than sixteen bits: 0 everywhere. The three
pdf.js images are refused earlier, at the shipped profiles' default decision. The archive census
with the pack present is line for line the one on HEAD, so the finish line is "converted count
unchanged, none moved", and the write-back is proved on planted documents.
**Built (ADR 2484).** `transcode` sends precision past sixteen to `written_as_jpeg_2000`: one or three
colour channels through `pdf_jpx_write::jp2`, colour planes at the widest depth with `/Decode` widened,
the opacity channel inside the file, the dictionary kept (`JPXDecode`, `/ColorSpace`, `/SMaskInData`),
and a read-back proof through `pdf_model::image::jpx_samples` before the original is replaced. Refusals
kept by name: four channels (waits for ADR 2480's JPX file), and the two veraPDF 1.30.3 forced
(ADR 2485, Q691: opacity of another depth, a `bpcc` box, 6.2.8.3 test 5; grey plus opacity, test 1; a conflict between the clause's words and the validator, left refused until the owner rules). veraPDF
passed 12 of 12 planted outputs at 2b and 4. The rewrite id `jpeg2000-transcoded-to-flate` is kept,
its cost stated in ADR 2484.
**Found, not fixed.** (1) A deep image with a widened non-default `/Decode` draws posterised through
the `pdf-model` reader (probably the 16-bit cap of the `Decode` table in `image/space.rs`; unconfirmed;
the written integers and pairs are §8.9.5.2's exact map). (2) `pdf-archive` accepts a `bpcc` box and a
grey-plus-opacity file that veraPDF rejects. (3) The decoder reads channel type 2 as no opacity, so
the redaction writer's premultiplied output would not read back through `pdf-model`; slot 1 should
check. (4) Slip: the first session ran `cargo fmt -p pdf-transform` once in the shared worktree.
**Gates.** In an export of HEAD and my hunks: `cargo fmt --check -p pdf-transform` exit 0; `RUSTFLAGS="-D
warnings" cargo clippy -p pdf-transform --all-targets` exit 0; rustdoc `-D warnings --document-private-items`
exit 0; `cargo test --profile gates -p pdf-transform --no-fail-fast` exit 0, 746 passed, 0 failed,
21 ignored (`deep_jpx` 7 passed); plants: disabling the route fails all 7, the `/Decode` widening fails
the mixed-depth test, and disabling the opacity-depth or the grey-plus-opacity refusal fails its test,
each restored; with the pack `archive_corpus --ignored` exit 0, 7 passed; `cross_check --ignored` exit
0, 13 560 compared, 0 disagreements; census `--tree 6` exit 0 before and after, identical;
`cargo tree -i pdf-transform --edges all` exit 0 reaches no render crate, so against HEAD's six arms
nothing moves; `cargo run -p conformance --bin cited` and `--bin quotations` exit 0;
`tools/state.sh --round 1825 archive` exit 0; `cargo test -p conformance --no-fail-fast` exit 0, 470 passed, 0 failed. The Opus validator's three findings (ADR 2485's argument and Q691, ADR citations in `transcode.rs`, the count in `doc/todo/72`) were fixed by me. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on ADR 2485's argument (ADR 0922 read the wrong way round — the conflict is now Q691's), two citations and an unreproducible count, and PASS on the route, the read-back proof, the plants, veraPDF and the census; the round fixed the findings.
