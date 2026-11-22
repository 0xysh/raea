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
PX=${FRAME_PURE_PX:-16}
[ -x "$R" ] || { echo "frame-pure-3d: no raea at $R (cargo build --release -p raea-cli)" >&2; exit 2; }
command -v ffmpeg >/dev/null || { echo "frame-pure-3d: needs ffmpeg on PATH" >&2; exit 2; }
W=$(mktemp -d "${TMPDIR:-/tmp}/frame-pure-3d.XXXX")
trap 'rm -rf "$W"' EXIT
[ $# -gt 0 ] || set -- "$ROOT"/engine/render/tests/diag/*.tsx
fail=0
ssim() {
  ffmpeg -v info -i "$1" -i "$2" -lavfi "[0:v]format=rgb24[a];[1:v]format=rgb24[b];[a][b]ssim" -f null - 2>&1 | grep -oE 'All:[0-9.]+' | cut -d: -f2
}
# pixels that differ by more than 96 of 255 levels, in patches: SSIM over a whole frame hides one small part (a cut
# letter scored 0.9996), and this doesn't. An erosion first drops lone pixels, the edge noise of a glyph drawn from
# another place in the atlas (17 of them along two revealed words), and keeps a patch (a cut letter: 318)
bad_px() {
  ffmpeg -v error -i "$1" -i "$2" -filter_complex "[0:v]format=rgb24[a];[1:v]format=rgb24[b];[a][b]blend=all_mode=difference,format=gray,erosion" -f rawvideo - |
    python3 -I -c 'import sys; d = sys.stdin.buffer.read(); print(len(d) - len(d.translate(None, bytes(range(97, 256))) ))'
}
# a pair matches when its SSIM is at the bar and no more than PX pixels differ by more than 96 levels
differ() {
  local s p
  s=$(ssim "$1" "$2"); p=$(bad_px "$1" "$2")
  echo "SSIM ${s:-none}, ${p:-?} pixels off"
  [ -z "$s" ] || [ -z "$p" ] || awk -v s="$s" -v b="$BAR" -v p="$p" -v m="$PX" 'BEGIN { exit !(s < b || p > m) }'
}
for t in "$@"; do
  n=$(basename "$t" .tsx)
  # a repro with a "cold" and a "warm" composition (the glyph atlas, SG3): what a cold atlas draws must match a warm one
  if grep -q 'id="cold"' "$t" && grep -q 'id="warm"' "$t"; then
    for c in cold warm; do
      "$R" render "$t" --composition "$c" --frame 0 -o "$W/$n-$c.png" >"$W/$n-$c.log" 2>&1 || { echo "frame-pure-3d: $n: $c didn't render:"; tail -3 "$W/$n-$c.log"; fail=1; }
    done
    if m=$(differ "$W/$n-cold.png" "$W/$n-warm.png"); then
      echo "frame-pure-3d: $n: drawn into a cold atlas it differs from a warm one ($m; bar SSIM $BAR, $PX pixels): a glyph allocated as the atlas grew is cut"
      fail=1
    else
      echo "frame-pure-3d: $n: cold and warm atlas match ($m)"
    fi
    continue
  fi
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
    if m=$(differ "$seq" "$W/$n-alone-$f.png"); then
      echo "frame-pure-3d: $n frame $f: the sequence's frame differs from the frame rendered alone ($m; bar SSIM $BAR, $PX pixels): a 3D part kept an earlier frame's inputs or took another's row"
      fail=1
    else
      echo "frame-pure-3d: $n frame $f: $m"
    fi
  done
done
exit $fail
