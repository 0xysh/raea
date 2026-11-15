Feature: Procedural arms and hands: a character reaches for, takes, holds, carries and throws things
  Set by the user 2026-11-14: the character SDK's arms and hands should grab, hold and move things procedurally,
  built in Rust for the engine from a mixture of musthafa45/Ik-Object-Holding, NVlabs/SOMA-X and whatever else
  helps, with research into what else would broaden the character SDK (bodies, interactions, movement), and a look
  at patch.gg.

  Where this sits beside docs/character.md: that plan's actions (C5, C6) are played from captures, with IK only to
  hold a contact. This is the other half: an action with no capture of it. The same rules hold where they can:
  every solver reports how far it moved the pose and how far it missed (principle 2), the whole action is a
  function of time (principle 9), and the body's own motion (a clip, a walk) comes in untouched and is only changed
  where a hand needs it.

  What the sources gave:
  - Ik-Object-Holding (MIT, Unity C#, 484 lines of script): hold points found by casting a ray from each side of
    the holder back into the thing's collider (ColliderEdgeDetector); hands eased onto them with two-bone IK and
    an aim; the thing parented to a hold transform in front of the chest and drawn to it; drop and throw. Its
    easing is a per-frame lerp (frame-rate dependent, not frame-pure) and its hands have no fingers and no palm
    turn. Ported as: hold points in the thing's own frame with the surface's facing, eases as curves of time.
  - SOMA-X (Apache-2.0, NVIDIA): a canonical body and hand rig that other body models map onto. For this work it
    gave the swing-twist split with half-angle stabilisation and the twist shares along a segment (0.05, 1/3,
    2/3, 0.95 over four twist joints), used here to move a wrist's twist to the forearm. Its other parts are for
    later: a wrist-local hand layer with a learned articulation prior (SOMAHand.npz: what hand shapes are
    natural), pose inversion (joint turns fitted to points), and body-model interop including Anny. It needs
    PyTorch and Warp; nothing of it runs in the engine as it is.
  - Around SOMA: Kimodo (Apache-2.0 code; the SEED weights under NVIDIA's Open Model License, said to allow
    commercial use; the SMPL-X weights are research-only) makes motion from text and constraints. MotionBricks, a
    real-time in-betweener, has preview code in GR00T-WholeBodyControl with no plain licence on the repository
    and sample motions under an evaluation-only licence: not usable until that is read. BONES-SEED (motion in
    SOMA's format) has its own licence, gated: unread. ProtoMotions (Apache-2.0) trains physically simulated
    characters.
  - Rust IK crates: bevy_mod_inverse_kinematics (to Bevy 0.16), bevy_fabrik (no multi-chain), bevy_ik (Bevy
    0.9). None is on Bevy 0.20, none does a two-bone solve that keeps a clip's twist or reports what it changed,
    so the solver here is ours: 170 lines.
  - Grasping research (Lightning Grasp and the like) is for robot hands, has no plain licence, and solves
    grasp stability, which a character doesn't need. A character's hand needs to look right: the palm on the
    surface, the fingers wrapped to the shape. Curling each joint until its bone meets the surface does that.
  - patch.gg: a place to publish browser games and mods, with a CLI (pch), an MCP server and an asset library of
    50,000 open-licensed models, characters and sounds. It is not an engine and has nothing for IK or hands.
    Its installer writes a "use Patch for games" section into ~/.claude/CLAUDE.md for every agent on the machine,
    which would put Patch ahead of raea in every session here: not installed. Its asset library conflicts with
    the rule that every model in a raea piece is made by us. Worth taking from Patch: its open repositories.
    anymove (Apache-2.0) captures motion for any rigged creature from one generated two-view video (about $3 a
    usable move, needs an NVIDIA card with 12 GB, 58% of clips pass) and is the one that fits: animals and
    creatures, where raea has no captures. crowdbricks (Apache-2.0) batches MotionBricks and inherits its
    licence question. Publishing a raea playable on Patch is a distribution question, not a character one.

  Rule: Built

    @done
    Scenario: HG1. An arm reaches a target, and says what it cost
      # 11-14, built: systems/grab arm.rs. On the stand-in body, 600 random targets inside the arm's reach are hit
      # within 0.1 mm; a target 20 cm past the arm is missed by 20 cm and flagged; a wrist already on its target
      # turns nothing; the pole puts the elbow on its side of the line within a degree; a reach 12 cm past the arm
      # lands within 5 mm by leaning the clavicle, chest and spine, each under its cap, and names them.
      Given a pose of the game_engine rig and a place for a wrist
      When the arm is solved for it
      Then the wrist is there when the arm reaches, the elbow bends about the hinge it had, and the pole chooses
        where the elbow points
      And past the arm's length the trunk leans, by capped shares, and the report names every joint turned
      And a wrist's twist is shared with the forearm, the hand staying where it was
      And the plan:
        | crate         | file        | lines | job                                                                  |
        | raea-sys-grab | src/rig.rs  | 210   | the rig, poses, forward kinematics, a stand-in body                  |
        | raea-sys-grab | src/arm.rs  | 180   | two-bone IK, the reach that leans, the forearm's twist               |

    @done
    Scenario: HG2. Hold points are found on the thing itself
      # 11-14, built: shape.rs, hand.rs. A box 36 cm wide gets a hold on each side face, facing out; a cup 8 cm
      # wide gets one hand; a turned thing's holds are on its surface from wherever the holder stands; the palm's
      # skin lies on the hold within 0.1 mm, facing into it, fingers the way the holder faces.
      Given a thing as a ball, a box, a capsule or a cylinder, placed and turned, and where the holder stands
      When its holds are asked for
      Then a thing wider than a hand's span gets a hold on the holder's left and one on the right, where a ray
        from that side meets its surface, each with the way the surface faces
      And a narrower thing gets one hand's hold
      And the holds are kept in the thing's own frame, so they ride with it
      And the plan:
        | crate         | file         | lines | job                                                             |
        | raea-sys-grab | src/shape.rs | 120   | solids as signed distances: casts, normals, clearance           |
        | raea-sys-grab | src/hand.rs  | 220   | holds, the wrist that lays a palm on one, the fingers' curl     |

    @done
    Scenario: HG3. Fingers wrap what they hold
      # 11-14, built: hand.rs `curl`. Each joint, knuckle first, opens past rest and bends until its bone meets
      # the surface. On a carried box at least three fingers of each hand are on it and none is over 12 mm inside.
      # The curl is solved with the hand at its hold, not where the arm has got it to, so it doesn't flicker as
      # the hand comes in (it did: 24 degrees a frame).
      Given a hand at its hold on a thing
      When its fingers close
      Then each finger bends until it meets the surface, a finger that meets nothing closes loosely, and a finger
        that is inside the thing even opened is reported, not hidden

    @done
    Scenario: HG4. Reach, grab, lift, carry, put down or throw, as one function of time
      # 11-14, built: take.rs `perform`. From the grab to the release both palms stay within 6 mm of their holds
      # at 30 frames a second; a cup carried while the body turns 34 degrees and back stays upright the whole
      # way and is set down within 0.1 mm of its place; a thrown box flies, lands on the floor and stays; the
      # body is back on its own motion 0.4 s after letting go. A frame asked for twice, in any order, is the same
      # frame; no joint of the body turns over 6 degrees in a 60th of a second, no finger joint over 20.
      # `cargo run -p raea-sys-grab --example strip -- out.svg` draws a take as a filmstrip of stick figures.
      Given the body's own motion as a pose at any time, a thing, and its holds
      When a frame of the take is asked for
      Then the hands leave the body's motion, come onto the holds from off the surface, and close
      And the thing is drawn to a carry place off the chest and rides there, turned only with the body's facing
      And a put sets it down at a place, a throw lets it go with a velocity and it falls to the floor
      And every frame says how far a wrist missed and how far the body was turned from its own motion
      And the plan:
        | crate         | file              | lines | job                                             |
        | raea-sys-grab | src/take.rs       | 210   | the take's timeline                             |
        | raea-sys-grab | tests/grab.rs     | 290   | the measures above, on the stand-in body        |
        | raea-sys-grab | examples/strip.rs | 90    | a take drawn as an SVG filmstrip                |

    @done
    Scenario: HG5. A character in a scene takes a thing
      # 11-14, built: raea/character's `take(skel, base, spec, t)` (sdk/character/src/take.ts) makes the native call
      # character.take (runtime/calls), which is systems/grab's `take_json`; the answer is the pose for <Character>
      # and the thing's place. Asked as JSON it is the same frame as asked in Rust (a test). Rendered on teal, the
      # Character SDK project's drawn character, 1.70 m (the sheet3d project's compositions/take.tsx, out/take.mp4;
      # first on a 1.81 m MakeHuman person, the same code): a box taken off a table with both hands and thrown, a
      # cup moved across the table in one hand, upright, the fingers round it, a box lifted from the floor onto
      # the table; 735 frames, no errors. Shane hasn't watched it.
      Given a <Character> and a thing in a 3D scene
      When the scene says the character takes it (which thing, when, then put where or thrown how)
      Then the engine works out each frame's pose and the thing's place, in a render and in live play alike
      And TypeScript reaches it as one native call (core/calls), typed, with the report per frame
      And the body's own motion is whatever the character was playing
      And the plan:
        | crate            | file                      | lines | job                                             |
        | raea-sys-grab    | src/json.rs               | 120   | a frame of a take asked for as JSON             |
        | sdk              | sdk/character/src/take.ts | 80    | take(): the call made, typed                    |

    @done
    Scenario: HG6a. Hands stay out of the scene's solids
      # 11-14, built: body.rs `clear`, a take's `avoid` list. A wrist's place is moved the least that keeps the
      # wrist, the palm and the fingers' reach 2 cm clear of every solid named; the wrist's path arcs up 6 cm and
      # leaves a hold backing off its surface. With a table under a cup, no point of the hand is in the table at
      # any frame of the take, and keeping clear costs the hold under 6 mm. Seen on turbo: the hand comes over the
      # table's edge onto the cup and leaves it without passing through the top.
      Given a take with the solids round it named (a table, a shelf)
      Then no frame has the hand inside one, and the hold is still kept

    @done
    Scenario: HG7a. A low thing is taken by bending, the feet planted; and the body is told where to stand
      # 11-14, built: body.rs `stoop` and `stand_for`. For hands that must go lower than 0.7 of an arm below the
      # shoulders, the hips come down and the trunk bends at the hips and along the spine, by as much as the hands have got along their reach, so the body rises again as the thing is
      # lifted. Leg IK keeps both ankles where they stood within 5 mm and the feet's turn. A box on the floor is
      # held within 2 cm; the hips come down over 30 cm and are back up with the box carried. `stand_for` gives
      # the place a forearm's length off the thing's near face, facing it; standing there both hands reach.
      # character.take answers it as `stand`. Seen on turbo: a box lifted from the floor onto a table. Not right
      # yet: the body itself keeps out of nothing (the head went into the table that stood 38 cm in front). The
      # squat was retuned after the first render folded the head to the knees: the hips take three quarters of it
      # (to 0.55 m), the trunk bends to 49 degrees, and the head looks at the thing.
      Given a thing lower than the arms reach, or out of reach
      Then the body bends and squats for a low one with its feet where they stand
      And for a far one the take says where to stand and which way to face

    @done
    Scenario: HG4a. The carry is against the body
      # 11-14: a two-handed thing rides 10 cm under the chest joint, 20 cm plus half its size in front, so the
      # elbows bend and the lift shows (it was carried at arm's length, hardly lifted).
      Then a carried box is held against the belly with the elbows bent

    @done
    Scenario: HG5a. The rest of both sources, in Rust
      # 11-14, built. From Ik-Object-Holding: its Interactor is interact.rs `pick` (the nearest thing in a radius
      # that the eyes see past the scene's solids; the original takes the first collider found), and its head aim
      # is body.rs `look` (neck two fifths, head the rest, 55 degrees at most), on through a take: the eyes go to
      # the thing as the hands do. With HG2 and HG4 that is every script of the original that isn't Unity input or
      # animator glue. From SOMA-X: fit.rs `fit` is the analytical pass of its pose inversion on marks instead
      # of mesh vertices (joints refitted parents first, each aligned to what it carries; the root on its own
      # marks), and arm.rs `twist_joints` is its procedural twist-joint evaluator (shares 0.05, 1/3, 2/3, 0.95;
      # an upper segment undoing its own joint's twist). 20 random poses, every joint turned up to 50 degrees,
      # are recovered from joint heads and bone ends with every joint within 4 mm. raea/character's
      # `fitPose(skel, marks)` is the native call character.fit; on turbo a second body posed only from the
      # first one's joint positions stands as the first does through a take (fingers aside: positions alone
      # don't give a bone's twist). A wrist stops at 70 degrees and the frame says so (`limited`); the fingers
      # point the level way the arm comes, so it is rarely asked for more.
      # Not converted, and why: SOMA's body and hand layers, identity models and correctives are a different
      # mesh and a 78-joint rig driven by PyTorch and Warp, and raea's body is MakeHuman's; its hand
      # articulation prior (SOMAHand.npz, 32 components over SOMA's 24 finger joints, bind-relative) needs
      # SOMA's finger frames mapped onto game_engine's 15 finger joints before a score means anything (HG8);
      # its Lie-algebra and autograd refinements fit mesh vertices through skinning, which marks don't need.
      Then a body can choose what to take, look at it, and be posed from positions seen

  Rule: Next, in order

    @needs:HG5
    Scenario: HG6. The rig's real limits and the body's real shape
      # sdk/body has both in TypeScript (limits.ts: ranges checked against 26,288 captured frames; shape.ts,
      # shell.ts: the skin). systems/grab has caps on the trunk's lean and finger ranges, and HG6a for the hands.
      Given a solved arm
      Then every joint is inside the range sdk/body's limits give, the elbow and wrist first
      And the forearm, the elbow, the trunk, the head and the held thing stay out of the body and the scene's
        solids, by the least move
      And one copy of the limits serves both the TypeScript and the Rust

    @needs:HG5
    Scenario: HG7. The body walks to the thing
      # HG7a says where to stand; nothing walks there yet.
      Given a thing out of reach
      Then locomotion (sdk/body's) is asked to arrive at the take's standing place, facing it, and the take
        starts when it has
      And a low thing's bend is checked against the library's pick-low and lift captures, and takes its shape
        from them where they fit (the back straighter, the head up)

    @needs:HG5
    Scenario: HG8. Things with a way to be held
      Given a thing that is held one way (a mug's handle, a sword's grip, a door's handle, a phone, a bag's strap)
      Then its holds come with the thing, as places and hand shapes, from the 35 captured hand poses
        (character/captured.ts) where one fits
      And a thing with no holds of its own gets HG2's
      And a hand shape is scored against SOMA-X's hand prior before it is kept

    @needs:HG5
    Scenario: HG9. Using what is held, and handing it over
      Then a held thing can be aimed, swung, drunk from, set to the ear, pushed, pulled and turned (a door, a
        drawer, a lever), each a path for the thing that the hands follow
      And a thing passes from one hand to the other, and from one character to another, with no frame where
        nobody holds it

    @needs:HG5
    Scenario: HG10. Weight
      Then a heavy thing is held low and close, the trunk leans back against it, the elbows straighten, and the
        lift is slower; a light one moves with the wrist
      And a throw's wind-up and follow-through scale with it

  Rule: Wider, for the character SDK

    Scenario: HG11. Creatures' motion from generated video
      # patch-gg/anymove, Apache-2.0. raea's animals have no captures (the dog and cat were called stiff, 11-14).
      Given a rigged creature made skeleton first
      Then its moves are captured from a generated two-view video, the joints read in both views, the rig fitted
      And the model that reads joints runs on hardware a user has, or the step is a cloud one and says so

    Scenario: HG12. Motion from words, between poses
      # Kimodo (Apache-2.0 code, weights' licence to be read in full first). MotionBricks only if its licence
      # turns out to allow it.
      Then a situation the library has no capture of is filled by generated motion on the rig, retargeted from
        SOMA's skeleton once, and judged against the captures by the same detectors

    Scenario: HG13. A body that reacts
      Then a pushed or struck character gives way and recovers (the ragdoll looked at again, with a physics
        crate chosen for the whole engine first: there is none in the workspace today)
      And a hand or a foot can brace on a wall, a rail or the floor as a contact found the way holds are
