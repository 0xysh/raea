@spec
Feature: raea Light: raea's own real-time realism renderer, measured against raea's own reference
  Shane, 2026-11-17, on gambit's characters in the game's own light: "how do we get UNREAL ENGINE level detail and
  quality"; then "before anything we have to figure out the shader/raytracing/ambientocclusion of the engine we want to
  use. right now we seem completely misaligned and need to centralize the strategy to get SOTA results"; then "what if
  we could build our own thing for raea that was a complete realism like system like unreal engine"; and, once FLOOR3
  measured Solari at 2.6 times raster's frame: "i want our own version our own raea realism renderer then that doesnt
  use solari ... if solari is too slow then we need to look elsewhere". Then, on what to aim it at: "another strategy
  we should ink into the plan is looking for realistic photos we want to model after and try to recreate them exactly
  and reverse engineer realistic photos by configuring the rendering system, shaders, raytracing, ambient occlusion
  etc"; "they should be full color high definition color photos. real photos that correctly test the realism and
  especially for the berlin environment, snow/fall/summer looks equally not just winter".

  This file is that renderer's one strategy, its owner's backlog and its two bars. raea's own path tracer says whether
  raea draws a scene correctly; real photographs say whether the scene, its materials, its light and its camera
  together look real. A match to one photo found by tuning that shot proves nothing, so what a photo teaches goes into
  the renderer, a shading model or a measured material, and has to hold on photos it wasn't found on (D15, D16). It
  replaces choosing a lighting path per project. docs/raytracing.feature (RT1-RT6) and docs/shaders.feature
  (SH0-SH11) keep the steps they recorded; anything about light, occlusion, reflections or how a surface is shaded is
  decided here first. The first version of this file (11-17, commits ttn, xok, quu on branch `realism`) built on
  Solari; the second (ypl) planned surface-cache cards and probe grids; this one follows the research.

  The design is the one hardware Lumen (Unreal 5.9), AMD's GI-1.0 and Snowdrop ship to get ray-traced light at game
  frame rates (docs/realism-research.md, 11-17). Raster draws what the camera sees and its direct light. The light
  that bounces is gathered in three stages: a ray marched on the screen first, which sees every material exactly as
  raster drew it; then a hardware ray against a coarser ray scene; ending in a world-space cache that keeps what it
  learned and is refreshed a little each frame, at a cost set by a ray budget, not by the screen. Solari instead
  traces every pixel every frame (ReSTIR), which is why it costs what it does. From a study of vork (an offline
  renderer for agents, 11-17; the study is kept out of this public repository, its source being under a licence that
  allows reading, not redistribution) raea takes three ideas: a ray hit runs the material's own surface code, every
  indirect pass corrects a cheap baseline so each lands on its own, and a dropped effect says so.

  What Unreal's realism is, and what raea Light makes of it:
    | Unreal                     | what it does                                         | raea Light                                          |
    | Lumen screen traces        | the first and cheapest ray, on the screen            | screen traces (LT8)                                 |
    | Lumen World Cache (5.9)    | a hash grid on surfaces: far and multi-bounce light  | the world cache (LT2 measures, LT3 builds)          |
    | hit lighting, cards        | what a ray sees where it hits                        | the material's own code at the hit, and cache cells |
    |                            | (5.9's hardware path dropped the cards)              | learning colours from the G-buffer (LT4)            |
    | Lumen screen probes        | detail and contact: sparse probes from the G-buffer  | screen probes (LT8)                                 |
    | ShortRangeAO               | contact occlusion at full resolution                 | the screen trace's own short rays (LT10)            |
    | Lumen reflections          | traced for smooth surfaces, the cache for rough      | the same, SSR first (LT9)                           |
    | shadow maps, MegaLights    | direct light and its shadows                         | Bevy's raster lights and shadow maps; light tiles   |
    |                            |                                                      | for the GI's many lights (LT6)                      |
    | skin, hair, cloth models   | subsurface skin, hair, cloth sheen                   | shading models the GI goes through (LT11)           |
    | TSR                        | a clean, stable frame from few rays and fewer pixels | caches' own history, our temporal filter, MetalFX   |
    |                            |                                                      | upscaling on the Mac (LT12)                         |
    | path tracer reference      | ground truth with the full materials                 | raea's path tracer, full materials at every bounce, |
    |                            |                                                      | and the film tier (LT5)                             |

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
    D2. No Solari renderer. It goes from every live, render and reference path once raea's replaces each use (LT13),
        and vendor/bevy_solari with it. Three of its parts are raea's to keep: its hash-grid world cache, its light
        tiles and its TLAS set up on the GPU are taken into systems/rt as raea's own code first (LT3, LT6).
    D3. One ray scene: systems/rt's TLAS, kept from frame to frame and written on the GPU from an instance buffer,
        holding everything opaque, characters posed (LT7), and coarser than the raster scene: merged tiles, compacted
        static BLASes, builds budgeted inside the frame's own encoder, a proxy beyond a distance (Nanite and Snowdrop
        trace fallback meshes). Each instance's custom index points into a record table (its mesh's vertex and index
        slices, transform, material, averaged albedo and emission), so a hit knows its normal and what it looks like
        (LT3).
    D4. Every material writes the deferred G-buffer with its own surface code (procmat first, then ShaderExt and the
        rest: M10). That one pass serves the screen traces and probes (LT8), the world cache's learning (D5), the
        contact occlusion (D9), MetalFX and the reference's first hit (LT5). A `lit` hook can't run there (M14): skin
        and hair become shading models (LT11) instead.
    D5. What a ray sees comes in three tiers, and together they are the surface cache every ray reads (LT4):
        (1) its instance record's averaged albedo and emission, which is enough for snow to bounce white and a lit
        window to glow; (2) the world cache's cell, which learns albedo, normal and emission from the G-buffer while
        its surface is on screen, so any hook, any season and any weather reach the rays exactly as raster draws them,
        and keeps them once the surface is off screen; (3) the material's own surface code run at the hit, from the
        raster's own vertex buffers through the instance record (vork's way), wherever a tier below isn't exact
        enough. No cards: Unreal 5.9's hardware path dropped them, and they'd need recapturing every time a season's
        snow changes.
    D6. raea GI gathers in three stages: a screen trace, then the TLAS, ending in the world cache. The world cache is a
        hash grid of cells on the surfaces rays hit, sized by their distance from the camera, split by normal
        direction, and refined near a hit, so light doesn't leak through Berlin's thin facades (octahedral probe
        grids leak there, depth maps and all). Its updates are a ray budget a frame (LT3). Screen probes: one every 16
        pixels at the internal resolution, a quarter refreshed a frame, filtered in probe space, not on the screen
        (LT8). Until they land, LT2's probe grid reaches the frame through Bevy's irradiance volume (M13), as a
        measuring step only: it has no visibility test.
    D7. Every light feeds the GI: a hit is lit by the sun and the sky, point, spot and rect lights and emissive
        surfaces, in raster's units, the many lights sampled by their power through light tiles with one shadow ray a
        hit (gaslit streets at dusk have hundreds).
    D8. Reflections: SSR first, blended by its confidence into traced rays below a roughness of 0.4, hits shaded by
        D5's tiers and ended in the world cache; rough surfaces read the screen probes.
    D9. Occlusion is counted once. Contact occlusion comes at full resolution from the screen trace's own short rays
        (the share that escapes), applied to indirect light only, and the sun gets screen-space contact shadows; the
        world cache and the probes give the rest. Bevy's SSAO and the 128-ray traced AO leave live play (traced AO
        stays as a tool for checking). Each indirect pass corrects a cheap baseline on demodulated light, so each can
        land, fail or be turned off without breaking the frame. M7 is explained first.
    D10. Skin, hair and cloth are shading models (subsurface skin, two-lobe hair, sheen cloth) that direct light and
        the GI both go through.
    D11. Ground truth is raea's own path tracer on systems/rt, with full materials at every bounce and never the GI's
        caches: a reference that shares a cache can't measure it. Its first hit comes from the camera's G-buffer. Where
        a hook can't run at a hit yet, it uses D5's best tier and that error is measured on FLOOR3 against Bevy's
        <Pathtracer>. A white-furnace scene checks it conserves energy. The same tracer is the film tier: no history,
        N samples a frame, OIDN on the GPU, a fixed seed, so a frame renders the same every time. Until it lands (LT5),
        Bevy's <Pathtracer> stands in for scenes it can draw (no hooks: FLOOR3, the glade).
    D12. One set of units: lux, cd/m², EV, and one exposure model for every path.
    D13. The game's budget, for Shane to confirm (M15): 60 fps at 1080p on an M-series Mac (16.7 ms a frame), with
        raea GI's share at most 5 ms of it; 30 fps the floor; quality tiers (rays a frame, probe spacing, screen-probe
        resolution) scale it down for smaller GPUs. The editor's live frame keeps docs/realtime.feature's budget.
        Speed is designed in from LT2 on, not left to LT12: FLOOR3's raster frame alone is 18.0 ms. The frame is drawn
        at 67-75% inside and upscaled by MetalFX at 2x or less, the GI is traced at that size or below, and the
        G-buffer stays thin (on a tile-based GPU it can't be memoryless).
    D18. On Metal, wgpu's ray queries become `intersection_query`, the slower of Apple's two paths, and wgpu can't
        refit an acceleration structure (ALLOW_UPDATE builds as normal). Once LT2 measures the gap, the hottest trace
        kernels may be raea's own Metal (`intersector`, refit) through passthrough, as docs/gpu.md already runs
        MetalFX, with wgpu's path kept for Windows and Linux: nothing in raea Light is Mac-only.
    D19. A dropped effect says so. A render or play that drops or substitutes a requested light effect warns with a
        code in its report, and every render reports its exposure (clipped, near black, mean) and GPU ms per pass.
        M2's teal snow was an effect dropped in silence.
    D14. One owner: the session that wrote this file, on lane `realism`. A change to light, shadow, occlusion,
        exposure or a shading model, in any lane, lands with its LT1 numbers before and after (LT14).
    D15. The second bar is real photographs (LT15-LT18): full colour, high definition (at least 1920 pixels wide; the
        camera's original where it can be had, raw where it exists), modern, under a licence that allows commercial
        use (public domain, CC0, CC BY, CC BY-SA; author and licence kept with each), each with what rebuilding it
        needs: focal length and camera (its field of view), shutter, aperture and ISO (its exposure in EV, D12), time
        and place (the sun) and the weather. Berlin first, and every season equally: snow, autumn, summer and a wet
        day, at several hours, in streets and squares whose buildings stand from before 1918. Then faces in daylight,
        a kitchen table by a window, a room and a glade, so each LT1 scene has photos of its kind.
    D16. A photo teaches the renderer, never one shot. A residual is traced to its pass (direct, sky, GI, reflections,
        occlusion, a shading model, tone) by turning them on one at a time, and fixed there or in a measured material
        value (fresh snow's albedo, sandstone's, brick's, skin's). Half the photos are held out by place and season;
        a change stays only if the held-out half comes nearer too.
    D17. Each photo is recreated as a set of its own (Shane, 11-17: "you'll have to model completely new sets"):
        modelled by raea from nothing, object for object, at the photo's season, day and hour, so it can be compared
        pixel for pixel and not only by region. The realism lane makes and owns the sets, in gambit, for every
        season. Berlin's world now has seasons: its look takes one (snow, autumn, summer, wet), and the lanes that
        build the city and its rooms (exteriors, interiors, first person lab) keep what they make ready for every
        season instead of fixing it to winter.

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

  Rule: The second bar: real photographs

    Scenario: LT15. The photo set
      Given D15
      Then bench/light/photos.json lists each photo: its source page and file URL, author, licence, size, season,
        place, hour and weather, its EXIF (camera, focal length, shutter, aperture, ISO, time, position where given)
        and a checksum; `raea light photos` fetches the files into raea's cache, outside git
      And Berlin has at least 32, eight a season (snow, autumn, summer, wet): half where the model stands (the
        Gendarmenmarkt, the Friedrichstadt, Unter den Linden), half in tenement streets of the same age (Prenzlauer
        Berg, Kreuzberg, Friedrichshain); every other scene kind has at least eight
      And half the set, by place and season, is held out (D16), and the set is shown to Shane before it's used

    Scenario: LT16. A set for each photo
      Given D17, and first eight Berlin photos (one fitted and one held out a season), then the rest of the set
      Then each has a set in gambit, modelled by us (no downloaded models): its buildings, street, ground, trees,
        cars, signs and people where they matter to the light, at the photo's true place where it's in the city, with
        its materials' values measured (albedo, roughness) rather than picked by eye
      And Berlin's look takes a season (snow, autumn, summer, wet: bare or leafed trees, fallen leaves, dry, wet or
        snowed ground, the sun of that day), which the sets use and the city can use too
      And each set is shown beside its photo before it's measured

    Scenario: LT17. A photo matched
      Then `raea light check` takes a photo and its set: the camera from the photo (its field of view from EXIF, its
        pose solved from points marked on both), the sun from its time and place, and the exposure from its EV
      And it compares in linear light (a raw photo decoded linear, a JPEG's sRGB curve undone): first at the EXIF EV
        unchanged, a test of the units, then with one fitted exposure
      And on 8x8 blocks, as LT1's references are, and by region: sky, sunlit wall, wall in shade, ground, snow,
        foliage, glass and faces, masked on both by one local segmentation model. Each region gets its luminance in
        stops and its colour (Oklab); the ratios between regions (sun to shade, the shade's blue, lit snow to shaded
        snow); local contrast by scale; and the darkening into contacts and across shadow edges
      And each photo's numbers go into LT1's report and table beside the reference's, the fit and held-out halves apart

    Scenario: LT18. Photos reverse-engineered
      Then each region whose residual is over a third of a stop, or whose colour is visibly off, has its cause found
        by passes (D16) and fixed in the renderer, a shading model or a measured material, with the photos' numbers
        before and after, fit and held out
      And Berlin's held-out photos come within 0.33 stops on every region ratio in every season, and Shane sees each
        season's photos beside their renders

  Rule: What rays cost first, then the ray scene and the world cache
    The order the steps are built in (docs/realism-research.md, ranked): LT2; LT3; LT4's first tier; LT8's screen
    traces and probes; LT10; LT6; LT7; LT9; LT5; LT4's later tiers; LT11; LT13. LT12's targets and LT14 hold from
    LT2 on.

    Scenario: LT2. What a frame of rays costs on Berlin
      Given a grid of probes around the camera, each tracing 64 rays a round through systems/rt, a hit shaded by a
        flat stand-in (albedo 0.5, facing the ray) lit by the sun (one shadow ray) and the sky's diffuse light, a miss
        seeing the sky, each probe keeping a running mean: a measuring step, not the world cache (D6)
      Then the grid writes Bevy's irradiance volume (M13) each frame, behind RAEA_GI=1, nothing changed unset
      And on Berlin, by GPU timestamps: the probe pass at 64k, 128k, 256k and 512k rays a frame; the TLAS and BLAS
        builds; and the raster frame by pass, so the rest is sized from what this Mac actually costs (D13)
      And the same probe pass through an owned Metal kernel (`intersector`), so D18's gap is a number
      And Berlin is shown in raster, raster with the grid, and the cost table, before anything else rests on it

    Scenario: LT3. The ray scene, and the world cache
      Then systems/rt keeps one TLAS from frame to frame, written on the GPU from an instance buffer (Solari's TLAS
        setup, made raea's); each instance's custom index points into a record table (D3); static BLASes are
        compacted; BLAS builds go in the frame's encoder on a budget; a streamed tile merges into a few BLASes; past a
        distance an instance is its proxy, and far off its tile's HLOD
      And the world cache is a hash grid on the surfaces rays hit (D6), from vendor/bevy_solari's world cache made
        raea's: cells sized by distance, split by normal, refined near a hit, filled by one path an 8x8 tile of at
        most three bounces ending in last frame's cache
      And on Berlin light no longer leaks through a facade into a room, and the TLAS's cost and the cache's are in
        LT2's table

    Scenario: LT4. What a ray hit sees
      Given every MaterialExtension writes the deferred G-buffer with its own surface code (D4; procmat's is first,
        then ShaderExt's `surface` hook, toon, tint, paint, grid, froxel glass), and wear's weathering keeps its
        second UV set there
      Then a hit reads D5's tiers: its record's averaged albedo and emission first; then its world cache cell's,
        learned from the G-buffer while on screen, in every season and weather; then, where a tier isn't exact, the
        material's own surface code, run at the hit from the raster's vertex buffers through a generated switch over
        the scene's materials
      And Berlin's snow bounces white, and a lit window throws warm light into the street at dusk; a hook that can't
        run at a hit is named by D19's warning

    Scenario: LT5. raea's path tracer, the reference and the film tier
      Then a path tracer on systems/rt takes its first hit from the camera's G-buffer and runs full materials at every
        bounce (D11), its lights as the GI's (D7), converging over RAEA_LIGHT_SPP draws; a white furnace checks it
      And `raea light check` uses it for every reference; Berlin, in its four skies, and the kitchen get theirs, and
        Bevy's <Pathtracer> leaves the check
      And the film tier renders FLOOR3, Stay and the anything films with no history, N samples a frame, OIDN and a
        fixed seed, the same frame every time
      And LT1's numbers are taken again on all four scenes, raster and raea GI

  Rule: Every light, everything in it

    Scenario: LT6. Every light feeds the GI
      Then a hit is lit by the sun and the sky, point, spot and rect lights and emissive surfaces in raster's units
        (D12), the many lights through light tiles sampled by power (Solari's, made raea's) with one shadow ray a hit
        (D7)
      And the kitchen's window panels and FLOOR3's ceiling panels bounce as the reference has them; the kitchen's
        stand-in "bounce the renderer doesn't trace" panel can go
      And Berlin at dusk, with its gas lamps and lit windows, bounces as its reference does; stochastic direct light
        for the many lamps (MegaLights' way) is the step after

    Scenario: LT7. Characters posed in the ray scene
      Given RT5's trial poses CPU twins (another lane), which stays a trial: they cost the bus every frame
      Then a <Character> is skinned once a frame in compute, into one buffer that raster and its BLAS both read (and
        the shadow and prepass stop skinning again), traced through a reduced proxy, rebuilt each frame until D18's
        refit; hair cards go in with one alpha an instance, rejected by blue noise
      And people block and bounce light: the street darkens round their feet, their faces take the street's bounce,
        and M7's brightening is explained

  Rule: Detail, reflections, occlusion

    Scenario: LT8. Screen traces and screen probes
      Then a probe every 16 pixels at the internal resolution, a quarter refreshed a frame, traces each ray on the
        screen first (it reads raster's own lit pixels), then through the TLAS, ending in the world cache; it keeps
        its history by motion and is filtered in probe space, then filled in between by depth and normal (D6)
      And indirect diffuse is applied per pixel in place of the irradiance volume, as a correction to the baseline
        (D9); Berlin within 0.25 stops mean and 0.5 worst tile of its reference in every sky, with no more flicker
        than raster

    Scenario: LT9. Reflections, glass and fog
      Then SSR runs first and hands what it can't find to traced rays below a roughness of 0.4, hits shaded by D5's
        tiers and ended in the world cache; rough surfaces read the screen probes; reflection probes captured by hand
        go (D8)
      And window glass and the fog volume read the world cache for their light
      And the snow's sheen, wet stone, puddles and window glass on Berlin's street measure within LT8's bar

    Scenario: LT10. Occlusion counted once
      Then contact occlusion comes at full resolution from the screen trace's own short rays, on indirect light only,
        and the sun has screen-space contact shadows; Bevy's SSAO and the 128-ray traced AO are off wherever raea GI
        is on, and <AmbientOcclusion> means nothing extra there (D9)
      And contacts on LT1's scenes (feet on the street, the mug on the table) are as dark as the reference's

  Rule: Surfaces and image

    Scenario: LT11. Skin, hair and cloth as shading models
      Then skin (subsurface, from sdk/character's skin look), hair cards (two lobes) and cloth (sheen, without
        11-17's velvet haze at grazing angles) are shading models in the deferred pass, direct light and the GI both
        going through them, replacing sdk/character's `lit` hooks (M14)
      And Berlin's faces are checked against the reference in every sky

    Scenario: LT12. Speed and image
      Then the caches keep their own history, a temporal filter of ours steadies the frame, and MetalFX's temporal
        upscaler draws it from 67-75% of its pixels on the Mac; frame interpolation is a bonus for 120 Hz screens,
        never part of the budget; on Windows and Linux a denoiser slot takes DLSS-RR or FSR's in MetalFX's place
      And the game meets D13's budget at 1080p on this Mac with raea GI on, measured live on Berlin, FLOOR3 and the
        kitchen, and the editor's live frame meets docs/realtime.feature's
      And quality tiers scale it for smaller GPUs, each tier's numbers in LT1's table

  Rule: One renderer

    Scenario: LT13. Solari gone
      Given its world cache, light tiles and TLAS setup are already raea's (LT3, LT6)
      Then every project that lights with Solari (FLOOR3, Stay, the anything films, gambit's trials) renders with raea
        GI or LT5's film tier, each shown before and after to its owner, and vendor/bevy_solari, MetalFX's denoiser
        path (RT4) and RAEA_LIGHT's solari path are removed

    Scenario: LT14. A lighting change shows its numbers
      Then a commit that changes light, shadow, occlusion, exposure or a shading model, in any lane, says LT1's numbers
        (references and photos, held out apart, and GPU ms per pass) before and after in its message, and the owner is
        told (M6)
      And every render and play reports D19's dropped effects and its exposure, so an effect lost in silence is caught
