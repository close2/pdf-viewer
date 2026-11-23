# 3405 — A sound's channels are mixed to two by their speakers

Session 2176. Status: accepted. Supersedes ADR 3397 decision 6 (the channel mask reported, not
applied). Context: ADR 3301 (§13.3's conversion in the host), ADR 3397 (the WAVE channel mask
read), ADR 3400 (AAC's decoded order); ISO 32000-2 §13.3; ITU-R BS.775 and Microsoft's *Multiple
Channel Audio Data and WAVE Files*, neither held in `doc/`.

## Decision

1. **A sound carries its speakers.** `pdf_media_worker::Sound::speakers` is a
   `WAVE_FORMAT_EXTENSIBLE` channel mask: the channels in frame order take its set bits from the
   lowest, a channel past the last set bit has no speaker, and `0` names none. A WAVE file's is its
   `dwChannelMask`; an AAC clip's is the mask `rusty_aac`'s `decode::layout` states for the
   configuration's output order — the channel configuration's, or the program configuration
   element's in the `AudioSpecificConfig` — kept only where it names one speaker a channel within
   thirty-two bits (a mono core that parametric stereo made two channels names none). It crosses
   in the run reply (`encode_pcm`), and a stream whose speakers change is refused like one whose
   format does. A sound object's is `0`: Table 305's `/C` is a count.
2. **On a device of one or two channels the channels are mixed by speaker.** §13.3 says "combine
   channels" and nothing of how, and no standard held defines a mix of speakers to fewer, so the
   weights are a choice, stated: ITU-R BS.775's two-channel downmix — front left and right at 1,
   centre at `1/√2` on both sides, each surround at `1/√2` on its own side, the low-frequency
   channel left out — extended by its pattern to the speakers it does not name (`convert::weights`).
   Each side is the weighted mean, the module's averaging rule, so no frame inside full scale is
   mixed past it; one channel is the mean of the two sides.
3. **A device of more channels keeps the order.** `quorra_sound`'s `OPENED` states a count and no
   positions, so the channels are placed by order as before, and a mask that is not the first
   speakers one per channel is said (`speakers_said`).
4. **What is said** moves from the worker's notes to the host's sentence for the sound, beside the
   device it was converted for: a mix by speaker, with the low-frequency channel named where it is
   left out, or the order kept on a device that states no speakers.

## Cost

One `u32` more on a sound and in a run reply. The weights quiet a lone front channel to about 41 %
in a 5.1 mix; that is the price of never clipping, and the averaging rule already paid it.
