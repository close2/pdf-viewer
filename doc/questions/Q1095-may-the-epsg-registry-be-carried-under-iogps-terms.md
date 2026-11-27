# Q1095 — May the EPSG registry be carried in the program, under IOGP's terms of use?

Source: round 2274, review 2271's D01 (ADR 3798).

## What the tree does now

§12.10.3 (Errata Collection 3, Issue #321) lets a file describe its coordinate system by "a numeric
EPSG reference code" or by WKT. This program reads WKT and refuses a code alone, by its number,
because no registry is carried (ADR 1586, on a census that A865 says is not what counts). The 12.10.3
row is `partial` until this is answered.

## What carrying it costs

PROJ 9.9.0's `data/sql`, EPSG v13.102, a pinnable public source: 902 geographic CRSs (637 2D, 265
3D), 5417 projected over them, 624 datums, 50 ellipsoids, 15 prime meridians, non-deprecated. A
compiled-in `static` table of codes, parameters and unit codes, no names, is about 360 KB (385 KB with
the deprecated codes). The eight projection methods built cover 95.2 % of the projected systems; the
rest need about 35 more (Cassini, polar stereographic, Hotine, Lambert azimuthal first).

## The licence (https://epsg.org/terms-of-use.html, revised 8 April 2016)

It allows use, copying and distribution, subsets included, with IOGP's ownership acknowledged and
values unchanged beyond its Table 1, and lets the data "be included in any commercial package
provided that any commerciality is based on value added by the provider and not on a value ascribed
to the EPSG Dataset". Two sentences do not sit with Apache-2.0: "Distribution for profit is
forbidden", and "[y]ou are obliged to inform anyone to whom you provide the EPSG Facilities
of these Terms of Use". So the data portion would carry terms of its own beside the program's licence,
and a downstream seller of the program would be bound by them. PROJ ships the same data under MIT
without restating them.

## The options

1. **Carry it**, as a separate data file in the tree under IOGP's terms (a `NOTICE` and a
   `LICENSE-EPSG`), generated reproducibly from a pinned PROJ tag, and build the methods past the
   eight as their own rows. *Recommendation, if the owner accepts the second licence.*
2. **Carry it after asking IOGP** whether an Apache-2.0 program may embed the subset; a round
   writes the letter, the owner sends it.
3. **Depart**: keep refusing a code alone, by its number, and the row becomes `departed` on this
   answer, priced above.
