Status: complete
Given: 2026-11-23, in conversation — transcribed by the round
Owes: check pdf_media_worker::mp4 against doc/ISO-IEC-14496-12-2012.pdf and replace "not held" in ADR 3369 and the module comments with citations to the placed fourth edition

> Q988: copy the iso file and keep in-tree mp4 reader

Reading:
1. ISO/IEC 14496-12:2012(E) (Fourth Edition, 196 pages, 1,873,614 bytes, SHA-256 `96d293519651696a6690f1e3b92b1097140edab1ca34ebd47f79d2c805959a96`) has been placed in `doc/ISO-IEC-14496-12-2012.pdf` (gitignored alongside the other reference PDFs in `doc/`). It covers the ISO Base Media File Format boxes read by `pdf_media_worker::mp4` (`ftyp`, `moov`, `trak`, `tkhd`, `mdia`, `mdhd`, `hdlr`, `minf`, `stbl`, `stsd`, `stts`, `stsc`, `stsz`, `stco`, `co64`, `elst`). A later round should verify the module's paraphrases against this placed text and update ADR 3369 and the module comments accordingly. ISO/IEC 14496-1, 14496-3, and 14496-14 remain unheld, so the `esds` descriptor layout remains paraphrased.
2. Keep the in-tree MP4 box demuxer (`pdf_media_worker::mp4`) as ADR 3369 recommends: it is a small, zero-`unsafe`, fuzzed container reader (not a codec, with AAC decoding delegated to `rusty_aac`). Revisit external crates (`shiguredo_mp4` or `mp4-atom`) only if the in-tree subset outgrows its scope or develops a recurring bug class.
