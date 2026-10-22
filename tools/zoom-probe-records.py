#!/usr/bin/env python3
"""Validate bounded opt-in toolkit receipts; timestamps are process-local.

A byte boundary must be taken before each stimulus. Freshness and coordinate agreement
are necessary observations, not proof that a particular physical input caused an event.
The driver owns session identity, process/window selection, focus and deadlines.
"""
import json
import math
from dataclasses import dataclass

PREFIX = b"zoom-record "
LIMIT = 4 * 1024 * 1024


@dataclass(frozen=True)
class Boundary:
    prefix: bytes
    session: str
    host: str
    pid: int
    window: int
    sequence: int
    us: int


def records(data, boundary, session):
    """Return new complete owned records, rejecting ambiguous/reset input streams."""
    if session != boundary.session:
        raise ValueError("driver session changed")
    if len(data) > LIMIT or len(boundary.prefix) > LIMIT:
        raise ValueError("trace bound exceeded")
    if boundary.prefix and not boundary.prefix.endswith(b"\n"):
        raise ValueError("boundary splits a log line")
    if not data.startswith(boundary.prefix):
        raise ValueError("trace reset or replacement")
    tail = data[len(boundary.prefix):]
    if tail and not tail.endswith(b"\n"):
        raise ValueError("incomplete trace line")
    owned = []
    seen = set()
    last_entry = boundary.sequence
    last_us = boundary.us
    span = None
    accumulated = False
    for line in tail.splitlines():
        if not line.startswith(PREFIX):
            continue
        try:
            row = json.loads(line[len(PREFIX):], parse_constant=lambda _: None)
        except (ValueError, UnicodeDecodeError) as error:
            raise ValueError("malformed receipt") from error
        if not isinstance(row, dict) or row.get("v") != 1:
            raise ValueError("unknown receipt schema")
        if row.get("pid") == boundary.pid and row.get("host") == boundary.host and row.get("kind") in ("dispatch-begin", "dispatch-end") and row.get("window") != boundary.window:
            raise ValueError("foreign-window dispatch makes accumulator association ambiguous")
        if row.get("kind") == "zoom-accumulator" and row.get("pid") == boundary.pid and row.get("host") == boundary.host:
            if span is None or accumulated:
                raise ValueError("unpaired or repeated accumulator")
            if row.get("unit") not in ("pixels", "notches") or type(row.get("steps")) is not int:
                raise ValueError("invalid accumulator units or steps")
            for key in ("amount", "core_x", "core_y"):
                value = row.get(key)
                if type(value) not in (int, float) or not math.isfinite(value):
                    raise ValueError("invalid accumulator value")
            # The Rust record has neither the C++ clock nor a window identifier.
            # Retain raw provenance and bind it only to the synchronous owned span.
            owned.append(dict(row, span_sequence=span))
            accumulated = True
            continue
        if (row.get("host"), row.get("pid"), row.get("window")) != (
            boundary.host, boundary.pid, boundary.window
        ):
            continue
        stamp = row.get("us")
        if type(stamp) is not int or stamp < last_us:
            raise ValueError("missing or reversed monotonic timestamp")
        last_us = stamp
        kind = row.get("kind")
        if kind in ("motion-entry", "motion-outcome", "wheel-entry", "wheel-branch", "wheel-outcome", "dispatch-begin", "dispatch-end") and "sequence" not in row:
            raise ValueError("callback receipt has no sequence")
        if "sequence" in row:
            sequence = row["sequence"]
            kind = row.get("kind")
            if type(sequence) is not int or sequence <= boundary.sequence:
                raise ValueError("stale or reused callback")
            key = (kind, sequence)
            if key in seen:
                raise ValueError("duplicate callback receipt")
            seen.add(key)
            if isinstance(kind, str) and kind.endswith("-entry"):
                if sequence <= last_entry:
                    raise ValueError("callback sequence went backwards")
                last_entry = sequence
        if kind == "dispatch-begin":
            if span is not None or row.get("operation") != "zoom":
                raise ValueError("nested or unknown dispatch span")
            span = row["sequence"]
            accumulated = False
        elif kind == "dispatch-end":
            if span != row["sequence"] or not accumulated or row.get("operation") != "zoom":
                raise ValueError("incomplete dispatch span")
            span = None
        owned.append(row)
    if span is not None:
        raise ValueError("unterminated dispatch span")
    return owned


