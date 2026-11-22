@spec
Feature: Stage guards: a 3D render an agent can't break
  Shane, 2026-11-21, on the mobile-ads essay's 3D cut (projects/mobile-ads, compositions/film2): "every aspect it was
  totally broken looking and text, cards, overlays, etc all were so incredibly broken we must have uncovered something
  bad". Then: "diagnose the 3D issues with text, glyphs, layouts, staging, blocking and why things are flipping back
  and forth on scale/size ... how to make sure an agent using raea cannot physically make them again".

  What the diagnosis found (11-21). The repros are in projects/mobile-ads: compositions/diag/kept-props.tsx,
  kept-props2.tsx, kept-props3.tsx, compositions/glyph-atlas-probe.tsx, and the measures tools/cam-diag.ts,
  tools/text-diag.ts and tools/at-diag.ts. Each repro renders in a second or two.
    1. A 3D part's shape and a RichText's words, size and colour are made once, when the part's row is first spawned
       (engine/render/src/engine/spawn.rs, spawn_3d). A later frame's values reach the live row (apply_rows), but only
       placement, visibility, material and pictures are applied from it (runtime/scene3d sync.rs sync_meshes and
       sync_pictures; systems/text3d sync_texts).
       - kept-props.tsx: over 30 frames, a Sphere's radius, a Cuboid's half_size and a RichText's words hold their
         first frame's values. Growth by the transform's scale follows the frame, and so does a part remounted each
         frame.
       - kept-props2.tsx: a RichText's colour and size and a Text3D's letters hold too. A material's colour, an
         ImagePlane's picture and RichText's own enter reveal follow the frame.
       - In the film: every curve broke into dashes. stage.tsx's Ribbon is a chain of boxes whose newest box grows,
         and each box kept the length it was born with. A count read "0.0 MB" until its chunk ended, and typed code
         lines stopped wherever their first frame left them.
    2. A RichText mounted where another unmounted takes over its row. It draws the old words at its own place, and its
       own glyph colours land on the old glyphs. In kept-props3.tsx, "FROM ITS STORE PAGE" drew as "FOUR OPENINGS"
       with one letter red. In the film this gave "A RENDE? ENGINE", "TALL · ?N APPS", "1.? MB" and a store card headed
       "…ENINGS". Every cut glyph traced so far came from this, not from the atlas.
    3. There were two clocks. RichText's `at` is a frame of useCurrentFrame, so it counts in its composition's or
       Sequence's frames. The film's scenes took their own frame as a prop and passed scene frames. Rendered in
       150-frame chunks, a reveal at scene frame 1013 never began, which is why the store quote never appeared
       (at-diag.ts). Rendered whole, every reveal came at once. 21 texts had an `at`.
    4. The film was rendered in chunks, each in its own process, to get past 8 below. At each chunk's first frame,
       every frozen value snapped to that frame's value and then froze again. The network's red dot grew to full size
       in one frame at 1:10.06, the first frame of a chunk. Together with rows changing hands (2) and the camera (5),
       this is the "flipping back and forth on scale".
    5. The camera path (stage.tsx camAt) is a Catmull-Rom through keys set at uneven times, with every segment
       running 0 to 1 regardless of its length. Of 155 keys in 13 paths, 51 see the camera's speed change 2x or more
       (up to 9.4x). The framing reverses 24 times within 2 s: in and then out, or one way and then the other
       (cam-diag.ts).
    6. Type was set in the world. The placement wall's labels were 15–16 px tall at 1080 (text-diag.ts). Other labels
       lay on floors or on pages turned at an angle. The 2D type laid over the 3D didn't know where the parts were:
       words ran over the curves at 2:08, and a headline sat on the AppLovin card at 6:40. 45 entrances used an easing
       that overshoots full size.
    7. The glyph atlas (bevy_rich_text3d 0.8) has its own fault. The glyph whose allocation doubles the atlas is drawn
       into the old bounds and stays cut (glyph-atlas-probe.tsx).
    8. A 13,175-frame render in one process logged "Could not despawn entity" and grew from about 5 GB to 26 GB.

  None of this was caught:
    - raea check passed the film on loudness, freezes and flashes.
    - Its text checks read the 2D page only.
    - The contact check found one 1 cm touch.
    - `raea fingerprint things`, which finds camera lurches and empty frames, wasn't run.

  Each rule below makes one of these impossible to ship. Either the engine stops doing it, or the SDK refuses to write
  it, or the render fails with the part and the frame named.

  Background:
    Given the repros above, each a few seconds to render, and their measures
    And frame purity as control.feature and blitz-bevy H10 state it: a frame is the same however it is reached
    And the per-frame scene the contact check already reads (systems/contact: every part's kind, transform, shape and
      bounds; the camera)

  Rule: SG0. The bar first, before any fix

    @done
    Scenario: SG0. The repros become tests, and they fail today
      Given the plan
        | crate              | file                                    | lines | job                                                                                  |
        | (script)           | tools/frame-pure-3d.sh                  | 50    | each repro rendered as one sequence (PNGs, one process) and four frames alone (--frame N); frame N of both compared, SSIM 0.999 or more, lossless; fails naming the repro and frame |
        | raea-render (test) | engine/render/tests/diag/*.tsx          | 110   | kept-props, kept-props2 (its brand-logo picture row left out), kept-props3 moved in from the project, with what each shows |
        | (L1)               | tools/agents/check                      | +4    | the "3d frame purity" step, when core/scene, runtime/scene(3d), runtime/live, engine/render's engine, systems/text3d or sdk/render/src/3d changes |
      Then the test fails on main today on radius, half_size, Text3D letters, RichText words, colour and size, the row
        taken over, and the atlas's cut glyph, and passes on transform scale, material colour, pictures and the reveal
      And every scenario below that the engine fixes turns one of its failures green
      # Status 11-21: built and green on main after SG1 and SG2 (f4b524f896): every sampled frame of the three repros
      # matches at SSIM 0.9998 or more. The bar is tight enough: a sequence's frame 28 against frame 29 alone scores
      # 0.986, so a part held since its first frame can't pass. Before SG1 the same repros showed frame 0's sphere,
      # box, words, colour and size in a sequence. The atlas's cut glyph is one frame, not a sequence: its repro
      # (glyph-atlas-probe.tsx, a cold atlas against a warm one) came with SG3, and with it a second measure, since SSIM
      # over a whole frame scored the cut letter 0.9996: no more than 16 pixels off by over 96 levels, after an erosion
      # drops lone pixels (a revealed word's edge noise).

  Rule: The engine stops doing it

    Scenario: SG1. A part's shape and its type follow the frame
      The fix sits where rows are made, not in the engine: the engine keeps building a part once, at spawn, and a part
      whose kept columns change is spawned again. An mp4's 3D is made live frame by frame (raea-runtime-scene live.rs);
      a still's and RAEA_3D=program's rows are made from several frames at once (rows).
      Given the plan
        | crate                | file                                   | lines | job                                                                                  |
        | raea-core-scene      | core/scene/src/kept.rs                 | 200   | kept_col: the columns a row keeps from its spawn, per kind; respawns; spawn_sig, a part's kept inputs as one number; split_kept; KeptInputs, the fast path's guard. The live column tables moved here from live.rs |
        | raea-runtime-scene   | runtime/scene/src/live.rs              | +25   | diff: a row whose kept columns changed is dropped with the rows under it and spawned again; a glyph's colour sent as col_r/g/b, as a mesh's is |
        | raea-runtime-scene   | runtime/scene/src/fast.rs              | +3    | the fast path takes the full path when a part's kept inputs changed                  |
        | raea-runtime-scene   | runtime/scene/src/lib.rs               | +1    | a render's mesh, model, panel, decal and text tracks cut where their kept inputs change |
        | raea-runtime-live    | runtime/live/tests/stage_guards.rs     | 160   | the tests: a shape spawned again, new words laid out again, a glyph's colour, a still's own shape and words |
      Then kept-props.tsx and kept-props2.tsx pass SG0
      And a Ribbon of 48 boxes drawn over 90 frames is solid in a sequence render, as it is frame by frame
      And a growing shape's cost is measured (one mesh a frame, cached): a 48-part curve drawn over 90 frames renders
        within 10% of the same frames alone
      # Status 11-21 (branch stage-guards): kept.rs, live.rs, fast.rs and the track cut are built, and
      # runtime/live/tests/stage_guards.rs passes (5 tests; live_emissive and live_sheet still pass). Left: SG0's
      # renders of the repros, and a shader's `three` and `shader` columns, whose uniforms change every frame
      # (frozen uniforms keep their own fix rather than a part spawned each frame).

    Scenario: SG2. A part never inherits another part's row
      The cause was live.rs: a text block kept the id its frame gave it (`text0` is the frame's first block), where a
      mesh, model, panel or decal has its identity's (its name, else what it draws). So a block mounted as another
      unmounted got the old one's row: the engine kept the old words and took the new glyphs' colours.
      Given the plan
        | crate              | file                                  | lines | job                                                                                  |
        | raea-runtime-scene | runtime/scene/src/live.rs             | +3    | text blocks by identity, as the groups are; their glyphs follow their block's id     |
        | raea-runtime-scene | runtime/scene/src/live.rs             | +10   | plates, splats, sequences, fog, fires and grass by identity                          |
        | raea-runtime-scene | runtime/scene/src/lib.rs              | 0     | their tracks cut where what they take whole changes (split_kept)                     |
        | raea-core-scene    | core/scene/src/kept.rs                | +18   | WHOLE: each kind's columns that say what it is, and the props they come from         |
        | raea-runtime-live  | runtime/live/tests/stage_guards.rs    | +30   | a_new_plate_never_takes_an_old_ones_row                                              |
      Then kept-props3.tsx passes: the new RichText draws its own words in its own colour, and the old one is gone
      And a part that unmounts while another mounts beside it changes nothing else's row
      # Status 11-21 (branch stage-guards): text blocks are rows by identity; a_new_text_never_takes_an_old_ones_row
      # passes. Branch stage-guards-6: plates, splats, mesh sequences, fog volumes, fires and grass are rows by identity
      # too, and what each takes whole from its first frame (a plate's clip, a splat's or a sequence's file, a fog's
      # density texture, a fire's simulation, a field's grids) spawns it again when it changes (core/scene kept.rs
      # WHOLE): a_new_plate_never_takes_an_old_ones_row. An unnamed fire or fog has only its order to be known by, so
      # name the ones that come and go. Sprites, 2D shapes and bare entities keep their render ids, which components
      # target.

    @done
    Scenario: SG3. The atlas allocates before it draws
      Given bevy_rich_text3d 0.8 reads the atlas's size before it allocates a glyph (tess_glyph)
      When raea patches it ([patch] in Cargo.toml, a vendored copy, with the upstream issue filed)
      Given the plan
        | crate              | file                                        | lines | job                                                                              |
        | bevy_rich_text3d   | vendor/bevy_rich_text3d/src/tess.rs         | +5    | the atlas image's size read after the glyph is allocated, in both branches (stroked, filled) |
        | (workspace)        | Cargo.toml, vendor/vendor.toml              | +8    | the [patch] and the vendor entry: why, and the exit (an upstream release that reads the size after allocating) |
        | raea-render (test) | engine/render/tests/diag/glyph-atlas-probe.tsx | 35 | "cold" (the film's glyph set into a fresh atlas) and "warm" (six Inter weights first, out of shot) |
        | (script)           | tools/frame-pure-3d.sh                      | +25   | a repro with cold and warm compositions: the two frames must match            |
      Then glyph-atlas-probe.tsx draws every glyph whole with no warm-up text
      And projects/mobile-ads components/stage.tsx Warm() comes out once the project renders on a raea with SG3 (0.1.9)
      # Status 11-21: on a build without the fix the cold probe cuts a Space Mono H (179 pixels off against warm); with
      # it, 0 pixels off (SSIM 0.99996). The upstream issue is drafted, not filed: filing it is a public post.

    @done
    Scenario: SG4. A part that won't despawn fails the render
      # Status 11-21: a live scene now drops the rows under another first (a block's glyphs, a part's components), in a
      # respawn and when rows are forgotten, so a block no longer takes its glyphs' entities before their rows go.
      Given the plan
        | crate              | file                                  | lines | job                                                                              |
        | raea-sys-frame     | systems/frame/src/store.rs            | +25   | despawn: try_despawn; a row whose entity had already gone leaves the store and is named (its kind and id) in the error; the store test |
        | raea-render        | engine/render/src/engine/spawn.rs, engine.rs | +8 | remove_rows_from and Engine::remove_rows return it                           |
        | raea-runtime-export | runtime/export/src/fed.rs, bevy_frame.rs | +4 | a render fails on it, with the frame                                             |
        | raea-runtime-react, raea-web | scene/plugin.rs, lib.rs       | +2    | a plain App and the web player pass it on                                        |
      Then "Could not despawn entity" is an error naming the part and the frame, never a log line
      # Status 11-21 (branch stage-guards-4): built ("frame N: Could not despawn entity: the text part 'quote' had
      # already gone, despawned with another part it hung from"); the store test covers it.

    Scenario: SG4a. A long render's memory stays flat
      Then a 13,000-frame 3D render's memory stays within 10% of its first minute's
      # Status 11-21: not measured yet (SG4's leak fixed; the bar is the proof).

  Rule: The SDK stops allowing it

    @done
    Scenario: SG5. One clock
      Given the plan
        | crate        | file                              | lines | job                                                                                  |
        | @raea/render | sdk/render/src/3d/text.tsx        | +12   | RichText refuses an `at` or `exitAt` outside its Sequence's frames                   |
        | @raea/render | sdk/render/src/3d/clock.ts        | 45    | the same check for every part that takes a frame of its Sequence (Animate3d's first key, BevyComponent's from) |
      Then a reveal set after the composition ends is refused at its first frame: "RichText 'quote': at 1013 is after
        this composition's last frame (149). Give `at` in this Sequence's frames (useCurrentFrame), or put the scene
        in a <Sequence from>."
      And the 3D guide says a scene is a <Sequence>, never a frame passed down as a prop
      # Status 11-21 (branch stage-guards): RichText refuses an `at` or `exitAt` at or past its Sequence's last frame,
      # naming the prop and the fix. Branch stage-guards-4: clock.ts holds the check, and RichText, a mesh's and a
      # material's `animate` (its first key: an animation the Sequence's end cuts short is fine, one that never starts
      # isn't), a lens effect's and MotionBlur's `animate`, and BevyComponent's `from` use it
      # (sdk/raea/conformance/one-clock.test.tsx). Left out, on purpose: Video's startFrom (a frame of the source
      # clip, which can run past the Sequence) and a VolumeSource's from/until (seconds). Screen's `from` isn't
      # checked yet: screenContent isn't a component, so it can't read the Sequence.

    @done
    Scenario: SG6. Lines are lines
      Given the plan
        | crate        | file                              | lines | job                                                                                  |
        | @raea/render | sdk/render/src/3d/line.tsx        | 160   | Line3d: points, width, drawn up to p, round joins, one mesh (a swept outline); Tube for a round section |
        | raea         | sdk/raea/src/mesh.ts              | +2    | raea/mesh exports them                                                               |
        | raea (test)  | sdk/raea/conformance/line3d.test.tsx | 80 | one part at every share drawn, the cut at p, the width and plane, the tube's radius and untwisted rings, caps, refusals |
        | (guide)      | sdk/guides/craft/bevy/meshes.md   | +22   | the curve recipe, and a chain of parts as a common mistake                            |
      Then a curve is one part, solid at every frame, and the 3D guide's curve recipe uses it
      And SG12 flags a chain of more than 8 parts named `NAME-i` whose ends touch as "a line made of parts: Line3d"
      # Status 11-21 (branch stage-guards-5): Line3d and Tube are built, one BufferMesh each, cut at p; renders of a
      # 120-point Tube and an 80-point Line3d growing over 40 frames read smooth with round ends at every frame. The
      # meshes guide's recipe uses them. The flag is raea-sys-stage's line-of-parts (a warning, motion.rs): a chain of
      # more than 8 parts named NAME-i that grows, which on the film names its 108 fields of 16 parts each.

    @done
    Scenario: SG7. A camera path that can't lurch
      Given the plan
        | crate        | file                                    | lines | job                                                                                  |
        | @raea/render | sdk/render/src/camera/path.ts           | 150   | cameraPath(keys): keys timed in frames, a cubic per channel paced by time with one velocity a key (so speed carries across it), `hold` keys where it stops, an ease only at the ends, no channel past a key (monotone); pos, look and fov at any frame; camera(f), Camera3d's props; check(), the keys whose timing still brakes hard |
        | raea         | sdk/raea/src/camera.ts                  | +2    | raea/camera exports it                                                               |
        | raea (test)  | sdk/raea/conformance/camera-path.test.ts | 140  | speed across keys (the per-segment path jumps over 2x on the same keys), keys passed through, ends, holds, no overshoot, Camera3d's props, check(), refusals |
        | (guide)      | sdk/guides/craft/bevy.md                | +7    | rule 10, the camera recipe, uses it                                                  |
      Then the camera's speed is continuous across every key it moves through, and no channel passes a key and comes back
      And on keys that don't set a quick move against a slow one, cam-diag.ts finds no speed change over 1.5x across a
        key and no framing reversal within 2 s the keys don't make themselves
      And path.check() names each key that does, with the fix (more frames for the quick move, or a hold)
      And the 3D guide's camera recipe uses it, and hand-written paths are left to the check (SG13)
      # Status 11-21: the speed is continuous across every key, so a path never jumps; a channel never passes a key and
      # comes back. On the film's 155 keys (cam-diag's measure) framing reversals went from 24 to 5 and speed changes
      # of 2x within 3 frames from 51 to 29. What's left is in the keys: the 5 reversals are turns the keys make
      # within 2 s; 11 of the 29 are stops the keys ask for (a pose given twice); the other 18 sit where a quick move
      # meets a slow one (10 m in 30 frames against 0.7 m in 134), which any path through those keys at those frames
      # must brake for, and check() names exactly those 18 with the fix. One scene's keys weren't in time order: the
      # old path drew them anyway, and cameraPath refuses them.

    Scenario: SG8. Labels that read
      Given the plan
        | crate        | file                              | lines | job                                                                                  |
        | @raea/render | sdk/render/src/3d/label.tsx       | 112   | Label: type drawn in the page at a part's projected point, from the engine's own camera; sized in pixels of the frame (at least 22 at 1080); kept clear of the parts' projected boxes |
        | raea         | sdk/raea/src/camera.ts            | +2    | raea/camera exports Label                                                            |
        | @raea/render | sdk/render/package.json           | +1    | the ./3d/label subpath                                                               |
        | raea (test)  | sdk/raea/conformance/label.test.tsx | 60  | the point through a camera and a cameraPath, ortho, title-safe, px of the frame, refused under 22 |
      Then type that points at a part is never at an angle, never tiny and never on top of the part
      And RichText stays for type that is itself part of the scene (a sign, a page in a book)
      # Status 11-21 (branch stage-guards-7): Label (raea/camera) projects its point through the props the shot's
      # Camera3d gets (a cameraPath's camera(frame), perspective or orthographic) and draws flat page type there: its
      # capitals 32 px at 1080 by default and refused under 22, its box inside title-safe and growing away from the
      # nearer edge, a line and a dot to the point, nothing while the point is behind the camera or out of frame
      # (sdk/raea/conformance/label.test.tsx). It stands `gap` px off the point you give it: give the part's top.

  Rule: The render fails when a frame would look broken
    Each check below reads the frame's scene (as systems/contact does) and the page's layout, runs in `raea render`
    and `raea check`, and reports the part, the frame and the fix, as P1 unless it says otherwise.
    SG10, SG12, SG13 and SG14 are raea-sys-stage (systems/stage): they read each frame's parts through its camera
    (view.rs; lib.rs finds the shots' cuts), run at the end of every render beside the clipping check
    (runtime/export/src/scene.rs), and run without drawing in `raea scene stage`, which moved with lint, explain and
    light out of raea-runtime-export into raea-app-scene (apps/scene) so the export crate keeps room under 2,000 lines.

    Scenario: SG9. A frame looks the same however it is reached
      Given a render of more than 60 frames with 3D in it
      When raea render re-renders a sample alone: every 300th frame, each chunk's first, and the frame after any
        frame where more than 20 parts mount or unmount
      Then any part whose pixels differ beyond the encoder's noise is an error naming the part, the frame and the props
        that differ
      And this catches SG1–SG4 and any later fault of the same kind before a person sees it
      # Status 11-21 (branch stage-guards-8): built for a video with 3D in it, more than 60 frames. At the render's end
      # one more raea renders the sample with `--at` (each frame reached by a jump) and each still is held to the
      # video's frame: a channel off by more than 96 levels through a 3x3 erosion (tools/frame-pure-3d.sh's bar, which
      # drops the encoder's and a glyph edge's lone pixels), on more than 1e-4 of the frame. The largest channel, not
      # luma: the positive control (a box moved by a counter outside the frame) put an orange box where the alone frame
      # has grey floor, 68 levels apart in luma and 116 in blue. A frame that differs is rendered
      # alone (`--frame N`, its own process): still different, a `frame-pure` error naming the frame, the patch's box,
      # the parts under it (the still's manifest now lists each frame's parts on screen, raea-sys-stage's on_screen)
      # and the two pictures, kept in OUT.frame-pure/; matching alone, a `frame-pure-jump` warning (raea's fault, not
      # the composition's). The sample is every 300th frame, the last (most history behind it, so a render under 300
      # frames is sampled too) and the frame after each remount, from the manifest's new `remounts`; at most 64.
      # Left: "each chunk's first" (raea render has no chunks of its own); "the props that differ" (React is
      # frame-pure, so both routes hand the engine the same props: what differs is the engine's state, and the
      # error names the parts instead). RAEA_FRAME_PURE=0 skips it. Measured before building, alone (`--frame N`)
      # against the sequence on four real scenes of 91-180 frames: three match exactly (SSIM 1.0); one under Solari
      # GI differs in SSIM (0.97-0.99, the GI's history) with no patch past the bar after frame 0.
      Given the plan
        | crate               | file                         | lines | job                                                                    |
        | raea-app-direction  | apps/direction/src/pure.rs   | 230   | the sample, the stills and the alone frame, the patch, the parts under it, the findings |
        | raea-app-direction  | apps/direction/src/rendered.rs | +2  | a video render's end runs it beside the text rules                     |
        | raea-runtime-export | runtime/export/src/scene.rs  | +40   | the manifest's `parts` (stills) and `remounts` (longer renders)        |
        | raea-runtime-export | runtime/export/src/lib.rs    | +4    | write_manifest takes them                                              |
        | raea-sys-stage      | systems/stage/src/framing.rs | +12   | on_screen: each part's box on screen, named                            |

    Scenario: SG10. 3D type that reads
      Given the plan
        | crate          | file                         | lines | job                                                                             |
        | raea-sys-stage | systems/stage/src/view.rs    | 118   | the frame's camera (Camera3d's quaternion or a lookAt), a point and a box on screen |
        | raea-sys-stage | systems/stage/src/lib.rs     | 183   | told and check, the frames as the checks read them, shots, runs, findings     |
        | raea-sys-stage | systems/stage/src/text.rs    | 217   | each RichText's capitals on screen, its turn from the camera, its box against title-safe, its reveal, its reading time; `read={false}` for scenery |
        | @raea/render   | sdk/render/src/3d/text.tsx   | +3    | RichText's `read` prop                                                          |
        | raea-app-scene | apps/scene/src/stage.rs      | 38    | `raea scene stage`: every frame stepped without drawing, the checks, errors fail it |
        | raea-app-scene | apps/scene/src/lib.rs        | 59    | `raea scene`, moved from raea-runtime-export (lint and explain) with light.rs  |
        | raea-app-scene | apps/scene/src/light.rs      | 58    | `raea scene light`, moved                                                       |
        | raea-runtime-export | runtime/export/src/scene.rs | 35 | a render's items, and its end checks: clipping, then staging                    |
      Then every RichText and Text3D is checked on every frame it shows:
        | measure                              | limit                                              |
        | cap height on screen                 | at least 22 px at 1080 (scaled to the frame)       |
        | its face against the line to camera  | within 35°                                         |
        | its box                              | inside the title-safe area                         |
        | parts in front of it                 | none covering it                                   |
        | time held still on screen            | its reading time (words ÷ 3 a second, plus 0.5 s) before it moves |
        | its reveal                           | finished before it unmounts                        |
      And on the film these flag the placement wall's labels (16 px), the floor labels (over 35°) and the store quote
        (a reveal that never starts)
      # Status 11-21 (branch stage-guards-6): RichText is checked on every frame (systems/stage/src/text.rs): capitals
      # under 22 px at 1080 are an error; turned past 35° a warning, past 50° an error (squashed under 64%, a floor seen
      # low); outside title-safe a warning; a reveal never started or never finished an error; held still for less than
      # its reading time a warning. `read={false}` leaves scenery out. A still stepped alone (raea check --quick, render
      # --at) is judged for size, angle and safe area, and says nothing of reveals or time. Measured with `raea scene
      # stage` on film2 (13,174 frames, 64 s): 182 small-type errors (code on screens at 3 to 7 px, labels at 17 to 19
      # px), 152 angle errors (195 labels at 60 to 75°: 'ASA RULING · UK' at 74° for 12 s, 'KIT' at 71°), no false
      # error on 33 clean scenes (gambit, sheet3d, lightbench, Stay, floor3, blender-loop). The film was rewritten since
      # the notes: its placement wall no longer exists, so the 16 px labels can't be named. Left: Text3D, and parts in
      # front of the words.

    Scenario: SG11. 2D type stays off the parts
      Then a page text box that covers more than 10% of a 3D part's projected box is an error, unless one of them says
        allowOverlap
      And on the film this flags the words over the curves at 2:08 and the headline on the AppLovin card at 6:40

    Scenario: SG12. Nothing snaps
      Given the plan
        | crate          | file                         | lines | job                                                                             |
        | raea-sys-stage | systems/stage/src/motion.rs  | 311   | cuts; a named part's size or place on screen jumping in one frame, alone; a growing chain of parts named NAME-i (SG6's flag) |
      Then a part whose projected size changes by more than 25% from one frame to the next, with no cut, is an error
      And so is a RichText whose words change without a remount, until SG1 lands
      And on the film this flags the red dot at 1:10.06 and every chunk's first frame in a curve's drawing
      # Status 11-21 (branch stage-guards-6): part-snap (motion.rs) is a named part's size on screen changing over 25%
      # in one frame between steady frames, no cut: an error. Measured first as written, it said 1,446 errors on the
      # film and 1,252 on four clean scenes; what was left out, each a measured false error: a part coming in (no
      # steady frame before it), a name several items carry in one frame, four or more parts at once (the layout
      # changing: the creatures reel's 30 tiles), parts reaching over 6 m (water, floors), and a part whose words or
      # shape change that frame (spawned again as new content, SG1: kept-props2's counter). A jump across a tenth of
      # the frame is a warning (a tile recycled, a part thrown). Now 0 errors on film2 and on 33 clean scenes. Left:
      # the red dot at 1:10.06 and the chunks' first frames were drawn wrong by the engine from right props, which only
      # SG9's re-render sees; and RichText words changing without a remount (SG1).

    Scenario: SG13. A camera that settles
      Given the plan
        | crate          | file                         | lines | job                                                                             |
        | raea-sys-stage | systems/stage/src/motion.rs  | 311   | inside each shot, a speed kink in the camera's travel or turn (2x an error, 1.5x a warning), and a dolly or pan undone within 1.5 s |
      Then the camera's speed changing by more than 2x within 3 frames, or its framing reversing (in then out, one way
        then the other) within 1.5 s, is an error, and 1.5x is a warning
      And a camera still moving while type is up past the type's reading time is a warning
      And `raea fingerprint things`'s lurch check folds into this one, so it runs on every render
      # Status 11-21 (branch stage-guards-6): camera-lurch and camera-reverse (motion.rs) run inside each shot on every
      # render. Speeds under 0.1 degrees or 3 mm a frame are below a change anyone sees; turns are measured by atan2
      # (acos of an f32 dot read 0 and 0.02 degrees on alternate frames and said 48 false lurches on Stay). A reversal
      # is the whole there-and-back within 1.5 s; a dolly in and back out is an error, a pan there and back a warning
      # (a first-person look round). Words up while the camera moves past their reading time are type-time's warning.
      # On film2: 41 lurch errors, each at a key where the stage's Catmull-Rom segments differ in length (its speed
      # jumps there); on the clean scenes, 13 lurches on cameras locked to a walking body (sheet3d) or a first-person
      # walk starting with no ease, and two dolly reversals (soma_dressed's 0.15 m, Stay's 0.84 m in and 0.43 m out at
      # 1:12). Left: `raea fingerprint things`' lurch check, still its own.

    Scenario: SG14. The frame is composed
      Given the plan
        | crate          | file                         | lines | job                                                                             |
        | raea-sys-stage | systems/stage/src/framing.rs | 182   | the share of the frame any part covers (a grid of their boxes), and the shot's subject cut by the edge |
      Then the part a shot is about (one a Label or pin points at, else the nearest named part) cut by the frame's edge
        for more than 10 frames is an error
      And under 8% of the frame showing any part, for more than 1 s, is an error
      And on the film these flag the AppLovin logo cut at 6:32, the dashboard cut at 0:40 and 7:00, and the empty frames
        at 3:40, 6:04 and 6:08
      # Status 11-21 (branch stage-guards-6): framing.rs says both, as warnings for now: the page's own 2D (a headline
      # over an empty 3D) isn't known to them until SG11. Coverage leaves out a slab over 12 m (a floor, a wall: a frame
      # of floor and fog is empty), and a part wholly behind the camera. On film2: empty frames at 1:59, 3:07 and 3:41
      # (the notes' 3:40), and 21 subject cuts. Left: errors once SG11 knows the page, and the subject a Label points at.

    @done
    Scenario: SG15. A broken render can't be posted
      Given the plan
        | crate    | file                              | lines | job                                                                       |
        | -        | engine/cli/src/held.ts            | 82    | the newest render that wrote each file shown, from its project's record, and its errors less what was let through |
        | -        | engine/cli/src/app.ts             | +6    | show refuses before it posts or opens anything; Here lists a folder        |
        | -        | engine/cli/src/show.ts            | +14   | `--despite CHECK "why"`, and the why on the item's ask line                 |
        | -        | engine/cli/conformance/held.test.ts | 69  | refused, cleared by a later render, let through either way, frames and absolute outs, the flag |
        | raea-sys-stage | systems/stage/src/lib.rs    | +12   | a still stepped alone stands for its moment; reveals and reading times only over frames all stepped |
      Then `raea show` refuses a render whose record holds an open SG9–SG14 error, naming each, unless the agent passes
        `--despite CHECK "why"`, which the person sees on the item
      And the agent's hooks run SG10–SG14 on a composition's stills after each save (`raea check --quick`), so the
        fault shows before the first full render
      # Status 11-21 (branch stage-guards-6): held.ts refuses `raea show` of any file whose newest render in its
      # project's record holds an error it wasn't let through with, every error named, before it posts or opens
      # anything (all of a render's errors, not only SG9–SG14's: each one already exited the render 1). `--despite
      # CHECK "why"` passes one, and the why goes on the item's ask line. The quick check's stills are a render's, so
      # the stage checks run on them (a still stepped alone is judged for what a still shows).

  Rule: SG16. The 3D guide says it first
    Scenario: SG16. The guide's 3D rules, before any recipe
      Then sdk/guides (3d.md and craft/floor.md) and the raea skill say, before any recipe:
        | rule                                                                                                 |
        | type goes in the page over the scene; a Label points at a part; RichText only for type in the world  |
        | a scene is a <Sequence>; SDK parts read useCurrentFrame, never a frame passed down as a prop          |
        | a curve is a Line3d; a camera is a cameraPath                                                         |
        | grow a part by its transform's scale until SG1 lands; remount it when its words change                |
        | render frame-pure: SG9 runs on every render; a chunked render is a workaround, never the plan         |
      And each rule names the check that enforces it
      # Status 11-21 (branch stage-guards): sdk/guides/craft/bevy.md's rules 5 and 8-10 say these (grow by scale, one
      # clock, type that reads, a camera that settles). Branch stage-guards-7: rules 4, 5, 9, 10 and 11 name the check
      # that holds each (SG2's names, line-of-parts and part-snap, the type checks and read={false}, camera-lurch and
      # camera-reverse, raea scene stage, frame-empty and subject-cut, raea show --despite), Line3d and cameraPath are
      # the recipes, and craft/floor.md refuses labels written into the 3D, naming Label (SG8). Left: SG9's re-render.
