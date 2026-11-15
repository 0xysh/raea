#!/bin/bash
# The garment data engine (docs/character.feature CH48a): N random GarmentCode designs cut to SOMA's measurements,
# draped on the GPU, skinned, pictured on the mannequin, one contact sheet.
#   tools/garments/engine.sh OUT_DIR N [SEED]
# Needs (see README.md): GARMENTCODE (a clone at d449629), GARMENT_PY (its venv's python, with mlx and libigl),
# and the body's signed distance grid at $GARMENT_DATA/soma_sdf.npz (sdf_igl.py builds it once).
set -euo pipefail
T=$(cd "$(dirname "$0")" && pwd)
GC=${GARMENTCODE:?a GarmentCode clone, see tools/garments/README.md}
PY=${GARMENT_PY:?the venv python, see tools/garments/README.md}
DATA=${GARMENT_DATA:-$HOME/.cache/raea-garments}
BODY=${SOMA_BODY:-$HOME/.local/share/raea-models/soma/body/body.glb}
RAEA=${RAEA:-raea}
OUT=${1:?out dir}; N=${2:?count}; SEED=${3:-1}
[ -f "$DATA/soma_sdf.npz" ] || "$PY" -I "$T/sdf_igl.py" "$BODY" "$DATA/soma_sdf.npz"
(cd "$T" && "$PY" -I sample_designs.py "$GC" "$T/soma.yaml" "$OUT" "$N" "$SEED") | grep --line-buffered -v rejected
for d in "$OUT"/g*/; do
  n=$(basename "$d")
  [ -f "$d/$n.drape.json" ] && continue
  (cd "$T" && "$PY" -I drape_mlx.py "$DATA/soma_sdf.npz" "$d/$n.panels.json" "$d" 150 -0.064 "$T/soma.yaml") \
    | grep --line-buffered -E '^drape: \{|Error|Traceback' || echo "drape: $n failed"
done
"$RAEA" blender run "$T/bake_batch.py" "$BODY" "$OUT"/g*/ 2>&1 | grep --line-buffered -E '^bake|Error|Traceback'
"$PY" -I "$T/sheet.py" "$OUT/sheet.jpg" "$OUT"/g*/
