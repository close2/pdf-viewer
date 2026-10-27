# 1827 — SWF, ASF and MPEG-2 program stream clips judged by their own headers

Slot 5 of batch one hundred and twenty-three, 2026-10-27, a clause slot (Sonnet, high), finished from a
handover after the first agent stopped at its context bound; ADRs 2488 and 2489 used, Q693 unused. I
worked in `scratchpad/r1827/`. Row §13.2.4.3 stays `implemented` and its note, its code list and its test
list name the three readers; §13.2.2's note is untouched (it names no container).
**Premise** partly held. The census is now a command (`cargo run --release -p pdf-model --example
media_clip_census -- @list`): 93 080 documents opened, 172 skipped for size, 423 unreadable, 28 with media clip data
and 69 clips. With the three arms disabled 11 clips were read, 39 unread container, 19 not judged;
with them 22, 21 and 26. Read: 10 Flash, 1 Windows Media (GHOSTSCRIPT-695445-0: 38.093 s of play less
8 000 ms of preroll, 30.093 s, and the same figure from its bytes by hand, so the GUID layout is
confirmed on a real file), 3 AIFF, 5 QuickTime, 2 AVI, 1 MP4. Unread, for want of ISO/IEC 11172-3 and
13818-3: 19 `audio/mpeg` clips in 3 documents, plus 2 clips with no usable `/CT`. **Premises that
did not hold**: the first census walked indirect objects and found 15 documents, renditions inline
their media clip data, and the fixed walk found 28; the seven `video/mpeg` clips are all URLs, so no
corpus file exercises the program stream arm; the brief's ASF revision
01.20.06 could not be had and 01.20.03 was read. **Proposed habit**: a census over objects visits the
dictionaries nested inside a direct dictionary, not only the indirect objects.
**Built.** `media/container/{swf,asf,program_stream}.rs` (Adobe's version 19 header, `FrameLabel`
and `DefineSceneAndFrameLabelData`; ASF's File Properties and Marker objects, a preroll beside
markers refused; H.222.0 (07/95) clause 2.5.3's pack header, no length and no offset), `container::unread`,
`media::clip_timeline`, the census, eight fuzz content types and nine seeds. **Specs** cited by section,
none quoted, none committed: Adobe's mirror, the Internet Archive copy of Microsoft's page and the
ITU's free H.222.0; ISO/IEC 11172 and 13818-3 are not held, so MP3 and MPEG-1 stay unread.
**Proof (trap 13)**: with the SWF rate division changed, five unit tests and the integration test
fail; with the ASF preroll subtraction dropped two unit tests fail (the integration test uses no
preroll); with the program stream's third marker bit inverted two unit tests and the integration test
fail; each restored by its own reverse patch.
**Found, not fixed**: the MP4 reader's cut-short message prints a box type's raw bytes (the census
prints them as `?`); `fuzz/` has no clean full clippy because a sibling's `formcalc.rs` fails `manual_assert`.
**Gates.** `cargo fmt --check -p pdf-model` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model
--all-targets` exit 0; rustdoc `-D warnings --document-private-items -p pdf-model` exit 0;
`cargo test -p pdf-model --lib media::` 58 passed, `--test media_viability` 21 passed; `-p viewer-core
--test renditions` 5 passed; `tools/fuzz.sh media_container` 600 s exit 0, 38 781 293 runs, 985 edges
added; `cargo fmt --manifest-path fuzz/Cargo.toml --check` exit 0; fuzz clippy of `media_container` and
`jpx_write` exit 0 and of the whole manifest exit 101 on a sibling's `formcalc.rs`; `cited`, `quotations`
and `inapplicable` exit 0; the census `--tree 6` exit 0 twice (189 s, 193 s); `cargo test -p conformance
--no-fail-fast` exit 0, 470 passed (an earlier run failed `fuzz_workspace` on a sibling's `formcalc`
and `records` on 1823's record, both since fixed by their rounds).
The validator's five findings (the unsourced MPEG-1 pack-header branch, `MAX_TAGS`'s reason, the SWF inflate error's variants, the census's silent skips, the census input) were fixed by me and the gates below re-run. **Validation** (the orchestrator): the Opus validator's verdict was FAIL on five comment, provenance and census-tooling findings and PASS on the readers' behaviour, the citations and the plants; the round fixed them.