def motion(data, boundary, session, expected=None, tolerance=0.5):
    """Accept one unambiguous fresh toolkit motion and its accepted matching outcome."""
    rows = records(data, boundary, session)
    entries = [r for r in rows if r.get("kind") == "motion-entry"]
    if len(entries) != 1:
        raise ValueError("missing or ambiguous motion entry")
    entry = entries[0]
    if entry.get("action") != "moved":
        raise ValueError("calibration received a drag or unknown motion action")
    outcomes = [r for r in rows if r.get("kind") == "motion-outcome"
                and r.get("sequence") == entry["sequence"]]
    if len(outcomes) != 1 or outcomes[0].get("outcome") != "accepted":
        raise ValueError("missing or suppressed motion outcome")
    outcome = outcomes[0]
    if any(r.get("sequence") != entry["sequence"] for r in rows if r.get("kind") == "motion-outcome"):
        raise ValueError("orphan motion outcome")
    for key in ("core_x", "core_y"):
        value = outcome.get(key)
        if type(value) not in (float, int) or not math.isfinite(value):
            raise ValueError("invalid core coordinate")
    if rows.index(outcome) < rows.index(entry):
        raise ValueError("outcome precedes entry")
    for index, key in enumerate(("logical_x", "logical_y")):
        value = entry.get(key)
        if type(value) not in (float, int) or not math.isfinite(value):
            raise ValueError("invalid pointer coordinate")
        if expected is not None and abs(value - expected[index]) > tolerance:
            raise ValueError("pointer differs from requested owned-widget position")
    return entry, outcome


def wheel(data, boundary, session):
    """Count callbacks separately from zoom branches and accepted dispatches."""
    rows = records(data, boundary, session)
    entries = [r for r in rows if r.get("kind") == "wheel-entry"]
    if not entries:
        raise ValueError("missing wheel entry")
    callbacks = []
    for entry in entries:
        sequence = entry["sequence"]
        branches = [r for r in rows if r.get("kind") == "wheel-branch"
                    and r.get("sequence") == sequence]
        outcomes = [r for r in rows if r.get("kind") == "wheel-outcome"
                    and r.get("sequence") == sequence]
        if len(branches) != 1:
            raise ValueError("missing or ambiguous wheel branch")
        branch = branches[0]
        if type(entry.get("ctrl")) is not bool:
            raise ValueError("missing control-modifier state")
        if entry["ctrl"] and branch.get("branch") != "zoom":
            raise ValueError("control-wheel chose a nonzoom branch")
        if branch.get("branch") == "zoom":
            amount = branch.get("amount")
            if not entry["ctrl"] or type(amount) not in (int, float) or not math.isfinite(amount):
                raise ValueError("invalid zoom branch")
        if rows.index(branch) < rows.index(entry):
            raise ValueError("branch precedes callback")
        if branch.get("branch") == "delegate":
            if outcomes:
                raise ValueError("delegated callback claims host outcome")
        elif branch.get("branch") in ("zoom", "scroll"):
            if len(outcomes) != 1 or outcomes[0].get("outcome") not in ("accepted", "host-busy"):
                raise ValueError("missing wheel dispatch outcome")
            if rows.index(outcomes[0]) < rows.index(branch):
                raise ValueError("outcome precedes branch")
        else:
            raise ValueError("unknown wheel branch")
        spans = [r for r in rows if r.get("kind") == "dispatch-begin" and r.get("sequence") == sequence]
        if branch.get("branch") == "zoom" and outcomes[0]["outcome"] == "accepted":
            if len(spans) != 1:
                raise ValueError("accepted zoom has no synchronous accumulator span")
            accumulator = [r for r in rows if r.get("kind") == "zoom-accumulator" and r.get("span_sequence") == sequence]
            if len(accumulator) != 1 or type(branch.get("pixels")) is not bool:
                raise ValueError("missing accumulator or zoom units")
            facts = accumulator[0]
            if facts["unit"] != ("pixels" if branch["pixels"] else "notches"):
                raise ValueError("accumulator unit differs from branch")
            coordinates = []
            for key in ("logical_x", "logical_y", "dpr"):
                value = entry.get(key)
                if type(value) not in (int, float) or not math.isfinite(value):
                    raise ValueError("invalid wheel coordinate or scale")
                coordinates.append(value)
            if coordinates[2] <= 0:
                raise ValueError("invalid wheel scale")
            expected = dict(amount=branch["amount"], core_x=coordinates[0]*coordinates[2], core_y=coordinates[1]*coordinates[2])
            if not all(math.isclose(facts[key], value, rel_tol=1e-6, abs_tol=1e-5) for key, value in expected.items()):
                raise ValueError("accumulator inputs differ from callback branch")
            ends = [r for r in rows if r.get("kind") == "dispatch-end" and r.get("sequence") == sequence]
            if len(ends) != 1 or not (rows.index(branch) < rows.index(spans[0]) < rows.index(ends[0]) < rows.index(outcomes[0])):
                raise ValueError("misordered zoom dispatch")
        elif spans:
            raise ValueError("nonaccepted zoom claims accumulator span")
        callbacks.append((entry, branch, outcomes[0] if outcomes else None))
    entry_ids = {r["sequence"] for r in entries}
    if any(r.get("sequence") not in entry_ids for r in rows
           if r.get("kind") in ("wheel-branch", "wheel-outcome", "dispatch-begin", "dispatch-end")):
        raise ValueError("orphan wheel receipt")
    return callbacks


