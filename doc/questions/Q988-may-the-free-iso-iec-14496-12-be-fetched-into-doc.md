# Q988 — May the no-charge ISO/IEC 14496-12 be fetched into `doc/`, and the in-tree MP4 reader stand?

Source: round 2167, RFC 0015 section 5's step 8 (ADR 3369).

## What the tree does now

`pdf_media_worker::mp4` reads an MP4 file's sound track in the confined media worker: the boxes of
ISO/IEC 14496-12 an audio track needs, and the `esds` descriptor of 14496-14 and 14496-1. None of
the three is held in `doc/`, so the module paraphrases the layout by section number and quotes
nothing. The ITTF "publicly available standards" site, from which 14496-12 used to be free, is
closed (fetched 2026-11-22); its page says the free deliverables are now on the ISO and IEC
webstores at no charge. Getting one there is an account and a checkout, which a round does not do.

## The two asks

1. **Fetch 14496-12 (the current edition) from the ISO or IEC webstore's no-charge list into
   `doc/`**, as ISO 32000-2 and ISO 14289 are held. A later round then checks the module's
   paraphrase against the text and replaces "not held" in ADR 3369 and the module comment.
   *Recommendation: yes.* 14496-1 and 14496-3 are not on the free list as far as this round knows;
   the descriptor layout stays paraphrased.
2. **The reader is in-tree, against the reviewed-crate precedent of the bigint answer.** ADR 3369
   argues it: the subset is about 900 lines and the same sample-table walk is already in
   `pdf_model::media::container`; two crates (`shiguredo_mp4`, `mp4-atom`) pass `deny.toml` and are
   fuzzed and are the ones to weigh if it proves wrong. *Recommendation: keep it in-tree.*

## Meanwhile

The reader is built, tested and fuzzed, and is not yet called by the worker's `answer`: step 9
(AAC through `rusty_aac`) adds the wire and the decoder. Nothing waits on this answer.
