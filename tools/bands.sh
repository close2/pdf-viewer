#!/usr/bin/env bash
#
# Which machine this is, and its clock bands: found, or taken.
#
# The three clock gates (`t2-launch_path`, `t2-turn_path`, `t2-toolkit_launch`) judge a clock
# figure against a band that is a claim about one machine. Round 1914 measured whether those bands
# could instead be factors of a few baseline probes the gate takes itself, and they cannot: within
# every class (processor, disk, graphics device) the figures moved between machines by ratios
# whose largest is more than one and a half times their smallest, so no single probe of a class
# places every figure within a fifth of it (ADR 2662, `doc/history/1914-*.md`). So the bands stay
# absolute and are kept per machine, under `doc/checks/machines/<name>/`, found by a fingerprint,
# and a machine with none takes its own here instead of in a round.
#
#   tools/bands.sh fingerprint      print this machine's fingerprint, one line
#   tools/bands.sh which            print this machine's band directory, or say there is none (exit 1)
#   tools/bands.sh retake [--runs N] [--gates "launch turn toolkit"] [--again] [--round R]
#                                   take this machine's bands, unless it has them (`--again` takes
#                                   them over); every run of every gate is its own `--clock` hold,
#                                   and the GPU lock is the harness's own, taken around each device
#                                   section and never by this script (ADR 2664)
#   tools/bands.sh write <logdir>   write the bands from a retake's logs (what `retake` ends with)
#
# **The fingerprint** is the processor's model, how many processors this process may run on, and the
# adapter the gate draws on as its own bring-up names it — name, type and backend. Each moves the
# clock figures (ADR 2662's table) and none is something the afternoon moves. It is the adapter
# wgpu chose and not the one the environment meant: `VK_ICD_FILENAMES` naming a file that is not
# there leaves the Vulkan loader with no driver and wgpu draws through OpenGL on the same card,
# which is a different device with different bands (every band of round 1913 was taken that way).
# The driver's version is not in it, because the adapter's description does not carry it: a driver
# upgrade that moves the figures shows as a red gate, and `retake --again` is the answer.
#
# **The rule** is ADR 1969's over every run of the retake: the smallest × 0.85 to the largest × 1.20,
# rounded outward. The disk probes take theirs by ADR 2660's shape, the smallest less a sixth to the
# median × 2.5, over every cold sample; a cold first page takes its row's warm band (ADR 2158).
# A figure no run printed (a declined child, every run over the load ceiling) is written `none`,
# and the file says so beside it.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
machines="$here/doc/checks/machines"
# Every command here builds or runs a gate, so the whole script holds the agent's task budget.
task_budget=$("$here/tools/bounded.sh" --task-budget)
ulimit -u "$task_budget"

# The fingerprint is the harness's own (`machine_bands::fingerprint`), because the device in it is
# the adapter wgpu chose, which the environment does not say: asked of `vulkaninfo`, it named
# lavapipe where the gate drew through OpenGL on the RX 580. So it is the turn gate's test binary,
# built and asked; its bring-up takes the shared adapter's lock for itself (ADR 2664).
fingerprint() {
    (cd "$here" && cargo test --release -q -p render-raster --test turn_path --no-run) >/dev/null 2>&1 || {
        echo "bands: the turn gate's test binary does not build" >&2
        return 1
    }
    local said
    said=$(cd "$here" && cargo test --release -q -p render-raster --test turn_path -- \
        --ignored --exact this_machine_s_fingerprint --nocapture 2>/dev/null |
        sed -n 's/^bands: fingerprint //p')
    [ -n "$said" ] || { echo "bands: the turn gate printed no fingerprint" >&2; return 1; }
    printf '%s\n' "$said"
}

