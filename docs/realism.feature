@spec
Feature: raea Light: one realism system for shading, ray tracing and occlusion, measured against a reference
  Shane, 2026-11-17, on gambit's characters in the game's own light: "how do we get UNREAL ENGINE level detail and
  quality"; then "before anything we have to figure out the shader/raytracing/ambientocclusion of the engine we want to
  use. right now we seem completely misaligned and need to centralize the strategy to get SOTA results"; then "what if
  we could build our own thing for raea that was a complete realism like system like unreal engine".

  This file is that system's one strategy, its owner's backlog and its bar. It replaces choosing a lighting path per
  project. docs/raytracing.feature (RT1-RT6: guide data, Solari, MetalFX, DLSS) and docs/shaders.feature (SH0-SH11)
  stay where their steps are recorded; anything they decide about light, occlusion or how a surface is shaded is
  decided here first.

  What Unreal's realism is, and where raea stands (11-17):
    | Unreal                         | what it does                                   | raea today                                                   |
    | Lumen                          | bounce light, reflections, occlusion: one GI   | five partial paths (M1)                                      |
    | MegaLights, RT shadows         | many lights, traced shadows                    | Solari's ReSTIR DI; not in the games                         |
    | Substrate                      | one material model in every pass               | a hook runs in the forward pass only (M2)                    |
    | skin, hair, cloth models       | subsurface skin, strand hair, cloth sheen      | skin in raster only; no hair or cloth model                  |
    | TSR, denoisers                 | a clean, stable frame from few rays            | MetalFX on the Mac (RT4); nothing on Windows                 |
    | Nanite                         | unlimited geometry                             | not needed yet                                               |

  What was found (measured 11-17 on gambit's people-ingame.tsx, the four heroes in the baked city, four skies; M5 Pro):
    M1. Five lighting paths, none whole: a flat AmbientLight (CityShot); Bevy's SSAO (a white veil and bright face
        outlines on the heroes); raea's own traced AO (systems/rt, its own TLAS: its pipeline failed to build on Metal,
        a ray query declared in a loop, fixed 11-17); Solari (Bevy's ReSTIR, vendored with raea's patches; FLOOR3 and
        the anything films light every shot with it); a captured environment (gambit, by hand: six views a sky
        stitched into a panorama). Two TLAS builders run side by side: systems/rt's and bevy_solari's.
    M2. A surface hook runs in the forward pass only (SH0c, 10-28: "the main pass alone runs the author's code").
        Solari lights the deferred G-buffer, which gets StandardMaterial's raw values: gambit's world shaders keep a
        surface's kind in the vertex colour's alpha, so under Solari the snow drew teal and the windows red and cyan.
        The hooked meshes are in the traced scene (the city's soft traced shadows show under Solari; apparently Solari
        tags them before systems/shader swaps their material), but its hits read the same raw StandardMaterial.
    M3. Characters aren't lit by traced light: Solari skips skinned and morphed meshes (RT5); systems/rt's TLAS takes
        them in their bind pose (arms out). A trial (RAEA_SOLARI_DYNAMIC=1, solari.rs, uncommitted, another lane)
        poses CPU twins: they then shadow and bounce, but stay raster-lit themselves. The twins hold on Metal for a
        skinned StandardMaterial part (gambit's first-person hands shadowed the table correctly over a 16 s take), but
        the character skin is an ExtendedMaterial and isn't traced at all: gambit's first-person arms bake their skin
        into a plain StandardMaterial to get into the traced scene.
    M4. Nobody owns it: docs/control.feature lists ray tracing's owner as "-". Each project tunes its own light by eye
        (FLOOR3's light-jump rules, the kitchen's rig, gambit's LIFT and ev).
    M5. No reference: no frame is measured against ground truth. Bevy's <Pathtracer> exists but reads the same raw
        materials, so it can't be one for hooked worlds.
    M6. Shading changes land without a lighting check: the shared tree's build of 11-17 evening draws gambit's faces
        brighter than raea-signed of 03:59 in 17% of the pixels of a dawn close-up. The cause is a fix (60b8152a86,
        11:57: a shaded part with no model beside it got its shader only when a model spawned), so head parts now get
        the skin shader they were meant to have; but no lighting owner measured it, and every people check rendered
        that day with raea-signed showed heads without their skin shading.
    M7. Cost, offline at 1080p with readback: raster 0.2 s a frame, Solari 1.0 s (20 frames, wall clock). Live frame
        time isn't measured; docs/realtime.feature asks <= 250 ms ray-traced at half size.
    M8. Traced AO (fixed build, reach 0.6 m) darkens contacts and creases as it should (7% of the line's pixels, to
        -0.23 stops), but also brightens 7-8% of a face close-up, which occlusion alone can't do: unexplained.
    M9. Solari ignores PointLight and RectLight: it lights from directional lights, emissive meshes and the
        environment. Projects stand emissive spheres and sheets in for them by hand (gambit's SetRooms `traced`:
        L = flux/(4 pi^2 r^2) for a bulb, flux/(pi A) for a sheet), so the same scene's lamps differ between paths.
    M10. The hook gap isn't ShaderExt's alone: procedural materials (systems/procmat) also named no deferred shader,
        so under Solari they drew their flat base colour (the kitchen's oilcloth lost its checks); a fix is in
        progress on another lane (deferred_fragment_shader returning procmat.wesl, whose prepass branch already
        writes the G-buffer). Every MaterialExtension raea has (ShaderExt, procmat, toon, tint, paint, grid,
        froxel glass) needs the same check.
    M11. Per-frame CPU geometry freezes in a render: a <BufferMesh> whose vertices change each frame is built once
        and never rebuilt (engine/render spawn.rs apply_rows updates live values only), so traced and raster scenes
        both keep its first shape.

  Decisions:
    D1. The engine stays Bevy 0.20 on wgpu (docs/gpu.md, 11-10): no backend of our own. raea Light is ours: the
        light transport, the material model, the reference and the bar, built on Bevy's raster and wgpu's ray queries
        (Apple silicon on Metal, RTX and RDNA2+ cards on Vulkan).
    D2. One light-transport core: Solari's ReSTIR DI (direct light, traced shadows, many lights) and ReSTIR GI with
        its world cache (bounce), the family of Unreal's MegaLights and NVIDIA's RTXDI. raea already vendors it with
        its own patches; it becomes raea's to change, not a dependency to wait on. A probe GI of our own (DDGI) would
        be an older generation, so it's only the fallback (D10).
    D3. One traced scene: Solari's TLAS. systems/rt's passes (traced AO, the sun and sky passes) read it, and its
        own BLAS and TLAS building goes.
    D4. One material, three places: a surface hook runs in the forward pass, in the deferred G-buffer pass and where
        a ray hits it. At a hit, where the hook can't run, a material proxy stands in: the hook's own average albedo,
        roughness and emission per kind, baked from it. This reverses SH0c's main-pass-only rule.
    D5. Everything is in the traced scene: characters as posed twins (RT5's trial now; skinning into the BLAS on the
        GPU later), morphs, alpha-tested hair cards (RT6's masks).
    D6. Occlusion isn't an effect of its own: under traced light it comes out of the bounce. Traced short-range AO
        stays for the raster fallback and as a crease term; SSAO goes.
    D7. Skin, hair and cloth are shading models (BSDFs) with a raster and a traced form each, not post effects.
    D8. Denoise and upscale per platform: MetalFX on the Mac (RT4), DLSS Ray Reconstruction through Bevy's `dlss`
        feature on NVIDIA, a temporal filter of our own elsewhere; a denoiser-only history reset for light jumps.
    D9. Ground truth: <Pathtracer> with the same materials (it needs D4) is the reference. The bar is distance to it
        and frame-to-frame stability on fixed reference scenes, not a look judged on a still.
    D10. Fallback where the GPU has no ray queries, and for live play if the frame budget isn't met: raster with
        irradiance volumes and reflection probes that raea captures from the same scene (as gambit's panorama was
        made by hand), in the same units and exposure, so switching paths doesn't change brightness.
    D11. One set of units: lux, cd/m², EV, and one exposure model for every path. 11-17's captured environment had
        to be scaled 1.36, 0.13, 0.02 and 0.006 per sky to match the flat ambient: a sign there's no shared model.
    D12. One owner (proposed 11-17: the session that wrote this file, until the orchestrator or Shane names one). A change to
        light, shadow, occlusion or a shading model lands with before and after on the reference scenes (LT1).

  Background:
    Given Bevy 0.20 with bevy_solari vendored (vendor/bevy_solari, changes marked "raea:") and objc2-metal-fx
    And the house rules: builds through `lane build --`, commits through `but`, the shared raea-signed rebuilt only
      after telling the sessions that render with it

  Rule: The bar first

    Scenario: LT1. Reference scenes, a reference frame for each, and the numbers
      Given four scenes: gambit's people-ingame (the street and faces, four skies), the FLOOR3 room (interior, light
        jumps), the kitchen (contact shadows and occlusion) and the glade (foliage, sun through leaves)
      Then each has a converged <Pathtracer> frame as its reference, in the same units and exposure; gambit's, whose
        world is drawn by hooks the path tracer can't run (M5), only once LT2 and LT3 put them in the traced path
      And `raea light check` renders each with the current path and reports distance to the reference (mean and
        worst tile, in stops), temporal stability over a moving shot, and the live frame time at 1080p and half size
      And the numbers for today's paths (flat ambient, captured environment, traced AO, Solari) are recorded here as
        the baseline before any step below lands

  Rule: One material, one scene

    Scenario: LT2. A surface hook in the deferred G-buffer
      Given ShaderExt's hooks run only in the forward pass (M2), and procmat had the same gap (M10)
      Then every MaterialExtension in raea names a deferred fragment shader that runs its own surface code (procmat's
        is the first, from another lane)
      And a hook that reads a second UV set keeps it in the deferred pass (wear's weathering recipes read three baked
        cause maps through TEXCOORD_1: under Solari without this, a shot draws no wear)
      Then ShaderExt has a deferred fragment shader that runs `surface` and writes Bevy's G-buffer from the PbrInput
        (as pbr_deferred_functions does), and a prepass that writes the hooked normal
      And under Solari, gambit's snow, facades and windows draw as they do in raster (LT1's scenes, by eye and by
        distance to raster's base colour)

    Scenario: LT3. A hooked mesh's ray hits shaded by a material proxy
      Given hooked meshes are traced, their hits shaded from StandardMaterial's raw values (M2), and they're traced only
        because Solari tags them before the hook's swap removes MeshMaterial3d<StandardMaterial> (an order, not a rule)
      Then a hooked mesh is traced by rule, and its hits are shaded from a material proxy: per kind, the hook's mean
        albedo, roughness and emission, baked from the hook's own output over the mesh's UVs
      And a lit window bounces warm light into the street at dusk, and the snow's bounce is white, not teal

    Scenario: LT4. One TLAS
      Then systems/rt's traced AO and its sun and sky passes bind Solari's TLAS, and systems/rt builds none of its own
      And a frame builds one acceleration structure, measured

    Scenario: LT5. Characters in traced light
      Given RT5's trial (CPU twins) on another lane
      Then a <Character> under Solari is traced as posed, every frame, garments, head parts and its skin (an
        ExtendedMaterial today, untraced: M3) included
      And its own pixels are lit by Solari (in the G-buffer, LT2), not raster-lit
      And people shadow the street and each other, and the street's bounce reaches their faces

    Scenario: LT5b. Every light type in traced light
      Given Solari ignores PointLight and RectLight (M9)
      Then a PointLight, SpotLight and RectLight are sampled by Solari's ReSTIR DI as the lights they are (or stand in
        as emissive geometry made by raea, not by each project), in the same units as raster
      And a scene's lamps measure the same in raster and under Solari on LT1's scenes (within 0.25 stops)

  Rule: Occlusion, surfaces, image

    Scenario: LT6. One occlusion story
      Then under Solari no AO effect is added; in the raster fallback traced AO (LT4's TLAS) replaces SSAO, and M8's
        brightening is found and fixed first
      And <AmbientOcclusion> means traced AO wherever the device has ray queries

    Scenario: LT7. Skin, hair and cloth as shading models
      Then skin (subsurface, from sdk/character's skin look), hair cards (anisotropic, two lobes) and cloth (sheen,
        no velvet haze at grazing angles: 11-17's wool needed reflectance 0.22 by hand) each have a raster form and a
        traced form, checked against the reference on LT1's faces

    Scenario: LT8. Denoising and upscaling beyond the Mac
      Then Bevy's `dlss` feature (Ray Reconstruction) runs on NVIDIA, a temporal filter of our own elsewhere
      And a denoiser-only history reset exists for light jumps (FLOOR3's rule of 0.2 a frame stays the fallback)

  Rule: Live and fallback

    Scenario: LT9. Live frame budget
      Then LT1's live frame time meets docs/realtime.feature's budget on an M-series Mac and an RTX card, or the
        game plays on LT10 and Solari draws the trailer, cutscenes and stills

    Scenario: LT10. Captured probes for the raster path
      Then raea captures irradiance volumes and reflection probes from the scene itself (six views at each probe,
        filtered by Bevy's generator), in LT1's units, and draws with them where there's no ray tracing
      And switching a scene between LT10 and Solari moves LT1's mean by less than 0.25 stops

  Rule: Keeping it aligned

    Scenario: LT11. A lighting change shows its numbers
      Then a commit that changes light, shadow, occlusion, exposure or a shading model says LT1's numbers before and
        after in its message, and the owner is told (M6)
