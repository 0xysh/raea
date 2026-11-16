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

## One garment: what `raea clothes make` runs

    tools/garments/sew.sh OUT_DIR NAME "a hooded sweatshirt" [--set sleeve.length=0.9] [--colour #3f5a3c]
    tools/garments/sew.sh OUT_DIR NAME "" --plain --set meta.upper=none --set meta.bottom=Pants --set pants.length=0.8

`sew.sh` takes a design from words (the model, `GARMENT_MODEL`) or from GarmentCode's plain base (`--plain`),
changes settings by path (`design.py`, which checks each against its options or range), cuts it to SOMA, drapes it
(`GARMENT_SDF`: the body's distance grid) and bakes `OUT_DIR/NAME/NAME.glb`. `raea clothes make` sets these up in
raea's models folder (`garments/`) the first time and calls it (engine/cli/src/sewn.ts); `hide_under.py` leaves out
the body's skin under worn garments for `raea clothes wear`.

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

## Words to a garment: the first model (CH48c)

SigLIP 2 (in raea's models) puts pictures and words in one space, so a small network trained on pictures of the
draped designs answers lines of words, with no captions written. Needs `onnxruntime` and `tokenizers` in the venv.

    LEAN=1 tools/garments/engine.sh DATA 300 101                                     # designs, draped
    PICTURES=1 raea blender run tools/garments/bake_batch.py BODY.glb DATA/g*/       # four pictures each
    $GARMENT_PY -I tools/garments/garment_embed.py DATA                              # DATA/embeds.npz
    $GARMENT_PY -I tools/garments/garment_model.py train DATA MODEL.npz              # seconds
    $GARMENT_PY -I tools/garments/garment_model.py eval DATA MODEL.npz               # held out, lines of words
    tools/garments/words.sh MODEL.npz OUT "a pencil skirt" "a top with a hood"       # designed, cut, draped, sheet

- `garment_params.py`: a design's settings as one vector (numbers by place in range, choices one-hot) and back
- `garment_embed.py`: SigLIP 2 picture and text embeddings, ONNX on the CPU (Core ML's GPU is wrong on this Mac)
- `garment_model.py`: the network (embedding to settings), its training with noise so text lands, eval, `make`

## Worn hard pieces (CH49)

What can't be sewn (helmets, hats, armour, visors, hard-shell suits) is drawn on SOMA's mannequin by an image model,
made 3D by Idol on this machine, laid on SOMA and split by bone:

    tools/garments/wear.sh NAME "a tall black Victorian silk top hat" keyed   # leaves skin bare: magenta mannequin
    tools/garments/wear.sh NAME "a full suit of plate armour" full            # covers the body: kept whole, lifted

- `wear_mannequin.py`: SOMA at rest from three-quarters ahead, its camera written down (`KEY=1`: magenta)
- `wear_fit.py`: Idol's mesh turned, sized and placed onto SOMA (turn searched, then refined against the skin), bare body
  cut (full: by distance; keyed: by the magenta, so a crown tight on the skull stays), what is kept lifted off the skin.
  Full: each face goes to its nearest bone. Keyed: each separate piece rides one bone whole, the one nearest its centre
  (never a neck bone; a midline bone for a piece across the middle), and pieces that hug the skin all over are dropped
- `wear_bake.py`: each piece weighted wholly to its bone on SOMA's armature, Idol's colours made linear, one glb + a picture

About 5 cents (the picture) and 5 minutes (Idol) a piece. Faults so far: the back is Idol's guess from the one view;
the surface is as lumpy as Idol's 4 mm mesh;
a full suit's hands and feet copy SOMA's bare shape when the drawn gauntlets don't match its hand pose.

For training data at scale, `LEAN=1 tools/garments/engine.sh OUT N SEED` keeps each design's settings, caption, drape
numbers and draped mesh only (about 0.4 MB a design).