which_dir() {
    local print=${1:-} dir
    [ -n "$print" ] || print=$(fingerprint) || return 1
    for dir in "$machines"/*/; do
        [ -f "$dir/fingerprint" ] || continue
        if [ "$(cat "$dir/fingerprint")" = "$print" ]; then
            printf '%s\n' "${dir%/}"
            return 0
        fi
    done
    printf 'bands: no band directory for %s\n' "$print" >&2
    return 1
}

# The cargo arguments of one gate, as `tools/batch.sh gates` runs it, into `args`; the workers it
# needs built beside it into `build`; the environment its clocks are taken under into `clocks`.
gate_args() {
    build=() clocks=(PDFVIEWER_BANDS_RETAKE=1)
    case $1 in
        launch) args=(-p viewer-ui --test launch_path)
                build=(--build '--release -p pdf-sandbox --bins'
                       --build '--release -p pdf-script-worker --features engine --bins')
                clocks+=(PDFVIEWER_LAUNCH_CLOCKS=1) ;;
        turn) args=(-p render-raster --test turn_path) ;;
        toolkit) args=(--no-fail-fast -p viewer-qt -p viewer-gtk --test launch_path) ;;
        *) echo "bands: no gate called $1" >&2; return 2 ;;
    esac
}

# The variables that choose the driver stack a launch loads rather than leaving it to the loader.
pinning_vars=(WGPU_BACKEND VK_ICD_FILENAMES VK_DRIVER_FILES VK_ADD_DRIVER_FILES
              VK_LOADER_DRIVERS_SELECT VK_LOADER_DRIVERS_DISABLE)

# **A device an unpinned launch takes is banded unpinned** (ADR 2730). The launch gate measures the
# unfiltered launch, so that its band is a claim about the binary and not about a launcher
# (`doc/questions/A270`), and every driver the loader finds is in its bring-up. So a retake under
# any pinning variable asks the fingerprint again without them, and refuses where the device is
# the same one: the pinning chose nothing and only took drivers out of the bring-up. A device the
# loader does not pick by itself — lavapipe, or the card through OpenGL — is reached only pinned,
# and is taken so.
unpinned_takes_it() {
    local print=$1 var set=() unset=() bare
    for var in "${pinning_vars[@]}"; do
        if [ -n "${!var+x}" ]; then
            set+=("$var=${!var}")
            unset+=(-u "$var")
        fi
    done
    [ ${#set[@]} -gt 0 ] || return 0
    bare=$(env "${unset[@]}" "$here/tools/bands.sh" fingerprint) || return 1
    if [ "$bare" = "$print" ]; then
        echo "bands: ${set[*]} pins the device an unpinned launch takes anyway ($print);" \
            "its bands are taken unpinned, as the gate runs (ADR 2730)" >&2
        return 1
    fi
}

retake() {
    local runs=3 gates="launch turn toolkit" again=0 round=bands
    while [ $# -gt 0 ]; do
        case $1 in
            --runs) runs=$2; shift 2 ;;
            --gates) gates=$2; shift 2 ;;
            --again) again=1; shift ;;
            --round) round=$2; shift 2 ;;
            *) echo "bands: unknown argument $1" >&2; return 2 ;;
        esac
    done
    local print existing logs
    print=$(fingerprint) || return 1
    unpinned_takes_it "$print" || return 1
    if existing=$(which_dir "$print" 2>/dev/null) && [ "$again" = 0 ]; then
        echo "bands: $existing holds this machine's bands already ($print); --again takes them over"
        return 0
    fi
    logs="$HOME/scratch/bands/$(date +%Y%m%dT%H%M%S)"
    mkdir -p "$logs"
    echo "bands: re-taking for $print, $runs run(s) of: $gates; logs in $logs"
    local run gate
    # Each gate's test binary is built once, before any run, and the same `-p` set is what keeps
    # Cargo's feature resolution, and so the binary, the one the runs use. No run is wrapped in the
    # GPU lock: each harness takes it around its own device sections and releases it before its
    # processor and disk work, so a neighbour's device work interleaves (ADR 2664).
    for gate in $gates; do
        gate_args "$gate" || return 2
        (cd "$here" && "$here/tools/bounded.sh" --lock --round "$round" --tree 24 "${build[@]}" -- \
            cargo test --release --no-run "${args[@]}") > "$logs/$gate-build.log" 2>&1 || {
            echo "bands: $gate does not build; $logs/$gate-build.log" >&2
            return 1
        }
    done
    for run in $(seq 1 "$runs"); do
        for gate in $gates; do
            gate_args "$gate"
            echo "bands: $gate run $run, load $(cut -d' ' -f1-3 /proc/loadavg)"
            # The gate's own exit is not the retake's: a counted figure may be red on this machine
            # (ADR 2660), and the clock figures it printed are what is read.
            (cd "$here" && GPU_LOCK_TIMEOUT="${GPU_LOCK_TIMEOUT:-1200}" PDFVIEWER_BANDS_RETAKE=1 \
                "$here/tools/bounded.sh" --lock --clock --round "$round" --tree 12 -- \
                env "${clocks[@]}" cargo test --release "${args[@]}" -- --ignored --nocapture) \
                > "$logs/$gate-$run.log" 2>&1 || true
        done
    done
    write "$logs"
}

write() {
    local logs=$1 print name dir
    # Every run said the fingerprint it judged under; they must be one machine.
    print=$(sed -n 's/^.*bands: fingerprint //p' "$logs"/*-[0-9]*.log | sort -u)
    if [ -z "$print" ] || [ "$(printf '%s\n' "$print" | wc -l)" != 1 ]; then
        echo "bands: the runs in $logs do not name one machine: ${print:-none}" >&2
        return 1
    fi
    printf '%s\n' "$print" > "$logs/fingerprint"
    if dir=$(which_dir "$print" 2>/dev/null); then
        :
    else
        # A readable name, and the fingerprint's hash so that two machines alike in the first two
        # never share a directory.
        name=$(printf '%s' "$print" | sed -E 's/processor=//; s/; processors=/-/; s/; device=/-/;
                s/AMD |Intel\(R\) |\(TM\)|[0-9]+-Core Processor| Series| Graphics//g' |
            tr 'A-Z' 'a-z' | sed -E 's/[^a-z0-9]+/-/g; s/^-//' | cut -c1-56)
        dir="$machines/${name%-}-$(printf '%s' "$print" | sha256sum | cut -c1-8)"
    fi
    PYTHONDONTWRITEBYTECODE=1 python3 "$here/tools/bands.py" --logs "$logs" --template "$(ls -d "$machines"/*/ | head -1)" \
        --out "$dir"
    printf '%s\n' "$print" > "$dir/fingerprint"
    echo "bands: wrote $dir"
}

case ${1:-} in
    fingerprint) fingerprint ;;
    which) which_dir ;;
    retake) shift; retake "$@" ;;
    write) shift; write "$@" ;;
    *) sed -n '2,/^set -euo/p' "$0" | sed '$d'; exit 2 ;;
esac
