Status: complete
Given: 2026-11-17, in conversation — transcribed by the round
Owes: once fonts-noto-cjk is installed on the VM (it may take a few rounds while a server audit finishes — check fc-list :lang=ja for Noto CJK first), re-run pdf-model's corpus and raster_golden walks and treat any page that still moves as a real finding (a tier-2 / model round)

> Q879: agreed with Option a.  I will install it as soon as the current server audit, improvements have finished.  Please give me short prompt I can then give the server project.  (Answer the question now, and say that it might take a few rounds until the font is installed)

Reading: adopts option (a) — the owner will install `fonts-noto-cjk` on this VM once an ongoing server audit and server improvements complete, so **it may take a few rounds before the font package appears on the machine**. Until `fc-list :lang=ja` reports Noto CJK installed, rounds continue comparing against `HEAD` as today; once `fonts-noto-cjk` is present, a round owes re-running both `corpus` and `raster_golden` and treating any page that still moves as a real finding.
