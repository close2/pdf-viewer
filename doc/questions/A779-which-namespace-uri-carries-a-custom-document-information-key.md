Status: complete
Given: 2026-11-04, in conversation — transcribed by the round
Owes: change CARRIED_INFORMATION_NAMESPACE in crates/pdf-transform/src/archive/prepare/metadata.rs (and its tests/docs) from close2/pdf-viewer to close2/quorra (https://github.com/close2/quorra/ns/document-information/1.0/, prefix docinfo) since pdf-viewer will move to quorra later (an archive round)

> Q779: option a.  We will move pdf-viewer to quorra later, so it might make sense to already use https://github.com/close2/quorra/ns/document-information/1.0 instead.

Reading: adopts option (a) with the repository path updated to `quorra` ahead of the repository rename — set `CARRIED_INFORMATION_NAMESPACE` in `crates/pdf-transform/src/archive/prepare/metadata.rs` to `https://github.com/close2/quorra/ns/document-information/1.0/` (retaining the trailing `/` XMP namespace separator and prefix `docinfo`).
