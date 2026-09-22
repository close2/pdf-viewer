# 1208 — What ISO 18619 states that nothing held here does

Status: accepted. Session 1185.
Context: ISO 32000-2 §8.6.5.9, §8.6.5.8; ICC.1:1998-09, ICC.1:2001-12, ICC.1:2022-05, ICC.2:2023,
ISO 15076-1:2010, PDF 2.0 Application Note 001; ADRs 0012, 0187, 0510.
Code: `crates/pdf-colour/src/icc.rs` (`Profile::detect_black`, `Profile::to_xyz_with`).
Documents: `doc/third-party-data.md`, §8.6.5.9's ledger row.

`§N` is ISO 32000-2 and nothing else. Nothing of ISO 18619, of ICC White Paper 40 or of Adobe's
2006 paper is quoted anywhere below or in the tree: the first forbids reproduction and the other
two state no terms, which ADR 0187 answers the same way.

## 1. The question the row had been answering with the wrong noun

§8.6.5.9 states the `ON` case by reference and in one sentence: where `/UseBlackPtComp` is `ON`,
"colour conversion shall be carried out according to the provisions in ISO 18619." §8.6.5.9's
ledger row has been `partial` on that half since session 607, and its stated blocker was
**availability** — "ISO 18619 is still a document this project does not hold" (ADR 0510).

Two things were asked this session and both have answers.

**Do the held texts state the algorithm?** No, and the checking is worth recording because it
closes a family rather than a file. `doc/md/PDF20_AN001-BPC.md` — the PDF Association's own
application note, and the document this tree's `icc.rs` already cites for the phrase it
compensates towards — defines compensation in prose and then defers to ISO 18619 for the
formal definition; its reference list names two documents and one of them is ISO 18619.
ICC.1:1998-09 §6.4.22 and ICC.1:2001-12 §6.4.26 define the `mediaBlackPointTag` and nothing about
using it, and the 2001 edition adds a note withdrawing the 1998 edition's claim that the tag feeds
absolute colorimetry. **The v4 texts held here define no black-point tag at all**: ICC.1:2022-05
has no `mediaBlackPointTag`, the ISO 15076-1:2010 preview's foreword saying it was deleted, and
the phrase *black point compensation* appears in neither, nor in ISO 15076-1:2010. ICC.2:2023
names it twice — section A.1.9 and section K.2.2 — and both times says the operations are
specified by ISO 18619, with ISO 18619 in its *bibliography* rather than its normative
references.

ICC.1:2022 section 6.3.4.3 does carry a linear adjustment in PCS XYZ that holds white fixed and
moves black, and it is the right *shape*. It is not this operation: it repairs a legacy
pre-v4 perceptual transform against the perceptual reference medium's own black, one profile at a
time, with no source–destination pair and no intent branch, and the clause never calls it
compensation. Reading it as BPC would be the failure principle 5 exists to prevent.

**Is ISO 18619's content obtainable?** Yes, and free. The ICC publishes what its cover describes
as the final approved ICC version of the standard, as the 2013 committee draft ISO/CD 18619, and
ICC White Paper 40 states the mapping step outright. `doc/third-party-data.md` records the three
sources, their sizes and their terms; none is downloaded into the tree, because a worktree's
`doc/` does not survive its merge.

## 2. The decision

**§8.6.5.9's row keeps `partial`, and its blocker is restated as a construction rather than an
acquisition.** The row may no longer say the document is unavailable. What it says instead is what
the construction consists of, so that a round taking it knows the size of the job before it starts
and so that the gap between this tree's stretch and every other renderer's has a diagnosis rather
than a measurement.

## 3. What ISO 18619 states that nothing held here does

Paraphrased from ISO/CD 18619 (2013) clause 4, section by section. Eight items, and the first,
third and fifth are the ones that move pixels.

1. **It is a source-to-destination pair operation.** The inputs are two profiles and one
   rendering intent; the compensation is computed from the pair alone and never from image
   content, so it is precomputed per triple. This tree has no destination side at all — the
   display's black is taken as zero — which is the single largest divergence.
2. **The intent is constrained**, to relative colorimetric, perceptual or saturation, and it must
   be the same on both sides. §8.6.5.9's own "AbsColorimetric ⇒ OFF" rule falls out of that
   constraint, which this tree implements already for a different reason.
3. **The darkest colour of a profile is found from a stated vertex set.** Grey takes two vertices,
   RGB two, and **CMYK four** — the paper corner, the registration corner, black alone and the
   three chromatic inks together — each evaluated through the profile and compared by lightness.
   `icc.rs::detect_black` walks **two** corners, the all-zero and the all-one, and picks the
   darker; that is the right answer for grey and RGB and is short by two for CMYK.
4. **An output-capable CMYK source takes a different route entirely**: its local black is the
   connection space's own black taken through the profile's *perceptual* transform rather than
   found by walking device corners.
5. **A LUT destination's black point is estimated, and this is the bulk of the standard.** A
   256-step ramp is built in the connection space from black to white, round-tripped through the
   destination profile and back, forced monotonically increasing, tested for validity and for a
   straight mid-range, and then a quadratic is least-squares fitted to the shadow samples and its
   positive-gradient root taken as the black. Nothing of this exists here, and it is precisely
   the construction Little CMS implements — which is the diagnosis ADR 0510 measured without
   having: through a profile carrying a `B2A`, this tree's deep ink lands eleven levels from
   where poppler, mupdf and ghostscript agree, and through a profile carrying none, where this
   branch has nothing to do, the same evaluator lands with them.
6. **Both black points are clamped in lightness**, at a stated maximum, and their chromaticity is
   discarded before use.
7. **The mapping is a single scalar scale with an offset towards the D50 white**, applied
   uniformly to the three connection-space components — algebraically a blend of the source
   colour with a little white. This tree applies a **per-axis** stretch with the white pinned,
   which is not the same map and is not a stated approximation of it.
8. **A validity test decides when to do nothing**, so a profile whose estimation fails gets no
   compensation rather than an arbitrary one. `icc.rs`'s `usable` guard is this tree's own answer
   to the same question, arrived at from a defect rather than from the clause (ADR 0510's fifth
   failure).

## 4. What this does not decide

**Whether to build it.** Items 1, 3, 5 and 6 are a self-contained piece of work with a measurable
target — the eleven levels above — and no patent obstacle, Adobe's own paper stating it holds
none. This ADR says what it is, not that it is next.

**Whether to fetch the documents.** They are free and reachable and this tree does not hold them;
`doc/third-party-data.md` states the three URLs and their terms, and a round on the main branch
can act on that in one command. This worktree deliberately did not, because nothing it writes
under `doc/` or `doc/md` survives the merge.

**Anything about the `OFF` and `Default` halves**, which are implemented and tested and are not
this ADR's subject.
