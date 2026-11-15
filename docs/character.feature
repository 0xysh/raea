Feature: The character SDK as one system: SOMA bodies, Kimodo motion, generated heads, creatures
  Set by the user 2026-11-15, after deciding that people's bodies standardize on SOMA (docs/hands.feature, the
  rule of that date): one session syncs the body, the motion system, the head work and the creature work, which
  three sessions had been building apart. In the user's words, the job is to standardize the body on SOMA, Kimodo,
  the head work and how creatures are managed and rigged in the character SDK; to make a whole system for
  locomotion, motion matching, staging and AI controllers for NPCs if Kimodo is the standard; and, once that is
  settled, a generative or procedural clothing system, perhaps with the mannequin as the base input and an image
  model on top, layered.

  This file is the map. The detail stays where it is: docs/hands.feature (arms and hands, the SOMA rule),
  docs/creatures.feature (four legs), docs/drawn-characters.feature (a drawing as a character),
  docs/character.md (the five layers and eleven principles of the rebuilt SDK, which still hold except principle
  8's skeleton), docs/skeleton.feature (the game_engine rig: replaced by CH1). A scenario here that another file
  already plans names it and does not repeat it.

  What exists today, so nothing is planned twice: motion matching, a clip database, style vectors, foot locking,
  inertialization, a chooser of situations and cloth that follows skin are built (sdk/character, sdk/body), on the
  52-joint game_engine rig, over captures from ACCAD and Quaternius. Arms and hands are built in Rust
  (systems/body, systems/grab) on the same rig. All of it moves to SOMA's joints; none of it is thrown away.

  Rule: The standard, and who builds to it

    Scenario: CH0. One plan, four parts, one place they meet
      # The parts and the sessions that hold them on 11-15: the body, motion and clothing (this plan's session);
      # heads and hair (the face session: a fitted head model with expressions, from close-up photographs); the
      # creature solver and its test animal (the raccoon session); anymove on that animal (this plan's session,
      # validated by the raccoon session). Agreements between sessions are written here as scenarios, so a
      # session that starts cold reads them.
      Then every part delivers a folder a `<Character>` loads, in the format of CH2
      And a change to a joint name, a unit, an axis or a file's layout is made here first, then built

    Scenario: CH1. People are on SOMA's skeleton
      # NVlabs/SOMA-X, Apache-2.0. 78 named joints (Root, Hips, Spine1-2, Chest, Neck1-2, Head, Jaw, two eyes,
      # shoulders, arms, hands with five fingers, legs, toes) and 32 twist joints the engine sets (four along
      # each upper arm, forearm, thigh and shin). SOMA's files are in centimetres; raea's are in metres, Y up,
      # facing +Z. Replaces docs/skeleton.feature's game_engine rig and docs/character.md's principle 8.
      Then no new work on people names a game_engine joint
      And the readers of the old names (sdk/character, sdk/body, systems/body, systems/grab, their tools and
        tests) are moved to SOMA's names as one refactor, not through an alias layer (as SK0b did before)
      And the people built on the old rig are retired once a SOMA person renders in each scene that used one

    Scenario: CH2. A body folder is the same for a person and a creature
      Given a folder with skeleton.json (name, rig, height, joints by name with parent and rest place, landmarks,
        skin samples per joint), body.glb (skinned to those joints: one skin, or several closed pieces each
        weighted to its own joints, as the raccoon's trunk and legs are), and optional head.glb, clothes and clips
      Then `rig` says which skeleton the joints are: "soma" for people, the creature's own name for an animal
      And helper joints (twist joints, a belly, ears, eye joints) are listed as helpers: the engine or a part's
        own layer sets them, and no pose, clip or call has to name them

  Rule: Motion: one clip format, two ways to get clips, one way to play them

    # The position this plan takes, for the user to overrule: motion at run time is always chosen from clips on
    # disk by motion matching, on the user's own machine, the same frame for a render and for live play. A model
    # that generates motion (Kimodo for people, anymove for creatures) is a way to MAKE clips, run when a project
    # is authored or built, in the cloud or on an NVIDIA card: Kimodo does not run on a Mac. So a game or a film
    # never waits on a model, and a project keeps working with no network. The alternative, a model called
    # while a scene plays, is not planned.

    Scenario: CH10. One clip format, on the body's own skeleton
      # The clip file the SDK already reads is the base (sdk/body/src/clip.ts `ClipFile`: the skeleton's name,
      # the joints' names, and per clip its frame rate, the root's place as 32-bit floats and each joint's turn
      # relative to its parent as 16-bit parts). It gains the rig's name, and per clip the contacts, tags and
      # source below.
      Then a clip is each joint's turn and the root's place per frame, at its own frame rate, with its contacts
        (a list by joint name, each planted or not and where, so four legs or six fit as two do; hands and a
        seat the same way), its situation tags, and where it came from (a capture and its licence; a model, its
        prompt, seed and constraints; a procedure)
      And a capture, a generated motion and a baked procedure are the same file, so the library, the matcher and
        the checks read one thing

    Scenario: CH11. Kimodo is a generation flow that writes clips
      # Kimodo: code Apache-2.0, weights under NVIDIA's Open Model License (commercial use allowed, outputs not
      # claimed). It needs an NVIDIA card and a gated text encoder, so it runs as a cloud step (docs/gen.feature)
      # or on a user's own card. Its inputs, from the examples it ships: one line of text; several lines with
      # durations; full-body keyframes; hand or foot targets; a path for the root; waypoints. Its output is 30
      # joints' turns and places at 30 frames a second. Not measured yet: seconds and cost per clip, and how
      # well it holds a keyframe.
      Given a line of text and any of: a start pose, an end pose, keyframes, a root path, hand or foot targets
      Then `raea gen` returns a clip in CH10's format, with its provenance, checked by CH13 before it is kept
      And the cost of a clip and the time it took are logged with it

    Scenario: CH12. The library: captures first, generated where there is none
      # docs/character.md's layer 2 and its 29 situations stand. Principle 1 stands too: a capture is the
      # ceiling, so a situation that has a good capture keeps it (retargeted to SOMA once, offline).
      Then each situation lists its clips, and a situation with no capture (the ones character.md asks for: turn
        and sit, sit back, rise and walk off, pick from the floor, carry, set down) is filled by Kimodo
      And all people share one library, since they share one skeleton; a body of other proportions plays it at
        the same Froude number

    Scenario: CH13. A generated clip earns its place
      Then a clip enters the library only after the detectors that judge captures have judged it: joint limits,
        self overlap, foot slide against its own contacts, pops, and its style vector beside the situation's
        captures
      And what fails is kept out and counted, so the pass rate of each source is a number

    Scenario: CH14. Locomotion is motion matching over that library
      # Built on the old rig: sdk/body/src/loco (the chooser, the feature database, the matcher, inertialized
      # switches, feet held by their contacts, the head steadied). This scenario is that code on SOMA.
      Then a body goes where a path says, by the situations the chooser allows, with every switch inertialized
        and every planted foot held
      And the turbo suite's cases pass on a SOMA person at least as well as they do today

    Scenario: CH15. Between two poses
      Given where a body is and how it stands, and where it must be and how, with no clip that does it
      Then at build a clip is asked of Kimodo with both poses as keyframes and the path between, checked (CH13),
        and stored in the project, so the next build asks for nothing
      And while a scene plays live and there is no such clip, the matcher's nearest is played and the gap is
        written down as a clip to make, never papered over

    Scenario: CH16. Staging: a performance is a list of steps
      # Built on the old rig: matching.ts `perform(lib, {at, facing, steps})` with walk, face, wait; course.ts;
      # the take (docs/hands.feature). This is the one vocabulary for all of them.
      Then a step is one of: go (a route, a gait, a style), face, wait, take, put, throw, hand over, sit, rise,
        use (a thing's own path), look at, and do (a line of text: a gesture or an act with no step of its own)
      And `do` is made into a clip by CH11 when the project is built, and from then on is a clip like any other
      And the whole performance is a function of its steps: any frame can be asked for, and a render and live
        play agree

    Scenario: CH17. NPC controllers speak in the same steps
      Then a controller is a function from what a character knows (where things and others are, what it holds,
        what it wants) to its next steps, asked a few times a second, never per frame
      And a scripted controller, a behaviour tree and a language model all return CH16's steps, so the motion
        under them is the same and a controller can be swapped without touching motion
      And a controller may ask for a step the library cannot play; the character does the nearest it can, and the
        ask is logged for CH15

    Scenario: CH18. What a clip cannot know is added on top, and measured
      # docs/hands.feature (reach, grab, carry, throw, look), feet and seat contacts (character.md layer 4).
      Then hands, eyes and contacts are laid over the matched pose by IK, each reporting how far it moved the pose
      And a large correction is reported as a missing clip (principle 2), which CH15 can then make

  Rule: Heads

    Scenario: CH20. The generated head joins the body at the collar
      # Agreed between the body and face sessions 11-15, reported to the user by the face session. The face
      # side fits a head model (Apache-2.0) to close-up photographs: identity, expressions, eyes, skin texture.
      Given head.glb skinned to SOMA's Neck1, Neck2 and Head, with SOMA's eye joints, and its expressions as
        about fifty named morph targets, and head.json (its collar ring, eye centres, morph names, neck colour,
        and the size it was fitted at, so the body's build scales the head to the body)
      Then the body's own head is cut away from the collar up when the body is built, and the neck is one mesh
      And the body's motion drives the neck and head; the face layer adds expression and the eyes on top; the
        eyes take a look target in the world and cover what the head has not turned
      And the face's skin colour is carried onto the body's skin at the seam
      And `<Character>` passes the face's weights to the head's morph targets by name (its `morphs` prop does this
        for a body's own morphs today) and takes a look target

    Scenario: CH21. Hair
      # Agreed with the face session 11-15. sdk/body/src/cloth.ts already moves hair and clothes that follow
      # skin. Hair cards are the face session's working choice, pending its tests; strand hair stays a reference
      # for offline renders. Open: whether spring chains under Head (helper joints) drive the cards instead.
      Then hair is a card mesh delivered with the head, attached to SOMA's Head, each vertex carrying how far it
        may stray (0 at the scalp), moved by the same follower as clothes
      And hats and glasses are rigid pieces on Head, delivered with the head

  Rule: Creatures

    # The creature flow is the raccoon session's, end to end, in docs/creatures.feature (its sandbox is the
    # trashpanda project; done when it works on the raccoon and a second, more complex creature). These three
    # scenarios say only where it meets the rest. Kimodo is no part of it: its models are for human skeletons
    # only (SOMA, a humanoid robot, SMPL-X).

    Scenario: CH30. A creature is its own skeleton, set down first
      # A creature's joints are placed by measure before its model is made, and its folder is CH2's with its
      # own rig name. Its model may be built, or generated the way the raccoon's now is: a grey mannequin of
      # the skeleton, an image model drawing the animal on it with the pose and view kept, that image made 3D,
      # and the mesh laid back on the skeleton and weighted there.
      Then a creature's rig is read from its joints (CR1), and its helper joints are named as helpers

    Scenario: CH31. A creature moves by its solver, and by clips where it has them
      # The solver (gait by speed, planted feet, one rigging for every move: CR2 to CR9) needs no clips. Clips
      # for a creature come from anymove (patch-gg/anymove, Apache-2.0): a generated two-camera video of the
      # rigged creature, read by a joint reader trained on that creature, fitted to its rig. Like Kimodo it is a
      # cloud step that writes clips, and it films a creature on a flat floor only.
      Then a creature's clip is CH10's format on its own joints, checked as CH13 checks a person's
      And where a creature has clips for a situation the matcher plays them, and the solver holds its feet; where
        it has none, the solver makes the move

    Scenario: CH32. anymove tried on the raccoon
      # 11-15, run end to end; not yet a pass. The raccoon's skin (no fur shells) and 36 bones on its own joints
      # were set up as an anymove creature in Blender on a Mac (its renderer made to use Metal there); the pose
      # dataset (2,200) and the joint reader were made on a cloud card in 35 minutes (held-out error 6.9 px
      # front, 4.8 side, anymove's wolf being 6.9 and 3.8); two five-second walk clips from the video model
      # both kept the split screen and the limb colours; each was read and fitted in 5.5 minutes and exported
      # as a GLB. By anymove's own measure, how well the paws' heights agree between the two views, the clips
      # score 0.23 and 0.56: under 0.5 is a reroll, 0.8 is a keep, so none of 2 passes and one is worth a look
      # (anymove reports about 7 clips for a usable move). Cost so far about 2.10 dollars: 0.52 for the clips,
      # the rest cloud time estimated from minutes. Measured against the raccoon traced from real footage (the
      # raccoon session's check, nine measures): the first two clips were smooth with good foot timing but
      # were a generic four-legged walk, the head and tail level with the back and the legs in diagonal pairs.
      # Two more clips, the prompt naming a low head, an arched back, a hanging tail and a pacing walk (0.77
      # dollars more with their captures): the carriage came into a raccoon's range (top of the head 0.74 of
      # the back's height, nose 0.32, withers 0.82) and the better clip has 7 of 9 measures in range; still
      # off in all four are the order of the legs (a fore foot 0.36 of a stride after its side's hind, a
      # raccoon's being 0.14) and the tail's tip (0.27 against 0.18). The views' agreement stayed under the
      # keep line (0.42 and 0.53), the far hind paw the weak one in all four clips. So far: a prompt sets
      # carriage, not the order of the legs; clips give timing and smoothness; CH31's split (clips for motion,
      # the solver for contact and carriage) stands. To come: the generated raccoon's skin and refitted joints
      # in place of the built one, and why the fore paws carry no agreement score.
      Then the raccoon has a walk from anymove, and the numbers say whether it beats the solver's walk

  Rule: Clothes, after the motion system is settled

    Scenario: CH40. A way to make clothes for any SOMA body
      # The user's direction, 11-15: generative or procedural; perhaps the mannequin as the base input, an
      # image model on top of it, and layers from there. Not designed yet: this is the next job once CH10 to
      # CH17 are settled. What exists: cloth that follows skin with no collision shapes (sdk/body/src/cloth.ts),
      # and SOMA's body has three UV sets and a fixed topology, so anything made for it fits every SOMA person.
      Then a garment is made for the standard body once and fits every person built from it
