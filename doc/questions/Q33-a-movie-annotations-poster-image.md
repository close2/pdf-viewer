# Q33 — Does §13.4's poster image come off principle 5's clause 13 exclusion?

Source: ADR 0906 (session 933), from the sweep of `appearance::construct`'s catch-all over every
subtype ISO 32000-2 Table 171 defines.
Status: **open** — answered when `A33-a-movie-annotations-poster-image.md` exists beside this file.

## The question

A `Movie` annotation with no appearance stream is refused and reported. The sweep that reads each
subtype's clause against that refusal found that §12.5.6.17 does state something to display, one
citation away: Table 189 makes `/Movie` required and calls it "[a] movie dictionary that shall
describe the movie's static characteristics", and §13.4's Table 306 gives that dictionary a
`/Poster` whose stream form is

> it shall contain an image XObject (see 8.9, "Images") to be displayed as the poster

An image `XObject` is what this tree draws on every page it opens. So the question is whether
drawing it is inside principle 5's clause 13 exclusion or outside it.

## Why it needs the owner

The exclusion list is closed and the owner's, and `CLAUDE.md` says an exclusion is revisited by
argument and never by attrition. The argument runs both ways and neither side is a reading of the
standard, which is what makes it the owner's rather than a round's:

- **Out.** The exclusion's stated *reason* is "a media engine, not a rendering question", and a
  poster is not a media engine: it is an image the file carries, decoded by the same code path as
  any other image, placed in `/Rect` by §12.5.5's algorithm. Nothing about it needs a player.
- **In.** The clause it is printed in is clause 13, and the entry's other form is unambiguously a
  media question — "if it is the boolean value true , the poster image shall be retrieved from the
  movie file", which needs the player to decode a frame. One entry, two forms, one on each side of
  the line.

The exclusion is also not what usually decides these: §12.5.6.17 is `partial` and deprecated in
PDF 2.0, so a decision either way changes nothing about a modern file.

## What the tree does meanwhile

The annotation is refused and **reported**, and since session 933 the report names the real
reason: `its clause states a poster image in §13.4's movie dictionary, which principle 5 excludes
with the rest of clause 13`. Nothing is drawn, nothing is silently dropped, and no page changes
whichever way this is answered — an appearance-less movie annotation is loud today and would be
drawn tomorrow. **Not a blocker.**

## Recommendation

Take the stream form and leave the boolean form excluded. It is one arm in
`crate::appearance::construct` reading `/Movie` → `/Poster`, drawing the stream as an image
`XObject` in `/Rect` — the same construction §12.5.6.19's `/MK` `/I` icon already uses — and it
costs the exclusion nothing it was drawn to protect: no player, no codec, no timeline, no
sandboxed media surface. The boolean form stays refused and reported, for the reason the exclusion
gives.
