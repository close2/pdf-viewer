#!/usr/bin/env python3
"""Finite owned-window input/geometry probe; no performance or sharpness verdict."""
import argparse
import importlib.util
import json
import os
import re
import signal
from pathlib import Path
import subprocess
import sys
import time


def load_parser(path):
    spec = importlib.util.spec_from_file_location("zoom_records", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def run(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True, timeout=4).stdout.strip()


def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument("--host", choices=("gtk", "qt"), required=True)
    args.add_argument("--binary", type=Path, required=True)
    args.add_argument("--pdf", type=Path, required=True)
    args.add_argument("--output", type=Path, required=True)
    args.add_argument("--parser", type=Path, required=True)
    args.add_argument("--headless", action="store_true", help="owned Xvfb rehearsal; software graphics permitted")
    args.add_argument("--page", type=int, default=20)
    args.add_argument("--settled-capture", action="store_true", help="GTK whole-page source/presentation observations; no timing verdict")
    config = args.parse_args()
    if config.page < 1:
        args.error("--page must be positive")
    if config.settled_capture and config.host != "gtk":
        args.error("--settled-capture currently requires GTK")
    probe = load_parser(config.parser)
    config.output.mkdir(parents=True, exist_ok=False)
    log = config.output / "viewer.log"
    session = str(config.output.resolve())
    environment = dict(os.environ)
    for key in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET"):
        environment.pop(key, None)
    environment.update(PDFVIEWER_ZOOM_PROBE="1", GDK_BACKEND="x11", QT_QPA_PLATFORM="xcb")
    if config.headless:
        environment.update(QT_XCB_NO_XI2="1", GSK_RENDERER="cairo", LIBGL_ALWAYS_SOFTWARE="1")
    if config.settled_capture:
        capture_dir = config.output / "pixels"
        capture_dir.mkdir()
        environment["PDFVIEWER_ZOOM_CAPTURE_DIR"] = str(capture_dir.resolve())
    command = [str(config.binary.resolve()), "--trace=launch,frames,events", f"{config.pdf.resolve()}#page={config.page}&zoom=100"]
    if config.headless:
        command.insert(1, "--cpu")
    report = dict(session=session, host=config.host, command=command, headless=config.headless,
                  environment={key: environment.get(key) for key in ("GDK_BACKEND", "QT_QPA_PLATFORM", "GDK_SCALE", "QT_SCALE_FACTOR", "QT_XCB_NO_XI2", "GSK_RENDERER", "LIBGL_ALWAYS_SOFTWARE")},
                  verdict="incomplete", phases=[])
    process = None
    control_down = False
    window = None
    started = time.monotonic()
    def interrupted(signum, _frame):
        raise RuntimeError(f"probe interrupted by signal {signum}")
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    signal.signal(signal.SIGALRM, interrupted)
    signal.setitimer(signal.ITIMER_REAL, 140)

    def read():
        with log.open("rb") as file:
            data = file.read(probe.LIMIT+1)
        if len(data) > probe.LIMIT:
            raise RuntimeError("trace exceeds bounded read")
        if process.poll() is not None:
            raise RuntimeError(f"viewer exited {process.returncode}")
        if time.monotonic()-started > 140:
            raise RuntimeError("finite probe deadline")
        return data

    def structured(data):
        found = []
        complete = data if data.endswith(b"\n") else data[:data.rfind(b"\n")+1]
        for line in complete.splitlines():
            if line.startswith(probe.PREFIX):
                try:
                    record = json.loads(line[len(probe.PREFIX):])
                except (ValueError, UnicodeDecodeError) as error:
                    raise RuntimeError("malformed complete structured receipt") from error
                if not isinstance(record, dict) or record.get("v") != 1:
                    raise RuntimeError("unknown structured receipt schema")
                if record.get("host") == config.host and record.get("pid") == process.pid:
                    found.append(record)
        return found

    def boundary():
        data = read()
        if data and not data.endswith(b"\n"):
            raise RuntimeError("cannot snapshot a partial log line")
        rows = [r for r in structured(data) if r.get("window") == local_window]
        return probe.Boundary(data, session, config.host, process.pid, local_window,
                              max((r.get("sequence", 0) for r in rows), default=0),
                              max((r.get("us", 0) for r in rows), default=0))

    def focus(call=run):
        if config.headless:
            call("xdotool", "windowfocus", "--sync", window)
        else:
            call("xdotool", "windowactivate", "--sync", window)
            if call("xdotool", "getactivewindow") != window:
                raise RuntimeError("owned window is not active")
        # X input focus may be a child; permit it only when ancestry reaches our window.
        focused = call("xdotool", "getwindowfocus")
        if focused != window:
            for _ in range(16):
                tree = call("xwininfo", "-id", focused, "-tree")
                parent = re.search(r"Parent window id:\s*(0x[0-9a-fA-F]+)", tree)
                if parent is None:
                    raise RuntimeError("cannot establish focused window ancestry")
                focused = str(int(parent.group(1), 16))
                if focused == window:
                    break
                if focused == "0":
                    raise RuntimeError("input focus is outside owned window")
            else:
                raise RuntimeError("focused window ancestry exceeds bound")

    def diagnose(label):
        # Read-only X11 observations; they never substitute for fresh toolkit callbacks.
        entry = dict(label=label, elapsed_seconds=time.monotonic()-started)
        commands = {
            "pointer": ("xdotool", "getmouselocation", "--shell"),
            "active_window": ("xdotool", "getactivewindow"),
            "focused_window": ("xdotool", "getwindowfocus"),
        }
        if window is not None:
            commands.update(
                native_geometry=("xdotool", "getwindowgeometry", "--shell", window),
                frame_extents=("xprop", "-id", window, "_NET_FRAME_EXTENTS", "_GTK_FRAME_EXTENTS", "_NET_WM_PID"))
        for name, command in commands.items():
            try:
                result = subprocess.run(command, capture_output=True, text=True, timeout=2)
                entry[name] = dict(exit=result.returncode, stdout=result.stdout, stderr=result.stderr)
            except (OSError, subprocess.TimeoutExpired) as error:
                entry[name] = dict(error=str(error))
        report.setdefault("diagnostics", []).append(entry)

    def refusal_capture():
        if window is None or process is None or process.poll() is not None:
            return
        try:
            owner = run("xprop", "-id", window, "_NET_WM_PID")
            if not re.search(rf"=\s*{process.pid}\s*$", owner):
                raise RuntimeError("refusal screenshot window PID differs")
            image = config.output / "refusal-owned-window.xwd"
            subprocess.run(["xwd", "-id", window, "-silent", "-out", str(image)],
                           capture_output=True, timeout=2, check=True)
            report["refusal_screenshot"] = dict(path=str(image),
                boundary="owned X11 window/backing pixels only; rejected trial, no measurement verdict")
        except (OSError, RuntimeError, subprocess.SubprocessError) as error:
            report["refusal_screenshot_error"] = str(error)

    def move(request, label):
        focus()
        mark = boundary()
        diagnose(label+"-before-warp")
        report["diagnostics"][-1]["requested_window_point"] = list(request)
        run("xdotool", "mousemove", "--window", window, str(round(request[0])), str(round(request[1])))
        diagnose(label+"-after-warp")
        deadline = time.monotonic()+3
        problem = "no observation"
        while time.monotonic() < deadline:
            try:
                data = read()
                result = probe.motion(data, mark, session)
                core = re.findall(rb"Pointer \{ at: \(([-0-9.]+), ([-0-9.]+)\), action: Move", data[len(mark.prefix):])
                if len(core) != 1 or any(abs(float(core[0][axis])-result[1][key]) > 0.01 for axis, key in enumerate(("core_x", "core_y"))):
                    raise ValueError("no unique fresh core Pointer matching host-delivered coordinates")
                report["phases"].append(dict(name=label, requested=request, entry=result[0], outcome=result[1], boundary_bytes=len(mark.prefix)))
                return result
            except ValueError as error:
                problem = str(error)
                time.sleep(0.05)
        raise RuntimeError(f"{label}: {problem}")

    def capture_sequence():
        import hashlib
        capture_dir = config.output / "pixels"
        report["captures"] = []
        next_id = 0

        def fresh_native(mark, previous=None):
            deadline = time.monotonic()+12
            problem = "no completed page paint"
            while time.monotonic() < deadline:
                try:
                    rows = probe.records(read(), mark, session)
                    paints = [r for r in rows if r.get("kind") == "raster-paint"]
                    if not paints:
                        raise ValueError("no fresh raster paint")
                    paint = paints[-1]
                    size = (paint.get("width"), paint.get("height"))
                    if paint.get("native_completed") is not True or paint.get("page") != config.page-1:
                        raise ValueError("paint is not the requested completed page")
                    if previous is not None and size == previous:
                        raise ValueError("zoom has not changed raster dimensions")
                    if any(r.get("kind") == "refresh-begin" and r.get("generation", 0) > paint["generation"] for r in rows):
                        raise ValueError("newer refresh has not painted")
                    return paint
                except ValueError as error:
                    problem = str(error)
                    time.sleep(0.05)
            raise RuntimeError(problem)

        def checkpoint(label, expected):
            nonlocal next_id
            problems = []
            for attempt in range(3):
                next_id += 1
                focus()
                mark = boundary()
                request = capture_dir / "request"
                if request.exists():
                    raise RuntimeError("prior capture request was not consumed")
                temporary = capture_dir / f"request-{next_id}.tmp"
                temporary.write_text(str(next_id)+"\n")
                temporary.rename(request)
                # A real pointer event requests an ordinary refresh after the settled paint.
                # It is outside the body ROI and excluded from all latency measurements.
                point = (round((x+8+next_id)*seed_x), round((y+8+next_id)*seed_y))
                run("xdotool", "mousemove", "--window", window, str(point[0]), str(point[1]))
                deadline = time.monotonic()+6
                metadata_path = capture_dir / f"capture-{next_id}.json"
                problem = "no dump and matching paint"
                while time.monotonic() < deadline:
                    try:
                        if not metadata_path.exists():
                            raise ValueError("dump metadata not ready")
                        metadata = json.loads(metadata_path.read_text())
                        rows = probe.records(read(), mark, session)
                        paint, capture_geometry = probe.settled_capture(rows, metadata, config.page-1)
                        for key in ("document", "page", "width", "height", "origin_x", "origin_y"):
                            if paint.get(key) != expected.get(key):
                                raise ValueError("settled phase changed before requested capture")
                        focus()
                        image_path = config.output / f"{label}-{next_id}.xwd"
                        run("xwd", "-id", window, "-silent", "-out", str(image_path))
                        # Prove no intervening refresh invalidated the source/paint join during capture.
                        probe.settled_capture(probe.records(read(), mark, session), metadata, config.page-1)
                        png_path = image_path.with_suffix(".png")
                        run("magick", "xwd:"+str(image_path), str(png_path))
                        dimensions = run("magick", "identify", "-format", "%w %h", str(png_path))
                        raw_path = capture_dir / f"capture-{next_id}.rgba"
                        raw = raw_path.read_bytes()
                        if len(raw) != metadata["width"]*metadata["height"]*4:
                            raise RuntimeError("raw source byte count differs from metadata")
                        report["captures"].append(dict(label=label, id=next_id, metadata=metadata,
                            paint=paint, geometry=capture_geometry, native_geometry=run("xdotool", "getwindowgeometry", "--shell", window),
                            screenshot_dimensions=dimensions, screenshot=str(png_path),
                            source_sha256=hashlib.sha256(raw).hexdigest(),
                            screenshot_sha256=hashlib.sha256(png_path.read_bytes()).hexdigest(),
                            capture_boundary="X11 window/backing pixels via xwd; not compositor scanout",
                            mapping_verdict="unjudged: independent page-edge or grid check required"))
                        return
                    except (ValueError, json.JSONDecodeError) as error:
                        problem = str(error)
                        if "invalidated" in problem or "phase changed" in problem:
                            break
                        time.sleep(0.05)
                if request.exists() and request.read_text().strip() == str(next_id):
                    request.unlink()
                problems.append(dict(id=next_id, problem=problem))
                report.setdefault("rejected_capture_attempts", []).append(problems[-1])
            raise RuntimeError(f"{label}: no accepted capture in three attempts: {problems}")

        # Use the existing user-facing layout action, verified by the core trace.
        for _ in range(6):
            focus()
            mark = boundary()
            run("xdotool", "key", "l")
            time.sleep(0.25)
            data = read()
            if data.startswith(mark.prefix) and b"Layout(SinglePage)" in data[len(mark.prefix):]:
                break
        else:
            raise RuntimeError("could not verify SinglePage layout")
        focus()
        mark = boundary()
        run("xdotool", "key", "0")
        initial = fresh_native(mark)
        checkpoint("fit", initial)
        previous = initial
        for index in range(2):
            focus()
            mark = boundary()
            run("xdotool", "key", "minus")
            painted = fresh_native(mark, (previous["width"], previous["height"]))
            if painted["width"] >= previous["width"] or painted["height"] >= previous["height"]:
                raise RuntimeError("zoom-out did not reduce both raster dimensions")
            checkpoint(f"zoom-out-{index+1}", painted)
            previous = painted
        focus()
        mark = boundary()
        run("xdotool", "key", "0")
        restored = fresh_native(mark, (previous["width"], previous["height"]))
        if (restored["width"], restored["height"]) != (initial["width"], initial["height"]):
            raise RuntimeError("return to fit did not restore initial raster dimensions")
        checkpoint("fit-return", restored)
        report["verdict"] = "settled-source-and-X11-captures-observed"
        report["limits"] = "No latency or native-wheel claim; screenshot mapping and blur localization require independent image analysis; X11 capture does not observe later compositor scaling."

    try:
        with log.open("wb") as output:
            process = subprocess.Popen(command, env=environment, stdout=output, stderr=subprocess.STDOUT)
        report["pid"] = process.pid
        deadline = time.monotonic()+25
        geometry = None
        viewport = None
        page_ready = False
        while time.monotonic() < deadline:
            rows = structured(read())
            geometries = [r for r in rows if r.get("kind") == "geometry"]
            viewports = [r for r in rows if r.get("kind") == "core-viewport"]
            visible = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(process.pid)], capture_output=True, text=True, timeout=4)
            windows = visible.stdout.split()
            if len(windows) == 1 and geometries and viewports:
                window = windows[0]
                geometry = geometries[-1]
                viewport = viewports[-1]
                if viewport.get("width_device", 0) > 100 and viewport.get("height_device", 0) > 100:
                    if re.search(rf"page {config.page} retained,", read().decode(errors="replace")):
                        page_ready = True
                        break
            time.sleep(0.1)
        if not page_ready:
            raise RuntimeError("requested page was not retained before readiness deadline")
        report["page_ready"] = config.page
        if window is None or geometry is None or viewport is None:
            raise RuntimeError("no unique owned window with positive geometry and viewport")
        pid_property = run("xprop", "-id", window, "_NET_WM_PID")
        if not re.search(rf"=\s*{process.pid}\s*$", pid_property):
            raise RuntimeError("native window PID differs")
        local_window = geometry["window"]
        if viewport["window"] != local_window:
            raise RuntimeError("geometry and viewport identify different windows")
        backend = geometry.get("backend")
        if backend not in ("xcb", "GdkX11Display"):
            raise RuntimeError(f"requested X11 but actual backend is {backend!r}")
        report.update(native_window=window, local_window=local_window, geometry=geometry, viewport=viewport)
        report["native_geometry"] = run("xdotool", "getwindowgeometry", "--shell", window)
        if not config.headless:
            trace = read().decode(errors="replace").lower()
            if any(name in trace for name in ("llvmpipe", "lavapipe", "software adapter")):
                raise RuntimeError("software adapter is not hardware evidence")
            if not any(name in trace for name in ("radv", "amd radeon", "radeon 890m")):
                raise RuntimeError("no expected hardware adapter receipt")
        focus()
        # Layout is a toolkit fact. XTest mapping is then measured independently.
        x, y = geometry["content_x_logical"], geometry["content_y_logical"]
        width, height = geometry["content_width_logical"], geometry["content_height_logical"]
        if any(type(v) not in (int, float) for v in (x, y, width, height)) or min(width, height) < 120:
            raise RuntimeError("content geometry unavailable or too small")
        native = dict(line.split("=", 1) for line in report["native_geometry"].splitlines() if "=" in line)
        seed_x = int(native["WIDTH"]) / geometry["client_width_logical"]
        seed_y = int(native["HEIGHT"]) / geometry["client_height_logical"]
        # Native/client ratio only seeds an inside-content guess, never the accepted input map.
        # Fresh callback coordinates and the independent third point must still validate it.
        requests = [(round((x+width*f)*seed_x), round((y+height*f)*seed_y)) for f in (0.25, 0.4, 0.55)]
        # A preliminary distinct move drains enter/focus events; it is not calibration evidence.
        run("xdotool", "mousemove", "--window", window, str(round((x+width*0.15)*seed_x)), str(round((y+height*0.15)*seed_y)))
        time.sleep(0.3)
        first = move(requests[0], "calibration-one")
        second = move(requests[1], "calibration-two")
        transform = probe.stimulus_to_core_mapping(requests[0], first, requests[1], second)
        third = move(requests[2], "independent-anchor-check")
        for axis, key in enumerate(("core_x", "core_y")):
            predicted = transform[axis][0]*requests[2][axis]+transform[axis][1]
            if abs(third[1][key]-predicted) > 1:
                raise RuntimeError("third point does not validate stimulus mapping")
        report["stimulus_to_core"] = transform
        report["logical_to_core"] = probe.logical_to_core_mapping(first, second)
        if config.settled_capture:
            capture_sequence()
            return
        focus()
        mark = boundary()
        control_down = True
        run("xdotool", "keydown", "ctrl")
        for _ in range(4):
            focus()
            run("xdotool", "click", "4")
            time.sleep(0.2)
        run("xdotool", "keyup", "ctrl")
        control_down = False
        time.sleep(0.5)
        tail = read()
        if config.host == "qt":
            callbacks = probe.wheel(tail, mark, session)
            report["wheel_callbacks"] = callbacks
            accepted = sum(branch["branch"] == "zoom" and outcome is not None and outcome["outcome"] == "accepted" for _, branch, outcome in callbacks)
            report["requested_wheel_pulses"] = 4
            report["accepted_zoom_callbacks"] = accepted
            if len(callbacks) != 4 or accepted != 4:
                raise RuntimeError(f"incomplete wheel delivery: {len(callbacks)} callbacks, {accepted} accepted zooms for4pulses")
        else:
            fresh = tail[len(mark.prefix):]
            if not tail.startswith(mark.prefix) or b"Zoom {" not in fresh:
                raise RuntimeError("GTK wheel phase has no fresh core Zoom receipt")
            report["gtk_wheel_evidence"] = "fresh core command; raw scroll callback census unavailable"
        fresh = tail[len(mark.prefix):]
        zoom_count = len(re.findall(rb"Zoom \{", fresh))
        report["core_zoom_commands"] = zoom_count
        if zoom_count != 4:
            raise RuntimeError(f"expected4discrete wheel Zoom commands, observed{zoom_count}")
        report["phases"].append(dict(name="four-control-wheel-pulses", boundary_bytes=len(mark.prefix)))
        report["verdict"] = "input-and-geometry-observed"
        report["limits"] = "No matched viewport/DPR, latency, settled sharpness, drift or native Wayland conclusion."
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
        diagnose("trial-refused")
        refusal_capture()
        raise
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        if control_down:
            try:
                subprocess.run(["xdotool", "keyup", "ctrl"], timeout=4, check=False)
            except (OSError, subprocess.TimeoutExpired) as error:
                report["modifier_cleanup_error"] = str(error)
        if process is not None and process.poll() is None:
            cleanup_deadline = time.monotonic()+12

            def cleanup_call(*arguments):
                remaining = cleanup_deadline-time.monotonic()
                if remaining <= 0:
                    raise RuntimeError("owned-window cleanup deadline")
                return subprocess.run(arguments, check=True, capture_output=True, text=True,
                                      timeout=min(2, remaining)).stdout.strip()

            try:
                if window is None:
                    raise RuntimeError("no identified owned window for Escape")
                def verify_owner():
                    owner = cleanup_call("xprop", "-id", window, "_NET_WM_PID")
                    if not re.search(rf"=\s*{process.pid}\s*$", owner):
                        raise RuntimeError("cleanup window PID differs")
                verify_owner()
                focus(cleanup_call)
                verify_owner()
                # Target our verified window explicitly, even if focus changes after the check.
                cleanup_call("xdotool", "key", "--window", window, "Escape")
                report["cleanup_escape"] = "sent to verified owned window"
                process.wait(timeout=5)
            except (OSError, RuntimeError, subprocess.SubprocessError) as error:
                report["cleanup_escape_fallback"] = str(error)
            if process.poll() is None:
                report["cleanup_termination"] = "TERM fallback"
                try:
                    process.terminate()
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    report["cleanup_termination"] = "KILL after TERM deadline"
                    process.kill()
                    process.wait(timeout=3)
        report["elapsed_seconds"] = time.monotonic()-started
        (config.output/"receipt.json").write_text(json.dumps(report, indent=2)+"\n")


if __name__ == "__main__":
    main()
