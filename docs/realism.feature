@spec
Feature: raea Light: raea's own real-time realism renderer, measured against raea's own reference
  Shane, 2026-11-17, on gambit's characters in the game's own light: "how do we get UNREAL ENGINE level detail and
  quality"; then "before anything we have to figure out the shader/raytracing/ambientocclusion of the engine we want to
  use. right now we seem completely misaligned and need to centralize the strategy to get SOTA results"; then "what if
  we could build our own thing for raea that was a complete realism like system like unreal engine"; and, once FLOOR3
  measured Solari at 2.6 times raster's frame: "i want our own version our own raea realism renderer then that doesnt
  use solari ... if solari is too slow then we need to look elsewhere".

  This file is that renderer's one strategy, its owner's backlog and its bar. It replaces choosing a lighting path per
  project. docs/raytracing.feature (RT1-RT6) and docs/shaders.feature (SH0-SH11) keep the steps they recorded;
  anything about light, occlusion, reflections or how a surface is shaded is decided here first. The first version of
  this file (11-17, commits ttn, xok, quu on branch `realism`) built on Solari; this one replaces it.

  The design is the one Unreal's Lumen and AMD's GI-1.0 use to get ray-traced light at game frame rates: raster draws
  what the camera sees and its direct light; the light that bounces is traced into caches that keep what they learned
  and are refreshed a little each frame, at a cost set by a ray budget, not by the screen. Solari instead traces every
  pixel every frame (ReSTIR), which is why it costs what it does.

  What Unreal's realism is, and what raea Light makes of it:
    | Unreal                     | what it does                                         | raea Light                                          |
    | Lumen surface cache        | what a ray sees: each mesh's material on cards       | the surface cache (LT4), captured through the       |
    |                            |                                                      | deferred pass every material writes                 |
    | Lumen world radiance cache | far and multi-bounce light, probes around the camera | the world radiance cache (LT2, LT3)                 |
    | Lumen screen probes        | detail and contact: sparse probes from the G-buffer  | screen probes (LT8)                                 |
    | Lumen reflections          | traced for smooth surfaces, the cache for rough      | the same (LT9)                                      |
    | shadow maps, MegaLights    | direct light and its shadows                         | Bevy's raster lights and shadow maps, as today      |
    | skin, hair, cloth models   | subsurface skin, hair, cloth sheen                   | shading models the GI goes through (LT11)           |
    | TSR                        | a clean, stable frame from few rays and fewer pixels | caches' own history, our temporal filter, MetalFX   |
    |                            |                                                      | upscaling on the Mac (LT12)                         |
    | path tracer reference      | ground truth with the same materials                 | raea's path tracer on systems/rt (LT5)              |

  What was found (11-17, gambit's people-ingame.tsx, the four heroes in the baked city, four skies; this Mac, M5 Pro):
    M1. Five lighting paths, none whole: a flat AmbientLight (CityShot); Bevy's SSAO (a white veil and bright face
        outlines on the heroes); raea's own traced AO (systems/rt; its pipeline failed to build on Metal, a ray query
        declared in a loop, fixed 11-17); Solari (FLOOR3, Stay and the anything films light every shot with it); a
        captured environment (gambit, by hand: six views a sky stitched into a panorama).
    M2. A surface hook runs in the forward pass only (SH0c, 10-28). Any pass that reads materials another way (a
        deferred G-buffer, a ray hit) sees StandardMaterial's raw values: gambit's world shaders keep a surface's kind
        in the vertex colour's alpha, so under Solari the snow drew teal and the windows red and cyan (posted
        i-acp-mwl90ova-102). procmat had the same gap (M10).
    M3. Characters aren't in traced light: Solari skips skinned and morphed meshes (RT5); systems/rt takes them in
        their bind pose (arms out). A trial (RAEA_SOLARI_DYNAMIC=1, another lane) poses CPU twins.
    M4. Nobody owned light: docs/control.feature lists ray tracing's owner as "-". Each project tunes its own light by
        eye (FLOOR3's light-jump rules, the kitchen's rig with a panel for "the bounce the renderer doesn't trace",
        gambit's LIFT and ev).
    M5. No reference: no frame was measured against ground truth. Bevy's <Pathtracer> reads raw materials (M2), so
        it can't be one for hooked worlds.
    M6. Shading changes land without a lighting check: 60b8152a86 (11-17) gave head parts the skin shader they were
        meant to have, and faces drew brighter in 17% of a dawn close-up; nobody measured it.
    M7. Traced AO (reach 0.6 m) darkens contacts as it should, but also brightens 7-8% of a face close-up, which
        occlusion alone can't do: unexplained (the bind-pose arms of M3 are a suspect).
    M8. Solari ignores PointLight and RectLight; projects stood emissive spheres and sheets in for them by hand.
    M9. Per-frame CPU geometry freezes in a render: a <BufferMesh> whose vertices change each frame is built once.
    M10. Every MaterialExtension raea has (ShaderExt, procmat, toon, tint, paint, grid, froxel glass) named no
        deferred shader; procmat's is written (6 lines, handed over by the first-person lab 11-17: its shader's
        PREPASS_PIPELINE branch already wrote the G-buffer, and the oilcloth kept its checks under Solari).
    M11. Solari is accurate but slow (LT1, FLOOR3 at 1080p): 0.96 stops from the reference against raster's 1.38,
        its error almost all one bias a stop dark; 47.6 ms a frame against raster's 18.0.
    M12. systems/rt builds a BLAS a mesh from Bevy's own vertex buffers and a TLAS every frame, with a shadow mask
        but no instance data: a ray learns where it hit, not the surface's normal or colour.
    M13. Bevy's irradiance volume is a 3D texture of ambient cubes (Rx, 2Ry, 3Rz: one colour each way a voxel) that
        every standard material samples, hooked ones included; it has no visibility test, so light can leak
        through a wall. A compute pass can write it each frame.
    M14. The skin and hair shaders of sdk/character use `lit`, the hook that runs after lighting; gambit's world
        shaders don't.
    M15. docs/realtime.feature's budget is the editor's (a drag to its first changed pixel <= 120 ms p50, 250 ms
        ray-traced at half size), not a game's frame.

  Decisions:
    D1. The engine stays Bevy 0.20 on wgpu (docs/gpu.md): no backend of our own. Bevy's raster draws what the camera
        sees and direct light (sun, point, spot, rect lights, shadow maps), as it does today. raea Light adds what
        raster fakes: bounce light, occlusion, reflections, and the reference.
    D2. No Solari. It goes from every live, render and reference path once raea's replaces each use (LT13), and
        vendor/bevy_solari with it.
    D3. One ray scene: systems/rt's TLAS, every frame, holding everything opaque, characters posed (LT7). Each
        instance carries a record (its mesh's vertex and index slices, its surface-cache cards), so a hit knows its
        normal and what it looks like (LT3).
    D4. Every material writes the deferred G-buffer with its own surface code (procmat first, then ShaderExt and the
        rest: M10). That one pass serves three readers: the surface cache's capture (D5), the screen probes (LT8) and
        the reference's first hit (LT5). A `lit` hook can't run there (M14): skin and hair become shading models
        (LT11) instead.
    D5. The surface cache is what a ray sees: each mesh captured from up to six sides into low-resolution cards
        (albedo, normal, emission) through the deferred pass, so a hook's look reaches the rays without being
        rewritten. Until it lands, hits read a flat stand-in.
    D6. raea GI has two levels. The world radiance cache: probes on nested grids around the camera, each probe's
        irradiance and its distance to what it sees (so light doesn't leak through walls), refreshed by a fixed ray
        budget a frame, multiple bounces by reading itself at hits. Screen probes: one every 16 pixels from the
        G-buffer, short rays through the TLAS ended in the world cache, filled in between by depth and normal. Until
        the screen probes land, the world cache reaches the frame through Bevy's irradiance volume (M13).
    D7. Every light feeds the GI: a hit is lit by the sun and the sky, point, spot and rect lights and emissive
        surfaces, each with a shadow ray, in raster's units.
    D8. Reflections: smooth surfaces traced, hits shaded from the surface cache and the world cache; rough ones read
        the probes.
    D9. Occlusion isn't an effect of its own: it comes out of the GI. SSAO and traced AO go once the GI replaces them,
        and M7 is explained first.
    D10. Skin, hair and cloth are shading models (subsurface skin, two-lobe hair, sheen cloth) that direct light and
        the GI both go through.
    D11. Ground truth is raea's own path tracer on systems/rt: its first hit from the camera's G-buffer (every
        material exactly as raster draws it), its bounces from the surface cache, its lights as the GI's. Until it
        lands (LT5), Bevy's <Pathtracer> stands in for scenes it can draw (no hooks: FLOOR3, the glade).
    D12. One set of units: lux, cd/m², EV, and one exposure model for every path.
    D13. The game's budget, for Shane to confirm (M15): 60 fps at 1080p on an M-series Mac (16.7 ms a frame), with
        raea GI's share at most 5 ms of it; 30 fps the floor; quality tiers (rays a frame, probe spacing, screen-probe
        resolution) scale it down for smaller GPUs. The editor's live frame keeps docs/realtime.feature's budget.
    D14. One owner: the session that wrote this file, on lane `realism`. A change to light, shadow, occlusion,
        exposure or a shading model, in any lane, lands with its LT1 numbers before and after (LT14).

  Background:
    Given Bevy 0.20 on wgpu with ray queries (Apple silicon on Metal; RTX and RDNA2+ on Vulkan), systems/rt's TLAS
    And the house rules: builds through `lane build --`, commits through `but`, the shared raea-signed rebuilt only
      after telling the sessions that render with it, benches run on a copy of the binary in scratch

  Rule: The bar first

    Scenario: LT1. Reference scenes, a reference frame for each, and the numbers
      Given four scenes (bench/light/scenes.json): Berlin, gambit's people-ingame (the street and faces, four skies),
        the FLOOR3 room (interior, light jumps), a kitchen (gambit's first-person table, fp-lab-table: contact shadows,
        window light; the anything film's kitchen doesn't render on 11-17, every composition in content/anything
        throwing "Can't find variable: raea") and the glade (foliage, sun through leaves)
      And the plan
        | crate                 | file                       | lines | job                                                       |
        | raea-light            | src/lib.rs                 | 260   | `raea light check`: scenes, references kept, paths, live  |
        | raea-light            | src/metric.rs              | 190   | stops on 8x8 blocks, worst tile, flicker against the shot |
        | raea-render-raytrace  | src/force.rs               | 70    | RAEA_LIGHT: every camera on one path                      |
        | raea-render-raytrace  | src/lib.rs                 | 55    | restart_pathtracer, Solari wanted when forced             |
        | raea-render           | src/lib.rs                 | 40    | restart_pathtracer for the frame loop                     |
        | raea-runtime-export   | src/bevy_frame.rs          | 340   | RAEA_LIGHT_SPP: a frame drawn N times, accumulated afresh |
        | bevy_solari           | src/pathtracer/extract.rs  | 40    | raea: the path tracer's reset one-shot, as Solari's       |
        | raea-cli              | src/cli.rs                 | 384   | the `light` verb                                          |
      Then each has a converged reference frame from raea's path tracer (LT5), in the same units and exposure; until
        then Bevy's <Pathtracer> stands in where it can draw the scene (FLOOR3, the glade)
      And Berlin is the main scene: every step is aimed at it first and shown on it before and after (Shane, 11-17)
      And `raea light check` renders each on each path and reports distance to the reference (mean and worst tile,
        in stops), temporal stability over a moving shot, and the live frame time at 1080p (half size waits on a
        switch that draws the 3D smaller: the live 3D draws at its composition's size whatever the stage's, so
        `raea play --size` measured the same frame, 11-17)
      And measured 11-17 on this Mac: FLOOR3, stills 1500 and 4000 against a 2048-sample <Pathtracer> reference (44 s
        a frame), the distance on 8x8-pixel blocks, flicker against the path-traced shot 1500-1504, live 6 s
        unthrottled at 1080p
        | scene  | path      | mean stops | bias  | worst tile | flicker | live ms a frame |
        | floor3 | raster    | 1.379      | -0.80 | 3.37       | 0.101   | 18.0            |
        | floor3 | traced-ao | 1.379      | -0.80 | 3.37       | 0.101   | 18.6            |
        | floor3 | solari    | 0.959      | -0.95 | 2.62       | 0.113   | 47.6            |
      And what they say: Solari is nearer the truth than raster but costs 2.6 times its frame (M11); raster's error
        is patchy. traced-ao is raster here (FLOOR3's camera has no <AmbientOcclusion>). The shot held still, so its
        flicker is the reference's own noise, about 0.10 stops at 2048 samples: a moving shot is still to pick.
        Berlin and the kitchen have no reference yet (M2, M8); the glade waits on a re-run

  Rule: The world radiance cache first

    Scenario: LT2. What a frame of probe rays costs on Berlin
      Given a grid of probes around the camera, each tracing its share of a ray budget through systems/rt a frame,
        a hit shaded by a flat stand-in (albedo 0.5, facing the ray) lit by the sun (one shadow ray) and the sky (a
        miss), each probe keeping a running mean
      Then the cache writes Bevy's irradiance volume (M13) each frame, behind RAEA_GI=1, nothing changed unset
      And on Berlin the GPU time of the probe pass is measured at 64k, 128k, 256k and 512k rays a frame, with the
        frame's total, so the rest is sized from what this Mac's ray queries actually cost (D13)
      And Berlin is shown in raster, raster with the cache, and the cost table, before anything else rests on it

    Scenario: LT3. A ray hit knows its surface
      Then each TLAS instance carries a record: its mesh's vertex and index slices in Bevy's mesh allocator, its
        transform and its surface-cache cards, so a hit has its normal (the triangle's, then the mesh's own) and a
        place to look up what it looks like (D3)
      And the world radiance cache becomes whole (D6): nested grids around the camera, each probe's irradiance and
        distance on an octahedral map, rays a frame shared across the levels, a hit reading the cache for its own
        bounce; on Berlin, light no longer leaks through a facade into a room, and its cost is in LT2's table

    Scenario: LT4. The surface cache
      Given every MaterialExtension writes the deferred G-buffer with its own surface code (D4; procmat's is first,
        then ShaderExt's `surface` hook, toon, tint, paint, grid, froxel glass), and wear's weathering keeps its
        second UV set there
      Then each mesh is captured into cards (D5) through that pass, again only when it or its material changes
      And a ray's hit reads its albedo, normal and emission from its cards: Berlin's snow bounces white, a lit window
        throws warm light into the street at dusk

    Scenario: LT5. raea's path tracer, the reference
      Then a path tracer on systems/rt takes its first hit from the camera's G-buffer and its bounces from the
        surface cache, its lights as the GI's (D7, D11), and converges over RAEA_LIGHT_SPP draws
      And `raea light check` uses it for every reference; Berlin, in its four skies, and the kitchen get theirs, and
        Bevy's <Pathtracer> leaves the check
      And LT1's numbers are taken again on all four scenes, raster and raea GI

  Rule: Every light, everything in it

    Scenario: LT6. Every light feeds the GI
      Then a hit is lit by the sun and the sky, point, spot and rect lights and emissive surfaces, each with a shadow
        ray (a few lights sampled by their power where there are many), in raster's units (D12)
      And the kitchen's window panels and FLOOR3's ceiling panels bounce as the reference has them; the kitchen's
        stand-in "bounce the renderer doesn't trace" panel can go

    Scenario: LT7. Characters posed in the ray scene
      Given RT5's trial poses CPU twins (another lane)
      Then a <Character> is in systems/rt's TLAS as posed, every frame: skin, garments, head parts, and hair cards
        with their alpha masks tested where a ray meets them
      And people block and bounce light: the street darkens round their feet, their faces take the street's bounce,
        and M7's brightening is explained

  Rule: Detail, reflections, occlusion

    Scenario: LT8. Screen probes
      Then a probe every 16 pixels from the G-buffer traces short rays through the TLAS, ends them in the world cache,
        keeps its history across frames by motion, and is filled in between by depth and normal (D6)
      And indirect diffuse is applied per pixel in place of the irradiance volume; Berlin within 0.25 stops mean and
        0.5 worst tile of its reference in every sky, with no more flicker than raster

    Scenario: LT9. Reflections
      Then smooth surfaces trace their reflections, hits shaded from the surface cache and the world cache; rough
        ones read the probes; reflection probes captured by hand go (D8)
      And the snow's sheen, wet stone and window glass on Berlin's street measure within LT8's bar

    Scenario: LT10. Occlusion from the GI alone
      Then SSAO and traced AO are off wherever raea GI is on, and <AmbientOcclusion> means nothing extra there (D9)
      And contacts on LT1's scenes (feet on the street, the mug on the table) are as dark as the reference's

  Rule: Surfaces and image

    Scenario: LT11. Skin, hair and cloth as shading models
      Then skin (subsurface, from sdk/character's skin look), hair cards (two lobes) and cloth (sheen, without
        11-17's velvet haze at grazing angles) are shading models in the deferred pass, direct light and the GI both
        going through them, replacing sdk/character's `lit` hooks (M14)
      And Berlin's faces are checked against the reference in every sky

    Scenario: LT12. Speed and image
      Then the caches keep their own history, a temporal filter of ours steadies the frame, and MetalFX's upscaler
        draws it from fewer pixels on the Mac
      And the game meets D13's budget at 1080p on this Mac with raea GI on, measured live on Berlin, FLOOR3 and the
        kitchen, and the editor's live frame meets docs/realtime.feature's
      And quality tiers scale it for smaller GPUs, each tier's numbers in LT1's table

  Rule: One renderer

    Scenario: LT13. Solari gone
      Then every project that lights with Solari (FLOOR3, Stay, the anything films, gambit's trials) renders with raea
        GI, each shown before and after to its owner, and vendor/bevy_solari, MetalFX's denoiser path (RT4) and
        RAEA_LIGHT's solari path are removed

    Scenario: LT14. A lighting change shows its numbers
      Then a commit that changes light, shadow, occlusion, exposure or a shading model, in any lane, says LT1's numbers
        before and after in its message, and the owner is told (M6)
