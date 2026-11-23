#!/usr/bin/env bash
# The sound path measured in use — RFC 0015 section 9's audio rows, A946's condition for video
# (ADR 3441).
#
#   tools/audio-in-use.sh [<out-dir>]          # the whole run: about fifteen minutes of wall time
#   AUDIO_IN_USE_LONG_SECONDS=5 tools/audio-in-use.sh   # a dry run of the same, a minute and a half
#   AUDIO_IN_USE_ROUTE=loopback tools/audio-in-use.sh <out-dir>   # on ALSA's loopback card (ADR 3459)
#
# Builds the sound helper and the media worker in the `gates` profile (release-grade optimisation,
# thin LTO), makes the AAC LC clip with `ffmpeg`'s own encoder and fetches ISO/IEC 14496-26's two
# HE-AAC streams, pinned by hash, from ISO's Publicly Available Standards (never committed), then
# runs `crates/viewer-host/tests/audio_in_use.rs` in the same profile. That file builds the device
# it plays on — ALSA's `file` plugin over `null`, writing into a named pipe read at 48 kHz on the
# monotonic clock — because this machine's user may open no sound card, and says what such a
# device does not measure. With `AUDIO_IN_USE_ROUTE=loopback` the device is `snd-aloop`'s card
# instead — the helper plays into `hw:Loopback,0,0` and `ffmpeg` captures `hw:Loopback,1,0` — a
# kernel device with a driver's period and buffer; the run goes through `sg audio` where this shell
# does not carry the group the card's nodes belong to. The readings are printed and kept in
# `<out-dir>/readings.txt`.
#
# A tool and not a gate: its readings are minutes long, read against the budgets RFC 0015 states,
# and a dummy device's numbers are not a speaker's; nothing here fails on a number.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(dirname "$here")
# shellcheck source=tools/places.sh
. "$here/places.sh"
out=${1:-$PDF_VIEWER_AGENT/audio-in-use}
target=${CARGO_TARGET_DIR:-$root/target}
mkdir -p "$out/streams"
task_budget=$("$here/bounded.sh" --task-budget)
ulimit -u "$task_budget"

command -v ffmpeg >/dev/null || { echo "audio-in-use: ffmpeg makes the AAC LC clip and is missing" >&2; exit 1; }

# ISO/IEC 14496-26's HE-AAC (SBR) and HE-AAC v2 (SBR and parametric stereo) streams.
base=https://standards.iso.org/ittf/PubliclyAvailableStandards/ISO_IEC_14496-26_2010_Bitstreams/DVD1/mpeg4audio-conformance/compressedMp4
pins=(
    "al_sbr_cm_48_2 7e7c0390c950eda21775d1a193e05ee703257bed85602884e7d30a8b3c65be90"
    "al_sbr_ps_01_new 4d1005347349dd2ba34b40261a5c80dfac8994764a16b1283fcd003b780db6ea"
)
for pin in "${pins[@]}"; do
    read -r name sum <<<"$pin"
    file="$out/streams/$name.mp4"
    if ! echo "$sum  $file" | sha256sum --check --status 2>/dev/null; then
        curl -sSf -m 120 -o "$file.partial" "$base/$name.mp4"
        echo "$sum  $file.partial" | sha256sum --check --status \
            || { echo "audio-in-use: $name.mp4 does not hash to its pin" >&2; rm -f "$file.partial"; exit 1; }
        mv "$file.partial" "$file"
    fi
done
# AAC LC: thirty seconds of a 440 Hz tone, 48 kHz stereo, in an MP4 file.
lc="$out/streams/aac_lc_48_2_30s.mp4"
[ -s "$lc" ] || ffmpeg -hide_banner -loglevel error -y -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=30" \
    -ac 2 -c:a aac -profile:a aac_low -b:a 128k "$lc"

export CARGO_INCREMENTAL=0
cargo build --profile gates -p quorra-sound -p pdf-media-worker --bins
export AUDIO_IN_USE_HELPER="$target/gates/quorra-sound"
export AUDIO_IN_USE_WORKER="$target/gates/pdf-media-worker"
export AUDIO_IN_USE_DIR="$out/device"
export AUDIO_IN_USE_CLIPS="AAC LC 48 kHz stereo (ffmpeg's encoder, 30 s)=$lc;HE-AAC al_sbr_cm_48_2 (ISO/IEC 14496-26)=$out/streams/al_sbr_cm_48_2.mp4;HE-AAC v2 al_sbr_ps_01_new (ISO/IEC 14496-26)=$out/streams/al_sbr_ps_01_new.mp4"
export AUDIO_IN_USE_ROUTE=${AUDIO_IN_USE_ROUTE:-pipe}
run=(cargo test --profile gates -p viewer-host --test audio_in_use -- --ignored --nocapture --test-threads 1)
if [ "$AUDIO_IN_USE_ROUTE" = loopback ]; then
    grep -q '^Loopback' <(sed -n 's/^ *[0-9]* \[\([A-Za-z]*\) *\].*/\1/p' /proc/asound/cards) \
        || { echo "audio-in-use: no loopback card (modprobe snd-aloop, as root)" >&2; exit 1; }
    if ! id -Gn | tr ' ' '\n' | grep -qx audio; then
        run=(sg audio -c "$(printf '%q ' "${run[@]}")")
    fi
fi
"${run[@]}" 2>&1 | tee "$out/run.log"
cp "$out/device/readings.txt" "$out/readings.txt"
echo "audio-in-use: the readings are in $out/readings.txt"
