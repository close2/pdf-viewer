# 3806 — An overprinting mark on the simulated press chooses its transfer per component

Session 2276. Status: accepted. Supersedes ADR 3771. Context: ISO 32000-2 §11.7.5.2, §11.7.4.3,
§11.7.4.5 (Table 146), §10.5, §10.8.3; ADRs 1279 (the screen), 1311 and 1329 (overprint on the
planes), 3769.

## The reading

§11.7.5.2's last paragraph: "the halftone and transfer function to use at a given point shall be
determined independently for each colour component. An object is opaque for a given component only
if overprinting yields the source colour (not the backdrop colour) for that component." The rest of
the clause is unchanged per component: the topmost object is still chosen by shape, and where it is
not opaque the page's default is used. So at a pixel where an overprinting mark is topmost, a
component it is opaque for takes its function, and a component it keeps from the backdrop takes the
default — not the function of the object underneath, which is not the topmost object.

On §10.8.3's simulated press the components are its four process colourants and its spot
colourants, and which of them yields `C_s` is Table 146's: a `DeviceCMYK` colour under `OPM` 0
paints the four and keeps every spot colourant; under `OPM` 1 it keeps a zero tint as well; a
`Separation` or `DeviceN` colour paints the colourants it names and keeps the rest. The screen is
ADR 1279's answer and does not change: each of its three components is made from every process
colourant, so a mark that keeps one is opaque for none.

## The construction

- `pdf_render::OpaqueFor` is one flag per press component, and a `TransferRun` carries one beside
  its function where its marks keep some component (`TransferBuilder::push_opaque_for`); a run
  opaque for every component carries none, so runs merge as before.
- `pdf_model` computes it from the decision the planes are painted by — `DeviceSpots::overprint`
  asked for every plane, or the tints under `OPM` 1 on a page with no spot plane
  (`content::overprint::opaque_for`) — at the moment the decision is made, one slot for stroking
  and one for non-stroking marks because a `B` decides both before it paints either.
  `draw_mark` reads the slot for a mark that carries `BlendMode::Overprint`; an overprint no press
  decided (a page or group of one or three components, ADR 3805) is opaque for none.
- `resolve_press_transfers` resolves each run's table per component once and leaves a component
  the run is not opaque for alone; `resolve_transfers` treats such a run as the default.

Tested on every lane by `render-raster`'s `press_transfers`: a process colour overprinting a spot
one, and a spot colour overprinting the process colourants, each against the two answers the
departure and "every component" would give. Costs nothing on a page that does not overprint.
