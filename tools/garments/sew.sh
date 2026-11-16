#!/bin/bash
# One sewn garment for SOMA (docs/character.feature CH45c): a design from words (the garment model) or from the plain
# base, settings changed by path, cut to SOMA, draped on the GPU and baked to a skinned glb. `raea clothes make` runs it.
#   tools/garments/sew.sh OUT_DIR NAME "WORDS" [--plain] [--set PATH=VALUE]... [--colour #rrggbb]
# Needs GARMENTCODE, GARMENT_PY (README.md), GARMENT_MODEL (the words model, unless --plain) and GARMENT_SDF (the body
# as a distance grid, sdf_igl.py); SOMA_BODY is the body it is baked on. Writes OUT_DIR/NAME/: design.yaml, NAME.glb,
# NAME.drape.json, NAME.bake.json, caption.txt. Prints lines starting "sew: ". $0, about 30 s.
set -euo pipefail
T=$(cd "$(dirname "$0")" && pwd)
GC=${GARMENTCODE:?a GarmentCode clone, see tools/garments/README.md}
PY=${GARMENT_PY:?the venv python, see tools/garments/README.md}
SDF=${GARMENT_SDF:-$HOME/.cache/raea-garments/soma_sdf.npz}
BODY=${SOMA_BODY:-$HOME/.local/share/raea-models/soma/body/body.glb}
OUT=${1:?out dir}; n=${2:?name}; words=${3:-}; shift 3 || true
PLAIN=0; SETS=(); COLOUR=
while [ $# -gt 0 ]; do
  case "$1" in
    --plain) PLAIN=1; shift ;;
    --set) SETS+=("$2"); shift 2 ;;
    --colour) COLOUR=$2; shift 2 ;;
    *) echo "sew: unknown word $1" >&2; exit 2 ;;
  esac
done
d=$OUT/$n
rm -rf "${d:?}"; mkdir -p "$d"; printf '%s\n' "$words" > "$d/words.txt"
if [ "$PLAIN" = 1 ]; then
  cp "$GC/assets/design_params/t-shirt.yaml" "$d/design.yaml"
else
  "$PY" -I "$T/garment_model.py" make "${GARMENT_MODEL:?the words model (garment_model.py train), or --plain}" "$words" "$d" > "$d/make.log"
fi
if [ ${#SETS[@]} -gt 0 ]; then "$PY" -I "$T/design.py" "$d/design.yaml" "$d/design.yaml" "${SETS[@]}" > /dev/null; fi
pattern() { (cd "$T" && DESIGN="$d/design.yaml" GID=$n "$PY" -I sample_designs.py "$GC" "$T/soma.yaml" "$OUT" 1 1) 2>&1 | grep -E "^sample: " | cut -c1-200 || true; }
said=$(pattern)
# refused as decoded from words alone (a combination GarmentCode's checks don't allow): the training design nearest the words
if [ ! -f "$d/$n.panels.json" ] && [ "$PLAIN" = 0 ] && [ ${#SETS[@]} -eq 0 ] && [ -f "$d/nearest.yaml" ]; then
  cp "$d/nearest.yaml" "$d/design.yaml"; echo nearest > "$d/fallback.txt"; said=$(pattern)
  echo "sew: the design decoded from the words was refused; the nearest design known is used instead"
fi
[ -f "$d/$n.panels.json" ] || { echo "sew: FAILED the pattern can't be cut: ${said#sample: }" >&2; exit 1; }
log=$( (cd "$T" && "$PY" -I drape_mlx.py "$SDF" "$d/$n.panels.json" "$d" 150 -0.064 "$T/soma.yaml") 2>&1 ) || { echo "sew: FAILED the drape: $(grep -vE '^\s*$' <<<"$log" | tail -1)" >&2; exit 1; }
"$PY" -I -c 'import json,sys; r=json.load(open(sys.argv[1])); g=r["seam_gap_mm"]; print("sew: draped in %.0f s: %s" % (r["seconds"], "it stayed on and every seam closed" if r["ok"] else "FAULT " + ("it slid off" if not r["stayed"] else "a seam is open or cloth is in the skin") + " (seam gap %.0f mm at most)" % g.get("max", 0)))' "$d/$n.drape.json"
COLOUR=$COLOUR "${RAEA_BIN:-raea}" blender run "$T/bake_batch.py" "$BODY" "$d" 2>&1 | grep -E '^bake|Error|Traceback' | sed 's/^bake: /sew: baked /'
[ -f "$d/$n.glb" ] || { echo "sew: FAILED no glb was baked" >&2; exit 1; }
echo "sew: made: $(cat "$d/caption.txt" 2>/dev/null || echo "$n")"
