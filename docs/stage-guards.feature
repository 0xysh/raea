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

    Scenario: SG0. The repros become tests, and they fail today
      Given the plan
        | crate              | file                                    | lines | job                                                                                  |
        | raea-render (test) | engine/render/tests/frame_pure_3d.rs    | 160   | each repro rendered as one sequence and frame by frame alone; frame N of both compared (SSIM, per part); fails while any part differs |
        | raea-render (test) | engine/render/tests/diag/*.tsx          | 160   | kept-props, kept-props2, kept-props3, glyph-atlas-probe moved in from the project, with what each shows |
      Then the test fails on main today on radius, half_size, Text3D letters, RichText words, colour and size, the row
        taken over, and the atlas's cut glyph, and passes on transform scale, material colour, pictures and the reveal
      And every scenario below that the engine fixes turns one of its failures green

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
      Then kept-props3.tsx passes: the new RichText draws its own words in its own colour, and the old one is gone
      And a part that unmounts while another mounts beside it changes nothing else's row
      # Status 11-21 (branch stage-guards): text blocks are rows by identity; a_new_text_never_takes_an_old_ones_row
      # passes. Fires, fog volumes, plates and the other kinds a live frame diffs whole still take their frame's id.

    Scenario: SG3. The atlas allocates before it draws
      Given bevy_rich_text3d 0.8 reads the atlas's size before it allocates a glyph (tess_glyph)
      When raea patches it ([patch] in Cargo.toml, a vendored copy, with the upstream issue filed)
      Then glyph-atlas-probe.tsx draws every glyph whole with no warm-up text
      And projects/mobile-ads components/stage.tsx Warm() comes out

    Scenario: SG4. A part that won't despawn fails the render
      # Status 11-21: a live scene now drops the rows under another first (a block's glyphs, a part's components), in a
      # respawn and when rows are forgotten, so a block no longer takes its glyphs' entities before their rows go.
      Then "Could not despawn entity" is an error naming the part and the frame, never a log line
      And a 13,000-frame 3D render's memory stays within 10% of its first minute's

  Rule: The SDK stops allowing it

    Scenario: SG5. One clock
      Given the plan
        | crate        | file                              | lines | job                                                                                  |
        | @raea/render | sdk/render/src/3d/text.tsx        | +12   | RichText refuses an `at` or `exitAt` outside its Sequence's frames                   |
        | @raea/render | sdk/render/src/3d/clock.ts        | 40    | the same check for every part that takes a frame (Video's startFrom, Animate3d keys) |
      Then a reveal set after the composition ends is refused at its first frame: "RichText 'quote': at 1013 is after
        this composition's last frame (149). Give `at` in this Sequence's frames (useCurrentFrame), or put the scene
        in a <Sequence from>."
      And the 3D guide says a scene is a <Sequence>, never a frame passed down as a prop
      # Status 11-21 (branch stage-guards): RichText refuses an `at` or `exitAt` at or past its Sequence's last frame,
      # naming the prop and the fix. clock.ts (the same for the other parts that take a frame) is left.

    Scenario: SG6. Lines are lines
      Given the plan
        | crate        | file                              | lines | job                                                                                  |
        | @raea/render | sdk/render/src/3d/line.tsx        | 140   | Line3d: points, width, drawn up to p, round joins, one mesh (a swept outline); Tube for a round section |
      Then a curve is one part, solid at every frame, and the 3D guide's curve recipe uses it
      And SG12 flags a chain of more than 8 parts named `NAME-i` whose ends touch as "a line made of parts: Line3d"

    Scenario: SG7. A camera path that can't lurch
      Given the plan
        | crate        | file                              | lines | job                                                                                  |
        | @raea/render | sdk/render/src/camera/path.ts     | 160   | cameraPath(keys): keys timed in frames, a spline paced by time so speed carries smoothly across a key, `hold` keys where it stops, an ease only at the ends; pos, look and fov at any frame |
      Then cam-diag.ts on a path built with it finds no speed change over 1.5x across a key and no framing reversal
        within 2 s
      And the 3D guide's camera recipe uses it, and hand-written paths are left to the check (SG13)

    Scenario: SG8. Labels that read
      Given the plan
        | crate        | file                              | lines | job                                                                                  |
        | @raea/render | sdk/render/src/3d/label.tsx       | 120   | Label: type drawn in the page at a part's projected point, from the engine's own camera; sized in pixels of the frame (at least 22 at 1080); kept clear of the parts' projected boxes |
      Then type that points at a part is never at an angle, never tiny and never on top of the part
      And RichText stays for type that is itself part of the scene (a sign, a page in a book)

  Rule: The render fails when a frame would look broken
    Each check below reads the frame's scene (as systems/contact does) and the page's layout, runs in `raea render`
    and `raea check`, and reports the part, the frame and the fix, as P1 unless it says otherwise.

    Scenario: SG9. A frame looks the same however it is reached
      Given a render of more than 60 frames with 3D in it
      When raea render re-renders a sample alone: every 300th frame, each chunk's first, and the frame after any
        frame where more than 20 parts mount or unmount
      Then any part whose pixels differ beyond the encoder's noise is an error naming the part, the frame and the props
        that differ
      And this catches SG1–SG4 and any later fault of the same kind before a person sees it

    Scenario: SG10. 3D type that reads
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

    Scenario: SG11. 2D type stays off the parts
      Then a page text box that covers more than 10% of a 3D part's projected box is an error, unless one of them says
        allowOverlap
      And on the film this flags the words over the curves at 2:08 and the headline on the AppLovin card at 6:40

    Scenario: SG12. Nothing snaps
      Then a part whose projected size changes by more than 25% from one frame to the next, with no cut, is an error
      And so is a RichText whose words change without a remount, until SG1 lands
      And on the film this flags the red dot at 1:10.06 and every chunk's first frame in a curve's drawing

    Scenario: SG13. A camera that settles
      Then the camera's speed changing by more than 2x within 3 frames, or its framing reversing (in then out, one way
        then the other) within 1.5 s, is an error, and 1.5x is a warning
      And a camera still moving while type is up past the type's reading time is a warning
      And `raea fingerprint things`'s lurch check folds into this one, so it runs on every render

    Scenario: SG14. The frame is composed
      Then the part a shot is about (one a Label or pin points at, else the nearest named part) cut by the frame's edge
        for more than 10 frames is an error
      And under 8% of the frame showing any part, for more than 1 s, is an error
      And on the film these flag the AppLovin logo cut at 6:32, the dashboard cut at 0:40 and 7:00, and the empty frames
        at 3:40, 6:04 and 6:08

    Scenario: SG15. A broken render can't be posted
      Then `raea show` refuses a render whose record holds an open SG9–SG14 error, naming each, unless the agent passes
        `--despite CHECK "why"`, which the person sees on the item
      And the agent's hooks run SG10–SG14 on a composition's stills after each save (`raea check --quick`), so the
        fault shows before the first full render

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