def logical_to_core_mapping(first, second):
    """Derive per-axis logical-to-core scale only from two distinct accepted positions."""
    result = []
    for logical, core in (("logical_x", "core_x"), ("logical_y", "core_y")):
        delta = second[0][logical] - first[0][logical]
        a, b = first[1].get(core), second[1].get(core)
        if delta == 0 or any(type(v) not in (int, float) or not math.isfinite(v) for v in (a, b)):
            raise ValueError("degenerate or missing coordinate mapping")
        scale = (b-a)/delta
        if not math.isfinite(scale) or scale <= 0:
            raise ValueError("invalid coordinate scale")
        result.append(scale)
    return tuple(result)


def stimulus_to_core_mapping(first_request, first, second_request, second):
    """Return scale and offset from two requested XTest points and fresh core receipts.

    This is an observed local affine mapping, not toolkit geometry or screenshot mapping.
    A third independently requested anchor must validate it before gestures begin.
    """
    result = []
    for axis, key in enumerate(("core_x", "core_y")):
        delta = second_request[axis] - first_request[axis]
        a, b = first[1].get(key), second[1].get(key)
        if delta == 0 or any(type(v) not in (int, float) or not math.isfinite(v) for v in (a, b)):
            raise ValueError("degenerate stimulus calibration")
        scale = (b-a)/delta
        if not math.isfinite(scale) or not 0.25 <= scale <= 4:
            raise ValueError("implausible stimulus mapping")
        result.append((scale, a-scale*first_request[axis]))
    return tuple(result)


