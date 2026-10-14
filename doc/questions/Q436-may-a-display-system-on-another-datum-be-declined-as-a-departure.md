# Q436 — May a display system on another datum be declined, as a departure, so that §12.10.2 is settled?

Source: round 1541, ADR 1918, which built A271's reading and left §12.10.2's row `partial` for one
requirement.

## What is left of the row

Table 269's `/DCS` is "[a] projected or geographic coordinate system that shall be used for the
display of position values". The tree reaches it wherever `/DCS` is on `/GCS`'s own datum — a
geographic system as the same latitude, a projected one through its forward projection — and
refuses one on another datum by both datums' names (`Refusal::DatumTransformation`). That is the
clause's own illustration, a map "in a state planar coordinate system based on a 1927 datum"
displayed "in the WGS84 datum corresponding to values reported by a GPS device", and reaching it is
a datum transformation: parameters that are in no table of ISO 32000-2, sometimes in a WKT1
`TOWGS84[…]` clause, more often in a grid of shifts held by a national survey.

`doc/questions/A171`'s reading put datum transformation outside the projection's scope; it did not
decide against this sentence, which is why the row says it is not executed rather than departed.

## What the files say

`examples/geospatial_census.rs` now asks `Geospatial::display` of every `/DCS` beside a `/GCS`,
calibrated on the drive's `drive-displayed.pdf`, which it reads as reached. **Over the 90 763 files
of the crawl and the corpora, not one document states a `/DCS` at all** — 40 files hold the bytes
`/DCS`, and none as an entry of a geospatial measure dictionary.

## The options

1. **Decline it as a departure**: §12.10.2 becomes `departed` on two sentences, A271's and this one,
   each priced (151 documents gained; 0 documents reached), and the refusal stays by name.
2. **Build it where the file states the parameters**: a WKT `TOWGS84` seven-parameter Helmert
   transformation, refused where the string states none. No census document would use it.
3. **Leave the row `partial`**, as it is now.

## What the tree does meanwhile

Option 3: a `/DCS` on another datum is refused with both datums' names, and the position on the
file's own system is shown beside the refusal.

## Recommendation

**Option 1.** The population is empty, so option 2 is a build with no beneficiary and a geodetic
question (which transformation, which epoch) the standard does not ask; a departure says so with
its price and lets the row settle. Option 2 stays open for a round that finds a file needing it.
