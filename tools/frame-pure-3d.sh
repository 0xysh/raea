#!/bin/bash
# Frame purity for 3D (docs/stage-guards.feature SG0): each repro in engine/render/tests/diag is rendered as one
# sequence (a folder of PNGs: frame after frame in one process, as an mp4's live 3D is made) and four of its frames
# rendered alone (--frame N, each in its own process: a still's route), and frame N of both must match (SSIM 0.999 or
# more, lossless). A part that held an earlier frame's shape, words, size or colour (SG1), or drew in another part's row
# (SG2), fails here with the repro and the frame named. L1 runs it when the code that makes 3D rows changes.
#   tools/frame-pure-3d.sh [REPRO.tsx...]      (RAEA_BIN: the raea to render with; default target/release/raea)
set -uo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
R=${RAEA_BIN:-$ROOT/target/release/raea}
BAR=${FRAME_PURE_BAR:-0.999}
[ -x "$R" ] || { echo "frame-pure-3d: no raea at $R (cargo build --release -p raea-cli)" >&2; exit 2; }
command -v ffmpeg >/dev/null || { echo "frame-pure-3d: needs ffmpeg on PATH" >&2; exit 2; }
W=$(mktemp -d "${TMPDIR:-/tmp}/frame-pure-3d.XXXX")
trap 'rm -rf "$W"' EXIT
[ $# -gt 0 ] || set -- "$ROOT"/engine/render/tests/diag/*.tsx
fail=0
for t in "$@"; do
  n=$(basename "$t" .tsx)
  len=$(grep -oE 'durationInFrames=\{[0-9]+\}' "$t" | head -1 | grep -oE '[0-9]+')
  [ "${len:-1}" -gt 1 ] || continue
  if ! "$R" render "$t" -o "$W/$n/" >"$W/$n.log" 2>&1; then
    echo "frame-pure-3d: $n: the sequence didn't render:"; tail -5 "$W/$n.log"; fail=1; continue
  fi
  for f in 0 $((len / 3)) $((2 * len / 3)) $((len - 1)); do
    if ! "$R" render "$t" --frame "$f" -o "$W/$n-alone-$f.png" >"$W/$n-$f.log" 2>&1; then
      echo "frame-pure-3d: $n: frame $f alone didn't render:"; tail -3 "$W/$n-$f.log"; fail=1; continue
    fi
    seq=$(printf '%s/%s/frame-%05d.png' "$W" "$n" "$f")
    s=$(ffmpeg -v info -i "$seq" -i "$W/$n-alone-$f.png" -lavfi "[0:v]format=rgb24[a];[1:v]format=rgb24[b];[a][b]ssim" -f null - 2>&1 | grep -oE 'All:[0-9.]+' | cut -d: -f2)
    if [ -z "$s" ] || awk -v s="$s" -v b="$BAR" 'BEGIN { exit !(s < b) }'; then
      echo "frame-pure-3d: $n frame $f: the sequence's frame differs from the frame rendered alone (SSIM ${s:-none}, bar $BAR): a 3D part kept an earlier frame's inputs or took another's row"
      fail=1
    else
      echo "frame-pure-3d: $n frame $f: SSIM $s"
    fi
  done
done
exit $fail
