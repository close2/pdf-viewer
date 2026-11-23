# 3406 — A marker offset over a cue-point WAVE file is played here

Session 2176. Status: accepted. Context: ADR 3398 (media clip sections played here), ISO 32000-2
§13.2.4.3 and Table 289.

## Decision

ADR 3398 placed a marker at the offset its container states and left the path untested end to
end. `viewer-host`'s `played_sounds` now plays a screen annotation's rendition whose media clip
section's `MH` `/B` is a marker (`/S /M`) over an embedded WAVE file with a `cue ` chunk and a
`LIST` `adtl` `labl`: the core places the section at the cue's sample over the file's rate, the
section crosses with the sound, and the player hands the device the frames from the cue on. No
code changed: the reading of a cue point's position as a sample index (`dwSampleOffset`, the RIFF
form's) was already `container::wave`'s, and the test excludes a byte offset into the data chunk.