def settled_capture(rows, metadata, expected_page):
    """Join an exact raw-source dump to a subsequent applied GTK paint.

    This proves provenance at the toolkit paint boundary only. The caller still owes
    focus, visible unclipped ROI, screenshot dimensions and independent pixel mapping.
    """
    integer_keys = ("id", "generation", "document", "page", "width", "height", "source_index")
    for key in integer_keys:
        if type(metadata.get(key)) is not int or metadata[key] < 0:
            raise ValueError("invalid capture metadata integer")
    if metadata["page"] != expected_page or min(metadata["width"], metadata["height"]) < 4:
        raise ValueError("wrong capture page or empty dimensions")
    if metadata.get("format") != "RGBA8-straight":
        raise ValueError("unknown raw capture format")
    generation = metadata["generation"]
    if generation == 0 or not 1 <= metadata["id"] <= 16:
        raise ValueError("capture request or generation outside bounds")
    matching_dumps = [(i, row) for i, row in enumerate(rows)
                      if row.get("kind") == "raster-dump" and row.get("capture_id") == metadata["id"]]
    if len(matching_dumps) != 1 or matching_dumps[0][1].get("generation") != generation:
        raise ValueError("missing or ambiguous dump generation")
    dump_index = matching_dumps[0][0]
    beginnings = [i for i, row in enumerate(rows) if row.get("kind") == "refresh-begin"
                  and row.get("generation") == generation]
    if len(beginnings) != 1 or beginnings[0] >= dump_index:
        raise ValueError("dump has no unique preceding refresh")
    paints = [(i, row) for i, row in enumerate(rows) if row.get("kind") == "raster-paint"
              and row.get("generation") == generation and i > dump_index]
    if not paints:
        raise ValueError("dump has no subsequent matching paint")
    paint_index, paint = paints[-1]
    if paint.get("native_completed") is not True:
        raise ValueError("paint is not a native completed raster")
    for key in ("generation", "document", "page", "width", "height", "source_index"):
        if type(paint.get(key)) is not int or paint[key] != metadata[key]:
            raise ValueError("dump and paint metadata differ")
    for row in rows[dump_index+1:]:
        if row.get("kind") == "refresh-begin" and row.get("generation") != generation:
            raise ValueError("another refresh invalidated the captured paint")
    geometries = [row for row in rows[:paint_index+1] if row.get("kind") == "geometry"]
    if not geometries:
        raise ValueError("paint has no toolkit geometry")
    geometry = geometries[-1]
    for row, keys in ((paint, ("origin_x", "origin_y", "fixed_x_logical", "fixed_y_logical", "bounds_x_logical", "bounds_y_logical",
                              "bounds_width_logical", "bounds_height_logical")),
                      (geometry, ("scale", "content_width_logical", "content_height_logical"))):
        for key in keys:
            if type(row.get(key)) not in (int, float) or not math.isfinite(row[key]):
                raise ValueError("nonfinite or missing capture geometry")
    scale = geometry["scale"]
    if scale <= 0:
        raise ValueError("nonpositive toolkit scale")
    origin = metadata.get("origin")
    if not isinstance(origin, list) or len(origin) != 2:
        raise ValueError("capture origin is missing")
    roi = metadata.get("roi")
    if not isinstance(roi, list) or len(roi) != 4 or any(type(v) is not int for v in roi):
        raise ValueError("invalid raster ROI")
    x, y, width, height = roi
    if min(x, y, width, height) <= 0 or x+width >= metadata["width"] or y+height >= metadata["height"]:
        raise ValueError("ROI is not strictly inside raster")
    for axis, dimension, offset, extent in (("x", "width", x, width), ("y", "height", y, height)):
        at = 0 if axis == "x" else 1
        if paint[f"origin_{axis}"] != origin[at]:
            raise ValueError("capture and painted origin differ")
        if paint.get(f"extent_{dimension}") != metadata[dimension]:
            raise ValueError("paint scales the completed source")
        if abs(paint[f"bounds_{dimension}_logical"] * scale - metadata[dimension]) > 0.01:
            raise ValueError("actual widget extent differs from native raster")
        if abs(paint[f"bounds_{axis}_logical"] * scale - origin[at]) > 0.01:
            raise ValueError("actual widget origin differs from source placement")
        begin = paint[f"fixed_{axis}_logical"] + paint[f"bounds_{axis}_logical"] + offset / scale
        end = begin + extent / scale
        if begin < 0 or end > geometry[f"content_{dimension}_logical"]:
            raise ValueError("ROI is clipped by content viewport")
    return paint, geometry


