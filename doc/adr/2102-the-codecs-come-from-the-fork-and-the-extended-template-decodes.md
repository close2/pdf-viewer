# 2102 — The codecs come from the owner's fork, and the extended template decodes

Session 1633, slot 3 of batch ninety-nine, a clause round. Status: **accepted**, **built**. Context:
`doc/questions/A514` (the owner pushed `quorra/t88-patches`), ADR 2074 section 4 (the steps), ADRs
1447, 1459 and 1485 (the three commits), 1714 (why a git source at all). Clauses: ISO 32000-2
§7.4.7; ITU-T T.88 (08/2018) 6.2.5.3, 6.2.5.4, 6.2.5.7, 7.4.6.2, 7.4.6.3 and Table 5, cited and
paraphrased. ADR 2103 is unused.

## 1. What moved

`Cargo.toml`'s `hayro-jbig2`, `hayro-jpeg2000` and `hayro-ccitt` stanzas name
`https://github.com/close2/hayro` at `7e39af411b807cc267fb7a72545083f10ca2077f`, one source for the
three as the CCITT stanza requires. `cargo update -p hayro-jbig2@0.3.1 -p hayro-jpeg2000@0.4.1 -p
hayro-ccitt@0.4.0` moved exactly those three packages' `source` in `Cargo.lock`, and the same in
`fuzz/Cargo.lock`, which is its own workspace and decodes through the same `pdf-sandbox`. The fork
replaces upstream's repository in `deny.toml`'s `allow-git`; the crates.io copies the reference
renderer reaches keep their `skip` lines. The three files under `doc/patches/` are deleted, and so
are `decode::extended_template_region`, its refusal's sentence, and the two `SegmentHeader` fields
only it read.

## 2. What the fork was checked to hold

A fetch of the one commit into a bare repository in scratch, nothing run from it but `git` and, in
a separate unpacked copy, the codec's own unit tests. `ea9c81dc..7e39af41` is three commits and
touches five files, all under `hayro-jbig2/`; `hayro-jpeg2000/` and `hayro-ccitt/` are the release
commit's byte for byte, so no JPEG 2000 or CCITT decode can move. Each commit's added and removed
lines equal its patch's as a multiset (72, 147 and 321 lines). The fork's
`arithmetic_decoder::tests::table_h1_decision_by_decision` passes there: T.88 Annex H.2's 256
decisions, register for register.

## 3. The decode that replaced the refusal's test

`decode.rs`'s `a_generic_region_on_the_extended_template_decodes` takes the file's own 94-byte
stream (`PDFIUM-1236-1.pdf`, a template-0 region with its AT pixels at their nominal locations and
TPGDON 0), sets EXTTEMPLATE, puts Table 5's twelve nominal pairs where the ordinary four were, grows
the segment's length by sixteen bytes, and holds the decode to the ordinary template's bitmap. The
expected value is the clause's: Figure 3(b) at Table 5's locations covers Figure 3(a)'s sixteen
cells, read from the figure itself; 6.2.5.7 step 3 d) leaves the gathering order to the decoder,
provided it is consistent and does not depend on where the AT pixels are, and step 4 makes the
bitmap the one every decoder obtains. **Planted back** — the same test in an export of HEAD's manifest — it
fails with "a different bitmap" on upstream's 0.3.1, which reads four pairs and starts its
arithmetic decoder sixteen bytes early. Annex K item 7's generic region is held as `Matches` in
`t88_conformance.rs`, and the "waits on a patch" outcome and the codec-only check that said when
to delete the refusal are gone with it.

## 4. A premise that did not hold: "refuses each at once"

The seven symbol dictionaries of minutes were said to be refused by the patched codec at once. Held
to the codec's own sentence, five are, and `symbol_pixels_a` and `symbol_pixels_b` are refused after
about 0.65 s in the `dev` profile: the bound decodes its 2^28 symbol pixels before it refuses, so the
250 ms deadline the tests gave them ended them first. So the seven are decoded under a 30 s
`BOUND_DEADLINE` and held to the codec's sentence on both isolations — a lock that stopped naming
the fork now fails there instead of passing at a deadline. That took away the only input that
reached the confined worker's kill, so `confinement.rs` gives the kill an input of its own: the
largest bilevel region the filter produces, slow for an honest reason (moved, verbatim, from
`in_process_deadline.rs` into the shared test module), ended by `TimedOut` at 250 ms with the next
request answered.

## 5. The rows

§7.4.7 is `implemented`: the three sentences addressed to the reader are executed, and the extended
template was the one requirement of ISO 14492:2019 inside "excluding colour palette coding" that
was not. §7.4 stays `partial`, derived (ADR 1599), because §7.4.9 owes; the ledger holds no §7 row.

## 6. The corpus did not move

The six arms of the export (HEAD plus `Cargo.toml`, `Cargo.lock` and `decode.rs`), against HEAD's in
`/home/AI/arms-1631/`, by page digest: no page moved in any arm, 968 pages at 1× and 965 at 4× (the
comparison names a planted digest). The arms hold 104 documents that name `JBIG2Decode`, and the
first page of each, the one the gate draws, is unmoved in every arm. Their 110 JBIG2 streams, extracted by
`image_codec_seeds` and walked over segment headers, hold no template-0 generic region with
EXTTEMPLATE set (84 on template 0, 5, 5 and 5 on templates 1 to 3, 1 MMR); the walk names a planted
one. The fuzz seeds' 8 701 streams hold none either.
