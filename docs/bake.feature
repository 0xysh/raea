@spec
Feature: Light baked by raea: bounce light into lightmaps on raea's own ray tracer, no Blender
  Shane, 2026-11-23, while a test agent built the atelier scene (an architect's loft, every object modeled in raea)
  and baked its bounce light through Blender: "worth researching how we could do baking of bounce lights without
  blender and in raea", then "write in a spec and have another subagent buidl all 5".

  Why: the realism recipe (sdk/guides/craft/bevy/realism.md, docs/blender-research.md) halves the gap to Cycles with a
  sky, a balanced sun and a lens finish, and halves it again with bounce light baked into <Lightmap> (RMSE against
  Cycles 0.154 -> 0.081 -> 0.040 on the test set). Today that bake is Cycles through `raea blender open` + look.py +
  lightbake.py. It costs a Blender install, an export round trip that drops things (the atelier agent found
  `raea blender open` losing the export's meshes/ folder), lightmaps only on a glTF <Model> (so the baked set is one
  model, and `scene lint` and the review see one part), and a lit scene that isn't quite the scene raea draws.

  Blender stays what it is good at for raea: geometry steps (bevel, boolean, remesh, UV, geometry nodes) and Cycles as
  the reference the bake is measured against. Light is computed by raea.

  The model this file sets:
    1. A bake is a pure function of what it's asked: the static parts (their geometry, materials and placement), the
       sky, the lights it carries and the settings. It is kept in the composition's asset store under that key, as
       systems/bake keeps physics bakes (docs: systems/bake/src/lib.rs), so a render, a `render --at` and a play read
       the same bytes and nothing is baked twice.
    2. What is baked is the light that bounces: the sky, emissive surfaces and the lights marked baked, through every
       bounce, plus the sun's bounce. The sun's direct light stays live (as lightbake.py subtracts it), so anything
       that moves still casts its shadow and is lit as before.
    3. It runs on systems/rt: the same BLAS and TLAS the traced shadows and raea GI read, through wgpu ray queries
       (Metal on Apple silicon's ray hardware, Vulkan, DX12). Every OS raea ships on; never Mac-only. A device without
       ray queries is told so, with the reason, and the composition renders unbaked.
    4. Shading is Cycles', from tools/cycles-port (each function held to Cycles' own output), so the bake and the
       reference agree by construction, and LT5's path tracer (docs/realism.feature) reuses the same kernel.
    5. It lights raea's own meshes (Mesh3d, solids, raea/room) directly, as well as a <Model>'s, with no export.

  Background:
    Given systems/rt (raea-sys-rt): BLAS per mesh from Bevy's own buffers, a TLAS of visible opaque instances,
      glass left out so sky and sun reach through windows; its traced AO, raea GI's world probes and cache (LT2, LT3)
    And tools/cycles-port/cycles_bsdf.wgsl: GGX/Beckmann D, G2, Fresnel, F82 conductors, VNDF sampling, passing against
      Cycles on an M5 Pro; diffuse and energy-conserving Oren-Nayar next
    And <Lightmap> (sdk/render/src/bevy3d/lightmap.tsx, systems/material lightmap.rs): Bevy's lightmap on a <Model>'s
      glTF meshes by name, sampled by the mesh's second UVs, scaled by its material's lightmap_exposure
    And lightbake.py (sdk/guides/craft/bevy/blender/): Cycles' DIFFUSE bake per mesh into a 16-bit PNG, smart-project
      lightmap UVs, sun subtracted, edges smoothed; the reference this file measures against
    And Godot 4's LightmapGI (MIT) as the design to learn from: xatlas UVs, texels rasterized in lightmap space, a GPU
      ray tracer, a denoiser, dilation; Unity's progressive lightmapper and Cycles' own bake as the other two

  Rule: the bar (BK0)

    Scenario: BK0. The bake's test set, its references and its numbers
      Given the plan
        | crate              | file                                 | lines | job                                                                 |
        | raea-app-lightbake | apps/lightbake/src/lib.rs            | 220   | `raea bake light EDIT.tsx [--composition ID] [--samples N] [--compare DIR]`: bake, then report |
        | raea-app-lightbake | apps/lightbake/src/compare.rs        | 180   | the same set's Cycles bake (lightbake.py) and Cycles still (look.py --engine cycles) beside raea's: stops and RMSE per mesh and per frame, time, memory |
        | raea-app-lightbake | apps/lightbake/sets.json             | 40    | the test set: blender-research's table set, a Cornell box, a sunlit room with one window, the atelier's static set |
      Then each set has a Cycles reference at 1024 samples: its lightmaps and a still from a fixed camera
      And `raea bake light --compare DIR` writes DIR/report.json and a table: for each set, raea's bake against Cycles'
        bake (mean and worst-tile stops per mesh) and raea's frame against the Cycles still (RMSE, as blender-research
        measured), bake time and peak memory
      And the bar is: the frame with raea's bake within 0.045 RMSE of Cycles on the table set (lightbake.py gives
        0.040), no mesh off by more than a third of a stop on average, in under a quarter of lightbake.py's time
      And the numbers before any BK step lands are recorded in this file, as LT1's are in docs/realism.feature

  Rule: the five parts (BK1-BK5)

    @needs:BK0
    Scenario: BK1. Lightmap UVs for every static mesh
      Given the plan
        | crate              | file                                 | lines | job                                                                 |
        | raea-sys-lightbake | systems/lightbake/src/uvs.rs         | 240   | a second UV set per mesh from xatlas (MIT): charts cut at hard edges, packed with padding, texel density from world size |
      Then every mesh a bake covers gets non-overlapping second UVs, charts at least 2 texels apart at its resolution
      And its texel density follows its world area (a 6 m wall and a 6 cm stapler at the same texels a metre, capped
        by `texels` and a per-bake atlas size), so nothing is stretched or wasted
      And the UVs are a pure function of the mesh: the same mesh gives the same atlas, cached with the bake
      And a mesh that already has second UVs keeps them (a <Model> exported with a lightmap layer)
      And thin and degenerate triangles are measured: a chart under a texel is merged or flagged, never left unlit

    @needs:BK1
    Scenario: BK2. Each texel's place in the world
      Given the plan
        | crate              | file                                 | lines | job                                                                 |
        | raea-sys-lightbake | systems/lightbake/src/texels.rs      | 160   | each mesh rasterized in its lightmap UVs: world position, normal, which mesh and triangle, per texel |
        | raea-sys-lightbake | systems/lightbake/src/texels.wgsl    | 90    | the raster pass, conservative, so a triangle thinner than a texel still writes |
      Then each mesh's atlas has, per texel, its world position, shading normal and whether it is covered
      And a texel whose position is inside other geometry (a table leg's foot in the floor, a wall's end inside the
        next wall) is found by a short ray test and marked, so it neither goes black nor leaks light into the bake
      And positions are offset along the normal by a scale-aware epsilon, so the trace never hits its own surface

    @needs:BK2
    Scenario: BK3. The light that bounces, traced on raea's ray tracer
      Given the plan
        | crate              | file                                 | lines | job                                                                 |
        | raea-sys-lightbake | systems/lightbake/src/trace.rs       | 260   | the passes: dispatch per atlas, samples accumulated over passes, progress, cancel |
        | raea-sys-lightbake | systems/lightbake/src/trace.wgsl     | 320   | per texel: cosine-weighted paths against systems/rt's TLAS; the sky from the composition's <Environment>, emissives, baked lights; Russian roulette; Cycles' BSDFs from cycles-port at each bounce |
      Then each texel integrates the incoming diffuse light over its hemisphere along paths of up to `bounces`
        (default 4) through systems/rt's BLAS and TLAS, the materials at each hit being the composition's
        StandardMaterials (base colour, its texture, roughness, metallic, emissive) as the raster reads them
      And the sky is the composition's <Environment> panorama at its intensity and rotation; emissive surfaces light
        the set; the sun's direct light is left out at the first hit and kept at every later bounce (so a sunlit floor
        lights the ceiling), and a light marked `bake` is carried whole
      And samples accumulate over passes (`samples`, default 512 a texel), a pass small enough that the window and the
        machine stay responsive, with progress printed and a stall guard's line each pass
      And a white furnace (a white set under a white sky) converges to the sky's value at every texel, within 1%
      And the same request on the same machine gives the same bytes (fixed seeds per texel and pass)

    @needs:BK3
    Scenario: BK4. A clean map: denoised, dilated, no seams
      Given the plan
        | crate              | file                                 | lines | job                                                                 |
        | raea-sys-lightbake | systems/lightbake/src/clean.rs       | 220   | denoise guided by normal and position, dilation past chart edges, seam texels averaged, 16-bit out |
        | raea-sys-lightbake | systems/lightbake/src/clean.wgsl     | 120   | the edge-aware filter (a-trous on normal, position and chart), as Godot's JNLM does it |
      Then the noise at 512 samples a texel is filtered without blurring across a crease, a chart edge or a contact
        shadow (the filter's weights from normal, position and chart id)
      And every chart is dilated by its padding, so bilinear and mip sampling never reads an empty texel
      And texels that meet across a UV seam are made equal, so a seam never shows as a line in the frame
      And the map is 16-bit HDR (KTX2, which Bevy reads, or 16-bit PNG as lightbake.py writes), with the exposure
        that puts it at raea's brightness, as lightbake.py's lightmap_exposure does
      And Open Image Denoise (Apache-2.0) may replace the filter where it is installed; the built-in filter is the
        default on every OS

    @needs:BK4
    Scenario: BK5. A bake in a composition: <LightBake> and `raea bake light`
      Given the plan
        | crate              | file                                 | lines | job                                                                 |
        | raea-sys-lightbake | systems/lightbake/src/lib.rs         | 200   | the bake as a pure function of its request, kept in the asset store by key; the native call |
        | raea-sys-lightbake | systems/lightbake/src/apply.rs       | 160   | each baked mesh's lightmap and second UVs on raea's own Mesh3d and on a <Model>'s meshes |
        | @raea/render       | sdk/render/src/bevy3d/lightbake.tsx  | 120   | <LightBake samples bounces texels> around the static parts it bakes; re-exported from raea/3d |
      Then `<LightBake>` around a composition's static parts bakes them on the first frame a render, a play or a
        `render --at` needs it, and every later frame and run reads the kept bake
      And it lights raea's own Mesh3d, solids and raea/room parts in place, as well as a <Model>'s meshes, so the
        parts stay parts: `raea scene lint`, `raea render --review` and `raea make` see each one, not one baked model
      And a change to a static part, a material, the sky or a setting changes the key and bakes again; a change to
        anything outside <LightBake> (the camera, a moving part, the sun's direct light) doesn't
      And `raea bake light EDIT.tsx` bakes ahead of a render and says what it baked (meshes, texels, samples, time)
      And `raea guide bevy/lights` gets a "Baked light" section that puts <LightBake> first, with lightbake.py kept as
        the Cycles reference to compare against; the realism guide's bake step points to it once that guide lands
      And on BK0's test set, a composition with <LightBake> meets the bar, and the atelier's static set bakes in raea
        with its Blender bake as the comparison
