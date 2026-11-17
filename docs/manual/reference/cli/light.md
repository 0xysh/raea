---
title: "raea light"
summary: "Measure a scene's light on each light path against its path-traced reference: distance in stops, the worst tile, flicker and cost."
order: 47
sources: ["engine/light/src/lib.rs", "engine/light/src/metric.rs", "engine/render/raytrace/src/force.rs"]
---

`raea light check` is the bar raea's lighting is held to (docs/realism.feature LT1). Each scene in a list is
rendered on each light path, raster and Solari, and against its reference: the same frames path-traced, each drawn
`--spp` times so it converges. For each path it reports how far the stills are from the reference in stops (the
mean, the bias, brighter positive, and the worst 64-pixel tile), how much a moving shot flickers (each pixel's
second difference in time, in 8-bit steps), and what a frame costs. A reference is made once and kept until
`--fresh`.

The scenes are a JSON file: `{ "scenes": [{ "name", "dir", "file", "composition", "stills": [frames], "shot":
[first, last], "reference": false, "why": "..." }] }`. `dir` is the scene's project (renders run there), `file` its
composition in it; a scene whose reference can't be made yet says so with `"reference": false` and why.

Any render takes the same switches: `RAEA_LIGHT=raster|solari|pathtracer` puts every 3D camera on that path,
whatever the composition asks, and `RAEA_LIGHT_SPP=N` draws each frame N times.

```sh
raea light check bench/light/scenes.json -o out/light/
raea light check bench/light/scenes.json -o out/light/ --scene kitchen --paths solari --spp 512 --fresh
```

<!-- generated:begin by tools/manual-ref/cli.ts from `raea help --json`; edit only the flag descriptions -->

## Usage

```sh
raea light check SCENES.json -o DIR [--scene NAME] [--paths raster,solari] [--spp N] [--fresh]   (each scene on each light path against its path-traced reference: distance in stops, the worst tile, a moving shot's flicker, a frame's cost; DIR/report.json)
```

Headless: it needs nothing running and opens no window. Takes up to 2 arguments besides its flags.

## Flags

| Flag | Takes | What it does |
| --- | --- | --- |
| `--fresh` | – | Make the references again, even where they're kept from an earlier check. |
| `--paths` | `raster,solari` | The light paths to measure, comma-separated (both by default). |
| `--scene` | `NAME` | Check only the scene with this name. |
| `--spp` | `N` | How many times each reference frame is path-traced: its samples a pixel (256 by default). |
| `-o` | `DIR` | The folder for each scene's frames and report.json. |

<!-- generated:end -->
