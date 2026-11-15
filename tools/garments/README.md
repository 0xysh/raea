# tools/garments: garments from sewing patterns, and the data engine

Garments for SOMA's body made the way clothes are made: flat pieces cut to a pattern, sewn, and draped by gravity
on the body (docs/character.feature CH47a, CH48a). The patterns come from GarmentCode (ETH Zurich, `pygarment`,
MIT): garments as programs of components (bodices, sleeves, collars, cuffs, waistbands, trousers, skirts) joined by
seams and sized from body measurements. The cloth is our own position-based solver on the GPU (MLX).

Not used, on purpose: GarmentCode's simulator (a fork of an old NVIDIA Warp under a non-commercial licence),
its sample SMPL bodies (research-only) and GarmentCodeData (draped on SMPL, licence unconfirmed).

## Setup, once

    git clone https://github.com/maria-korosteleva/GarmentCode && git -C GarmentCode checkout d449629
    python3.11 -m venv venv && venv/bin/pip install "numpy<2" scipy pyyaml svgwrite svgpathtools cairosvg \
      matplotlib cgal trimesh psutil networkx shapely rtree libigl pillow mlx
    export GARMENTCODE=$PWD/GarmentCode GARMENT_PY=$PWD/venv/bin/python

## Run

    tools/garments/engine.sh OUT_DIR 24 11     # 24 random designs, seed 11

Each `OUT_DIR/<id>/` holds `design.yaml` (the design's settings: the labels), `caption.txt`, GarmentCode's
pattern, `<id>.panels.json` (flat pieces, their place round the body, seams as vertex pairs, edge labels),
`<id>.draped.npz`, a skinned `<id>.glb`, four pictures on the mannequin and `<id>.drape.json` (seam gaps,
stretch, cloth inside the skin, and whether it stayed on).

## The scripts

- `soma_measure.py`: SOMA in GarmentCode's 26 measurements (`soma.yaml`; chest is the fullest girth under the arms)
- `sample_designs.py`: random designs from GarmentCode's sampler, its own checks, cut to `soma.yaml`
- `sdf_igl.py`: the body as a 7 mm signed distance grid, signed by winding number
- `drape_mlx.py`: the cloth (edges at their flat length, seams to a point, pins, pushed out of the skin)
- `bake_batch.py`: seams welded, the flat cut as UVs, weights from the skin, glb, pictures (`raea blender run`)
- `make_garment.py`, `design.py`: one garment from settings by path (`sleeve.length=0.95`)
- `render_pretty.py`: a garment to look at (EEVEE, fabric with sheen, soft studio light; `raea blender run`)
- `sheet.py`, `grid.py`, `snaps_view.py`: contact sheets, and the drape's snapshots (`SNAP=1`) over the body