def self_test():
    import unittest

    def row(kind="motion-entry", sequence=2, **fields):
        value = dict(v=1, host="gtk", pid=42, window=1, us=20,
                     kind=kind, sequence=sequence, logical_x=30, logical_y=30, action="moved", dpr=1)
        value.update(fields)
        return PREFIX + json.dumps(value).encode() + b"\n"

    class Tests(unittest.TestCase):
        def setUp(self):
            self.old = row(sequence=1, us=10)
            self.mark = Boundary(self.old, "s1", "gtk", 42, 1, 1, 10)
            self.good = self.old + row() + row("motion-outcome", outcome="accepted", core_x=45, core_y=45)

        def test_valid_entry_and_outcome_share_sequence(self):
            self.assertEqual(motion(self.good, self.mark, "s1", (30, 30))[0]["sequence"], 2)

        def test_invalid_motion_populations(self):
            invalid = {
                "stale": self.old,
                "dragged": self.good.replace(b'"moved"', b'"dragged"'),
                "reused": self.old + row(sequence=1),
                "duplicate": self.good + row(),
                "missing outcome": self.old + row(),
                "orphan outcome": self.good + row("motion-outcome", 3, outcome="accepted"),
                "missing sequence": self.good.replace(b'"sequence": 2, ', b''),
                "nonfinite core": self.good.replace(b'"core_x": 45', b'"core_x": NaN'),
                "nonfinite logical": self.good.replace(b'"logical_x": 30', b'"logical_x": Infinity'),
                "boolean core": self.good.replace(b'"core_x": 45', b'"core_x": true'),
                "missing core": self.good.replace(b'"core_x": 45, ', b''),
                "foreign window": self.old + row(window=2),
                "reset": self.good[len(self.old):],
                "rewrite": self.old.replace(b'10', b'11') + self.good[len(self.old):],
                "partial": self.good[:-1],
                "reversed time": self.old + row(us=9),
                "multiple callbacks": self.good + row(sequence=3),
                "suppressed": self.good.replace(b'accepted', b'host-busy'),
                "different position": self.good.replace(b'30', b'31'),
            }
            for label, data in invalid.items():
                with self.subTest(label=label), self.assertRaises(ValueError):
                    motion(data, self.mark, "s1", (30, 30))

        def test_session_and_bounds(self):
            with self.assertRaises(ValueError): records(self.good, self.mark, "s2")
            with self.assertRaises(ValueError): records(b"x"*(LIMIT+1), self.mark, "s1")

        def test_fractional_mapping_and_equal_coordinates(self):
            first = motion(self.good, self.mark, "s1", (30, 30))
            second = (dict(logical_x=50, logical_y=50), dict(core_x=75, core_y=75))
            self.assertEqual(logical_to_core_mapping(first, second), (1.5, 1.5))
            with self.assertRaises(ValueError): logical_to_core_mapping(first, first)

        def test_wheel_modifier_busy_and_sub_notch_remain_visible(self):
            data = self.old
            for seq, ctrl, amount, outcome in [(2, False, 120, "accepted"), (3, True, 0.1, "accepted"), (4, True, 120, "host-busy")]:
                data += row("wheel-entry", seq, ctrl=ctrl)
                data += row("wheel-branch", seq, branch="zoom" if ctrl else "scroll", amount=amount, pixels=False)
                if ctrl and outcome == "accepted":
                    data += row("dispatch-begin", seq, operation="zoom")
                    data += PREFIX + json.dumps(dict(v=1, host="gtk", pid=42, kind="zoom-accumulator", amount=amount, unit="notches", steps=0, core_x=30, core_y=30)).encode() + b"\n"
                    data += row("dispatch-end", seq, operation="zoom")
                data += row("wheel-outcome", seq, outcome=outcome)
            received = wheel(data, self.mark, "s1")
            for bad in (data.replace(b'"core_x": 30', b'"core_x": 31'), data.replace(b'"unit": "notches"', b'"unit": "pixels"')):
                with self.assertRaises(ValueError): wheel(bad, self.mark, "s1")
            self.assertEqual(len(received), 3)
            self.assertFalse(received[0][0]["ctrl"])
            self.assertEqual(received[1][1]["amount"], 0.1)
            self.assertEqual(received[2][2]["outcome"], "host-busy")

        def test_accumulator_spans_reject_missing_duplicate_and_orphan_records(self):
            accumulator = PREFIX + json.dumps(dict(v=1, host="gtk", pid=42, kind="zoom-accumulator", amount=0.1, unit="notches", steps=0, core_x=30, core_y=30)).encode() + b"\n"
            begin = row("dispatch-begin", operation="zoom")
            end = row("dispatch-end", operation="zoom")
            for tail in (accumulator, begin+end, begin+accumulator+accumulator+end, begin+accumulator, begin+begin+accumulator+end, begin+row("dispatch-begin", window=2, operation="zoom")+accumulator+end):
                with self.assertRaises(ValueError): records(self.old+tail, self.mark, "s1")

        def test_stimulus_mapping_keeps_offsets_separate_from_logical_scale(self):
            first = (dict(logical_x=30, logical_y=30), dict(core_x=45, core_y=45))
            second = (dict(logical_x=50, logical_y=50), dict(core_x=75, core_y=75))
            self.assertEqual(stimulus_to_core_mapping((100, 200), first, (120, 220), second), ((1.5, -105.0), (1.5, -255.0)))
            with self.assertRaises(ValueError): stimulus_to_core_mapping((100, 200), first, (100, 200), second)

        def test_ctrl_false_cannot_be_a_zoom_branch(self):
            tail = row("wheel-entry", ctrl=False)+row("wheel-branch", branch="zoom", amount=1)+row("wheel-outcome", outcome="host-busy")
            with self.assertRaises(ValueError): wheel(self.old+tail, self.mark, "s1")

        def test_orphan_and_reordered_wheel(self):
            for tail in (row("wheel-outcome"), row("wheel-branch", branch="delegate") + row("wheel-entry")):
                with self.assertRaises(ValueError): wheel(self.old+tail, self.mark, "s1")

        def test_settled_capture_requires_matching_source_and_paint(self):
            import copy
            check = settled_capture
            meta=dict(id=1,generation=5,document=2,page=19,width=100,height=120,source_index=0,
                      origin=[10,20],roi=[25,30,50,60],format='RGBA8-straight')
            rows=[dict(kind='refresh-begin',generation=5),dict(kind='raster-dump',capture_id=1,generation=5),
                  dict(kind='geometry',scale=2,content_width_logical=100,content_height_logical=100),
                  dict(kind='raster-paint',generation=5,document=2,page=19,width=100,height=120,
                       native_completed=True,source_index=0,origin_x=10,origin_y=20,extent_width=100,extent_height=120,
                       fixed_x_logical=0,fixed_y_logical=0,bounds_x_logical=5,bounds_y_logical=10,
                       bounds_width_logical=50,bounds_height_logical=60)]
            check(rows,meta,19)
            cases=[]
            def change_row(index,key,value):
                value_rows=copy.deepcopy(rows);value_rows[index][key]=value
                return value_rows,copy.deepcopy(meta)
            cases += [change_row(3,'native_completed',False),change_row(3,'source_index',1),
                      change_row(3,'document',3),change_row(3,'generation',6),
                      change_row(3,'bounds_width_logical',49),change_row(3,'origin_x',float('nan')),
                      change_row(3,'fixed_x_logical',200),change_row(2,'scale',0),
                      change_row(3,'bounds_x_logical',6),change_row(3,'extent_width',99)]
            cases += [(rows+[dict(kind='refresh-begin',generation=6)],meta),(rows[:3],meta),
                      (rows+[rows[1]],meta)]
            for key,value in [('roi',[0,30,50,60]),('page',20),('width',False),('format','RGB'),('source_index',None)]:
                altered=copy.deepcopy(meta);altered[key]=value;cases.append((rows,altered))
            cases += [(rows[1:],meta),(rows+[rows[0]],meta)]
            for key,value in [("id",17),("generation",0)]:
                altered=copy.deepcopy(meta);altered[key]=value;cases.append((rows,altered))
            for index,(altered_rows,altered_meta) in enumerate(cases):
                try: check(altered_rows,altered_meta,19)
                except ValueError: pass
                else: raise AssertionError(f'negative {index} was accepted')

    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Tests)
    if suite.countTestCases() != 10:
        raise RuntimeError("zoom receipt calibration requires all ten tests")
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if result.wasSuccessful():
        print("zoom records: every case holds")
    raise SystemExit(not result.wasSuccessful())


if __name__ == "__main__":
    self_test()
