# Q271 — May a projected system's registration points be read as degrees?

Asked by round 1375, which built §12.10's inverse projection on the owner's answer A171 and found,
in the census that answer asked for first, that the files and the table disagree about what the
projection is for.

## What the clause says

Table 269's `/GPTS` is "[a]n array of numbers that shall be taken pairwise, defining points in
geographic space as degrees of latitude and longitude, respectively when defining a geographic
coordinate system", and "[w]hen defining a projected coordinate system, this array contains values
in a planar projected coordinate space as eastings and northings". NOTE 2 beside it says only that any
projected coordinate system includes an underlying geographic one. `/PCSM`, where
present, "has priority over GPTS", and the table says that priority "provides backward
compatibility".

## What the files say

`crates/pdf-model/examples/geospatial_census.rs` over the 90 763 files of the crawl and the
corpora (ADR 1586): **158 documents state a projected `/GCS`, and every one of them writes its
`/GPTS` as latitudes and longitudes** — every pair inside ±90 by ±180 — and **not one states
`/PCSM`**. The two producers the census can name write it the same way: ArcMap 10.6.1
(`corpus-cache/safedocs/cc-main-2021-31/0669/0669055.pdf`, a Michigan State Plane system in feet
whose four points are 42.17 −84.56 and its neighbours) and a GDAL-written UTM zone 31N map in the
Tika corpus (`poppler-101379-0.pdf`, 43.04 2.74). No document anywhere writes eastings and
northings.

So the inverse projection serves none of them as the table describes it. Read as the table says,
each of those 158 maps' corners lands within a few hundred metres of its grid's origin — for a UTM
zone, on the equator half a zone west of the central meridian — which is the shape `measurement.rs`
was written to avoid: coordinates that look right and are somewhere else.

## What the tree does meanwhile

`Geospatial::registration_geographic` converts a projected system's points as eastings and
northings, as the table says, and **refuses** a set whose every pair is shaped as degrees with
`Refusal::RegistrationShapedAsDegrees`, naming the table's sentence. Nothing is guessed. The
inverse methods, the WKT reader, `/PCSM`'s leg and `/DCS` on the same datum are built and tested
on Guidance Note 7-2's worked examples, and wait on nothing.

## The options

1. **Keep the table's reading** and the refusal: a person tracing a path on any of the 158 maps is
   told the points contradict Table 269. Correct to the text, useful to nobody.
2. **Read degree-shaped points of a projected system as its base system's degrees**, as a
   `departed` row: the table's sentence is the one requirement withheld, its cost a conforming file
   whose eastings and northings all fall within 90 by 180 units of its grid origin, misread. The
   census found no such file, and for any system whose false origin offsets its grid by more than
   that the case cannot arise inside the system's own area.
3. **Ask the file**: read the points as degrees only where the eastings reading is impossible for
   the system — every point outside the projection's domain or its datum's area. This is option 2
   with a test no text states, which is the kind of rule principle 5 forbids inventing.

## Recommendation

**Option 2, as a departure with its ADR**, because the departure is the narrowest one available:
one sentence of one table, priced by the census at 158 documents gained and none lost, and NOTE 2
itself — every projected system includes an underlying geographic one — is the sentence that
reading leans on. The projection then serves those documents through a forward
projection of the registration points into the system's plane, where the map is linear, and back
through the inverse this round built; that forward leg is Guidance Note 7-2's own forward formulas
beside each inverse, and is the next round's build either way.
