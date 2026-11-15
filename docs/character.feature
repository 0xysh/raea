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

  The system on one page, as it stands 11-15 (each part names the scenarios that hold its detail):

  Body. Every person is SOMA (NVlabs SOMA-X, Apache-2.0): one skeleton of 78 joints and 32 twist helpers the
  engine sets, one body folder for a person or a creature (skeleton.json, body.glb, parts; CH2), builds from
  SOMA's shape model. The bodies on the old game_engine rig only play until they are retired (CH1, CH1a).

  Motion. At run time a body only plays clips, chosen by motion matching from a library on SOMA's skeleton, on the
  user's own machine, so a render and live play give the same frames (CH10, CH14a). Clips come from captures first
  (ACCAD and Quaternius, moved onto SOMA once) and from Kimodo where no capture exists, made when a project is
  built, in the cloud: between two poses with no text today (CH15a: "turn and sit", "rise and walk off"), from a
  line of text once its encoder's licence is granted (CH11). A generated clip is judged by the checks that judge
  captures before it is kept (CH13). Locomotion is that matcher following a path, its stops landing on their marks.

  Staging and NPCs. A performance is a list of acts: go, wait, look, face, sit, take (and put, throw, hand over and
  use, which are takes with another spec) and do (a line of text). A script, a behaviour tree and a language model
  all return those acts to `direct`, which asks a character's controller only when its acts run out, and the
  matcher carries them out (CH16a). An act no clip plays is stood through and written down as a clip to make, and
  the build turns that list into Kimodo clips (CH17a, CH15). Hands, eyes and contacts are laid over the matched
  pose by the Rust takes and the look solver, each saying how far it moved the pose (CH18a).

  Heads. The face session's head (GNM Head, Apache-2.0) is a Character part skinned to SOMA's neck, head and eye
  joints with about fifty named expressions, ending on the collar ring of a body built without its head; hair is
  cards skinned to Head with a stray distance per vertex; hats and glasses are rigid parts on Head (CH20a, CH21a).

  Creatures. The raccoon session's flow (docs/creatures.feature): a creature is its own skeleton set down by measure
  first, its model generated on a grey mannequin of that skeleton and weighted there, moved by a gait solver that
  owns contact, carriage and the order of the legs, with anymove's generated clips for timing where it has them
  (CH30 to CH32). The same body folder and the same clip format as a person; Kimodo is for people only.

  Clothes. Pictures of the dressed mannequin measured onto SOMA's skin (CH40 to CH46) were built and rejected by
  the user on the body. Garments are now sewing patterns cut to SOMA's measurements and draped by a cloth of our
  own on the GPU (CH47a, CH48a), with a model trained on that data proposed (CH48). CH47 and CH48 are the user's
  to decide.

  Who holds what: this plan's session the body, motion, staging and clothes, and the syncing; the face session
  heads and hair; the raccoon session creatures. A change to a joint name, a unit, an axis or a file's layout is
  made here first (CH0).

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

    @done
    Scenario: CH1a. The old SDK names SOMA's joints
      # 11-15. sdk/character's bones.ts `B` (the one place the SDK takes joint names from) gives SOMA's names:
      # Hips, Spine1, Spine2, Chest, Neck1, Head, LeftShoulder, LeftArm, LeftForeArm, LeftHand, LeftLeg,
      # LeftShin, LeftFoot, LeftToeBase, and a finger's phalanges 1 to 3 as SOMA's 2 to 4 (its 1 is the bone in
      # the palm; the thumb's are 1 to 3). A body made on the old rig (turbo, teal, the PSX people) still loads:
      # `joint` finds a SOMA name on it by its old one, the name helpers (side, mirror, finger) read both, and
      # a BVH map keyed by old names still maps. That fallback is the only place the old names stay, and it goes
      # with the last old body (CH1's last line). systems/body's `Skeleton::joint` reads a SOMA name on an old
      # body the same way. A body named as CMU or Mixamo name theirs (LeftUpLeg, with LeftLeg the shin, which is
      # SOMA's thigh) is refused with a message rather than read wrong; the gitignored local "kid" example is
      # one. The conformance tests' stick body is on SOMA's names; 27 of the character test's 29 pass, the 2
      # that fail failing the same way on the code before the change (a buckle on a planted knee, a dropped box).
      Then the SDK names SOMA's joints, and the bodies on the old rig still play until they are retired

    Scenario: CH2. A body folder is the same for a person and a creature
      Given a folder with skeleton.json (name, rig, height, joints by name with parent and rest place, landmarks,
        skin samples per joint), body.glb (skinned to those joints: one skin, or several closed pieces each
        weighted to its own joints, as the raccoon's trunk and legs are), and optional head.glb, clothes and clips
      Then `rig` says which skeleton the joints are: "soma" for people, the creature's own name for an animal
      And helper joints (twist joints, a belly, ears, eye joints) are listed as helpers: the engine or a part's
        own layer sets them, and no pose, clip or call has to name them

    @done
    Scenario: CH2a. Bodies, heads and creatures are read as one form and checked
      # 11-15. sdk/body/src/folder.ts (private repo 47ddc16) reads a folder's skeleton.json, or a creature's
      # <name>.rig.json (docs/creatures.feature CR1's form) as the same skeleton: `rig` the creature's name, each
      # bone's tail its `ends` entry, the height its highest joint or mark, the eyes between its eye marks, and
      # skin samples taken from its skinned mesh (samples.ts) since a rig carries none. Helpers not listed are
      # found by name (SOMA's 32 twist joints, its eyes and jaw; a creature's ears, lids, belly and jaw).
      # `bun sdk/body/src/folder.ts BODY [HEAD]` lists a folder's problems against CH2, CH20 and CH21. Run on
      # the work as it stands: SOMA's headless body with the generated head, its four hair styles, glasses, cap
      # and beanie has none (the head's collar ring lies exactly on the body's 80 ring vertices; hair and hats
      # ride Head alone; each hair mesh carries a stray distance per vertex). The raccoon has none: its rig reads
      # as the same skeleton, its 58 joints all skinned, 1,392 skin samples with the lowest on the ground.
      # Left to the owners, as notes rather than faults: the head carries 20 expressions where CH20 asks about
      # fifty; head.json names its body by an absolute path on one machine; the raccoon's body is raccoon.glb,
      # not body.glb; SOMA's skeleton.json doesn't list its helpers (found by name).
      Then a person's folder and a creature's are read by one reader, and a folder that breaks the form says how

  Rule: Motion: one clip format, two ways to get clips, one way to play them

    # The position this plan takes, confirmed by the user 11-15 ("clips at run time"): motion at run time is always chosen from clips on
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
      # joints' turns and places at 30 frames a second (which Kimodo widens to SOMA's 77 by name).
      # Measured 11-15, with no text (constraints only; an empty prompt zeroes the text features, so a stand-in
      # encoder of zeros is exact), Kimodo-SOMA-RP-v1 on its own SOMA examples, 100 denoising steps, on a Modal
      # L40S or A10G: 4.3 s of GPU a 5 s clip, 7.2 s for 10 s, the model loading in 40 s; about 12 cents for the
      # whole test with the image build. Misses at the constrained frames, raw and after Kimodo's own
      # post-processing: full-body keyframes 3.2 cm mean (8.2 worst) and 0.5 (6.0); feet 1.0-1.5 and 0; hands
      # 2.8-3.3 and 2.4-3.0 (6 worst); a root path 12 cm mean (26 worst) raw and 3.1 (4.0, its margin) after;
      # foot slide on planted frames 0.06-0.26 cm a frame. So post-processing is always on. Text is not
      # measured: the encoder's base (meta-llama/Meta-Llama-3-8B-Instruct) is gated behind a form that asks for
      # the account holder's name, birth date, country and affiliation, which only Shane can fill in.
      # Runner: ~/.cache/raea-kimodo/ours/kimodo_constraints.py (and kimodo_modal.py for text).
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

    @done
    Scenario: CH14a. The rebuilt body SDK, its library and its matcher run on SOMA
      # 11-15. sdk/body (the rebuilt SDK of docs/character.md) names SOMA's joints: its roles keep the keys of
      # its tables, its limits are re-measured over the library on SOMA (8,848 frames), and the elbow's fold
      # axis is read from the captures, SOMA's rest elbow being nearly straight. The library's 60 clips (ACCAD,
      # Quaternius) were moved from the old rig joint for joint (sdk/body/tools/resoma.ts: a joint's world turn
      # after the turn that lays SOMA's rest bone on the source's, hands by their knuckles, the wrist's turn
      # about the forearm given to the forearm, contacts carried). On it the matcher follows a path (stand,
      # walk, turn, stand, walk back, stop): 449 frames over 10 clips with 20 switches, planted feet sliding
      # 17 mm at worst (8 mm on the old rig's own suite), no joint past a limit, drawn on SOMA's body with the
      # twist joints set (raea/character `withTwists`). 23 of the SDK's 27 tests pass; the four that don't are
      # the hand layer (the cup grasp, hands in the lap, finger IK), which needs SOMA's fingers and a shell.
      # Not moved yet: the old SDK (sdk/character: its matcher, items, ragdoll), the arms and hands in Rust
      # (systems/grab's takes name the old joints), and the turbo suite's cases as checks.
      Then a body on SOMA walks where a path says from the library's captures

    Scenario: CH14. Locomotion is motion matching over that library
      # Built on the old rig: sdk/body/src/loco (the chooser, the feature database, the matcher, inertialized
      # switches, feet held by their contacts, the head steadied). This scenario is that code on SOMA.
      Then a body goes where a path says, by the situations the chooser allows, with every switch inertialized
        and every planted foot held
      And the turbo suite's cases pass on a SOMA person at least as well as they do today

    @done
    Scenario: CH15a. Two situations the captures lack, made by Kimodo from two of their poses
      # 11-15. sdk/body/tools/between.ts writes Kimodo's full-body keyframes from two library frames (the first at
      # the origin facing +z, the second where and how the body must end); src/kimodo.ts crosses a pose both ways
      # by its turns (a joint's world turn from bind is Kimodo's turn from the T-pose after the body's `tpose`;
      # a library pose goes out and back with no measurable change). Kimodo runs with no text (the stand-in
      # encoder, CH11). tools/fromkimodo.ts brings each motion onto the body, puts it back on the captures'
      # elbow and knee hinges with the hands and feet where Kimodo put them (src/fix.ts `ontoRig`: Kimodo's elbows
      # fold about an axis 25 degrees off the captures', which hold to within 3 to 7), and judges it against the
      # captures it joins, measured on the same body by the same checks: no joint further past a limit and the
      # body no deeper into itself. Of several seeds the best that passes is kept, with where it came from.
      # "turn and sit" (stand, then seated on a chair 1.2 m ahead, turned round): 4 of 5 passed.
      # "rise and walk off" (from the seat to a walking stride 1.6 m on): 2 of 5 passed, the others' hands
      # 5 to 9 cm into the thigh as they push up (the walking captures it joins: 4.4 cm). 4 s of GPU a clip,
      # about a cent. Both are in the library under src/library.ts's situations of those names, and play one
      # after the other on SOMA's body (sheet3d compositions/soma_between.tsx, feed item i-acp-mwi1xhlw-59).
      # Found on the way: contacts and capsules on a SOMA body were never read (the SOMA index was carried over
      # from the old rig); a SOMA limb's skin is weighted to its twist joints, now counted as the limb's.
      Then a situation without a capture gets a clip made from the captures' own poses, judged before it is kept

    Scenario: CH15. Between two poses
      Given where a body is and how it stands, and where it must be and how, with no clip that does it
      Then at build a clip is asked of Kimodo with both poses as keyframes and the path between, checked (CH13),
        and stored in the project, so the next build asks for nothing
      And while a scene plays live and there is no such clip, the matcher's nearest is played and the gap is
        written down as a clip to make, never papered over

    Scenario: CH15b. A line of text made into a clip between two standing poses
      # 11-15, the first run of Kimodo's text path (the account's access to the gated Llama 3 model was granted that
      # morning). Two standing poses from the captures as keyframes, in place, 4 s, and the line "A person waves at
      # the camera with their right hand", three seeds: in each the right wrist is above its shoulder for 48 to 59 of
      # 120 frames, reaching the head's height, while the left never leaves the hip, and the hips stay within 3 cm.
      # All three pass the bar of the 19 captures that end standing (sdk/body/tools/fromkimodo.ts with a "do:"
      # name); the kept one is written to the project's made clips (made.json, read by stage.ts `madeClips`). On
      # Modal: 1.6 s a clip on the GPU after 130 s loading the model, and 26 minutes queued behind the workspace's
      # other GPU jobs. Kimodo's code has nothing tied to NVIDIA (no custom kernels; its text encoder takes Apple's
      # `mps` device), so the earlier line that it does not run on a Mac is untested, not true.
      Then a stage `do` line gets a clip made from its words, checked, and kept in the project

    Scenario: CH15c. Kimodo on the user's own machine, in MLX (planned)
      # The user, 11-15: "lets plan that if kimodo works well we port it mlx weights or go find some". Gated on
      # an eval first: about ten lines (wave, point, shrug, clap, nod, bow, look at a watch, cross the arms,
      # scratch the head, stretch), each with a check of its own (which hand rises, where the head turns) and a look
      # by the user. If they hold, Kimodo moves to MLX, the general backend on Apple silicon already in raea
      # (raea-sys-mlx), in two halves. The text encoder (LLM2Vec: Llama 3 8B made bidirectional, with its
      # trained adapters): MLX copies of Llama 3 8B Instruct exist (mlx-community, 4-bit about 4.5 GB), so the
      # work is LLM2Vec's bidirectional mask and adapters merged onto them, checked against PyTorch's features
      # for the same lines; a line's features are kept once made, so the encoder runs once a line. The motion
      # model (a small diffusion transformer): its weights converted and run in MLX, checked against the PyTorch
      # motion for the same line, constraints, seed and noise. Before anything ships to users: the licences of
      # converted weights (NVIDIA's Open Model License for Kimodo, Meta's Llama 3 licence for the encoder).
      Given the eval's lines pass
      Then a do line's clip is made on a Mac with no GPU in the cloud, matching the cloud's within a stated
        tolerance
      And its cost is a one-off download and seconds a line

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

    @done
    Scenario: CH17a. An act no clip plays is done as near as can be and written down as a clip to make
      # 11-15. sdk/body's stage gained `do` (a line of text). With a clip made for the line (`direct`'s `made`,
      # keyed by the line in lower case with single spaces) it plays as the sitting captures do, carried on from
      # where the body stands; without one the character stands for the act's seconds and the line goes on its
      # `asks`; `toMake` gathers a cast's asks, once each, most asked first: the build step's list for Kimodo
      # (CH11, CH15). A language model's act outside the vocabulary ({"wave": "ben"}, a bare "shrug") becomes a do
      # line instead of being dropped, so what a controller wanted and could not get is counted; a known act with
      # wrong values ({"wait": -1}) is still dropped. Tested with two characters asking for one line spelled two
      # ways (listed once, asked twice) and with the library's sitting down standing in for a made clip (played,
      # no frame jump past 6 cm); the body SDK's suite is 37 of 39, the 2 failing being the hand layer's cup grasp
      # as before (private repo ee61dd1). The made clips themselves wait on Kimodo's text encoder: the user has
      # sent Meta's licence form; the account still reads 403 while it is reviewed.
      Then a controller can ask for anything, the character does the nearest it can, and the ask becomes a clip to
        make

    @done
    Scenario: CH16a. Acts, and controllers that return them, carried out by the matcher on SOMA
      # 11-15. sdk/body's stage module: an act is go (to a point or through several, at a gait), wait, or look
      # (at a point, a named character, or nothing). `direct` plays a cast for some seconds: each character's
      # controller is asked when its acts run out (and at the start), told where it and every other character
      # is and the time; go and wait become the matcher's steps, look is laid on the pose by the look solver.
      # Controllers so far: `script` (a fixed list), `keepNear` (stay within a reach of another character,
      # looking at it), and `actsFrom`, which reads acts from outside data (a language model's JSON) and keeps
      # only those in the vocabulary. Shown with two SOMA bodies: one follows a script, the other's controller
      # was asked 16 times in 26 seconds and kept him near her. The same cast and seconds give the same frames.
      # Added the same day: face (a step toward a point or a character) and sit (the library's captures of
      # sitting down, sitting and rising, played from where the body stands and eased at each join; the scene
      # is told where to put the seat). Found while adding sit: the matcher set the body back inside its
      # guide's reach at once, and a stop that overshot on SOMA moved the hips 19 cm in one frame; the return
      # is now limited a frame, and the worst hip step in the scene is a walking pace's (4.8 cm). 31 of the
      # body SDK's 35 tests pass, the four left being the hand layer.
      # Then take: the act reserves a stretch of standing and says which frame it starts at; the scene applies
      # the engine's take (raea/character `take`: reach, grab, carry, put down or throw, a hand-over, a thing's
      # own path) over the staged pose, so put, throw, hand over and use are the same act with another spec.
      # Shown: a controller walks a character to a table and, once she stands there, has her move a cup along
      # it; she walks off after. Faults seen: she stops 25 cm past her mark, into the table's edge (the stop's
      # overshoot again); and the other character stood with raised arms, which was two turning captures
      # whose actor holds his hands up throughout: dropped from locomotion.
      # The stop's overshoot was then measured and dealt with in the matcher: over six paths a walk came to
      # rest 20 to 37 cm past its mark along its way (26 on average), whatever the path. Nothing may drag a
      # standing body back, so the walk is played again (up to four times) with each stop's mark set back by
      # what the last playing overran, and the playing whose worst stop is nearest its mark is kept: 6 cm on
      # average, 14 at worst, the planted feet's slide unchanged (7 mm). She now stands at her mark by the
      # table. The cost: a staged character is performed several times over, seconds for a scene.
      # Not in the vocabulary yet: do (a line of text made into a clip, which waits on CH11). A controller's unmet ask is not logged
      # yet (CH15).
      Then a script and a controller give acts in one vocabulary and the matcher carries them out

    @done
    Scenario: CH18a. The takes run on a SOMA body
      # 11-15. The Rust arms and hands (docs/hands.feature) find a joint by its role on either rig, so reach,
      # grab, carry and throw run on SOMA with no second copy. The audit's six takes on SOMA's skin: holds
      # within 2 to 8 mm and no body skin in a carried thing, as on the old rig; the fingers are not tuned to
      # SOMA's hand (hand skin 22 mm into a box, no pads on a cup, 41 to 47 mm into the table as a hand leaves).
      # The joint limits have a SOMA table (the body SDK's, measured over the library on SOMA). The fingers'
      # rest curl is close to the old rig's (the thumb's first bone lies 31 degrees off the hand against 69),
      # so the misses were not a matter of rest pose. Found: the finger curl placed a knuckle from its parent
      # joint, which on the old rig is the wrist and on SOMA is a bone in the palm, so each finger was worked
      # out 8 cm from where it is. Placed from the hand's joint: finger pads on a cup go from none to 6, hand
      # skin in a box from 22 to 17 mm (the old rig's own numbers unchanged). Six takes are drawn on SOMA's
      # body (a box thrown, a cup moved, a mug drunk from, a cup passed between hands, a box at 14 kg). Left:
      # the middle finger and thumb tip 15 to 17 mm into a box, and hands 41 to 47 mm into the table as they
      # leave.
      Then a take runs on SOMA, its fingers still to be fitted to SOMA's hand

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

    @done
    Scenario: CH20a. The first generated head rides SOMA's body
      # 11-15. The body is built without its head (cut level through mid-neck at 1.515 m; its open edge written
      # as a ring of its own 80 vertices with their normals and weights); the face session's head ends on that
      # ring with those weights, so the neck is closed with no weld at run time. A Character part may name the
      # body's joints it has (`joints`), since the engine refuses a part that lacks a joint it is handed. The
      # head rides the matcher's walk, its expressions driven by name, glasses on as a one-joint part. Left: the
      # body is one flat colour against the head's baked skin (a step shows at the seam), the eyes' look target,
      # and the head's ears, which the face session is rebuilding.
      Then a generated head is drawn on SOMA's body as a part, joined at the collar

    Scenario: CH21. Hair
      # Agreed with the face session 11-15. sdk/body/src/cloth.ts already moves hair and clothes that follow
      # skin. Hair cards are the face session's working choice, pending its tests; strand hair stays a reference
      # for offline renders. Open: whether spring chains under Head (helper joints) drive the cards instead.
      Then hair is a card mesh delivered with the head, attached to SOMA's Head, each vertex carrying how far it
        may stray (0 at the scalp), moved by the same follower as clothes
      And hats and glasses are rigid pieces on Head, delivered with the head

    @done
    Scenario: CH21a. Hair cards ride the head, and a whole person stands on the standard
      # 11-15. The face session's hair (cards grown from its strand grooms, one mesh skinned to Head, a stray
      # distance per vertex in a custom attribute, four styles) is drawn as a Character part beside the head
      # and the glasses. With it a whole person is on the standard for the first time: SOMA's body, the
      # generated head joined at the neck, hair and glasses, generated clothes in two layers, walking by motion
      # matching. Not yet: the hair moving (the follower does not read the stray attribute from a part), and
      # raea's own hair shading on it (that shader wants tangents the cards do not carry, so it is drawn with
      # its own material).
      Then a person is a body folder and parts, each made by its own flow

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
      # the solver for contact and carriage) stands. The solver's own order was wrong too, found the same day: it
      # read each leg's offset with the wrong sign, so a fore foot came down 0.26 of a stride BEFORE its hind.
      # Fixed in the creature solver (private sdk/body, d89244e, with a footfall test that fails on the old sign),
      # a fore foot lands 0.10 after its hind and the hinds 0.50 apart, as a raccoon's do; the same change keeps
      # every planted foot inside its leg's reach (setting off after standing had left one 59 mm short on the
      # raccoon, 81 on a dragon). So on the order of the legs the solver now beats anymove's clips. To come: the generated raccoon's skin and refitted joints
      # in place of the built one, and why the fore paws carry no agreement score.
      Then the raccoon has a walk from anymove, and the numbers say whether it beats the solver's walk

  Rule: Clothes: first drawn on the mannequin by an image model (rejected), now sewing patterns (proposed)

    # Superseded the same day: the user saw these garments on the body and rejected them ("on the body it looks
    # terrible"; the views "are not consistent at all"). CH47a and CH48a below are the direction now; CH40-CH46
    # stay as the record of what the picture method could and could not do.
    # The user's direction, 11-15: generative or procedural; perhaps the mannequin as the base input, an image
    # model on top of it, and layers from there. Tried that day on SOMA's body, and it works; what follows is
    # the standard those tests point to. The idea in one line: a garment is not a mesh of its own, it is data
    # on the body's own vertices (which are covered, how far the cloth stands off, its colour, how freely it
    # may swing), so it carries the body's skinning, twist joints included, and fits every SOMA person without
    # being refitted; only cloth that hangs free of the body (a skirt, a coat's tails, a hood) is its own mesh.

    @done
    Scenario: CH40a. A picture of the dressed mannequin lies on the mannequin pixel for pixel, one view at a time
      # 11-15, with an image model that edits a given picture (flux_2_pro through `raea gen image --image`, 5
      # cents a picture). The mannequin: SOMA's body at rest, flat dark grey on mid grey, no perspective, 2 m
      # of height in a square frame, from the front, the back and each side (tools/soma_mannequin.py in the
      # test project writes the pictures and their cameras). Measured by laying each result over its mannequin:
      # one view, asked to change only the surface where clothing covers it: every limb, the head, the hands
      # and the feet stay where they were, front, back and both sides alike, and the clothes sit outside the
      # body's outline. Four views on one sheet in one call: the model lays the sheet out again (two fronts,
      # one side), so nothing can be laid back. A view given a second picture as the outfit to copy: it draws
      # a front again, smaller. A layer added to a dressed view ("add exactly one garment and keep everything
      # else": a jacket over the t-shirt): pose and place hold and the rest is kept. Between views drawn apart
      # the cut and the kind of garment agree from the words alone; the colour drifts (the jeans' wash).
      Then each view of a garment is drawn on its own mannequin, a layer at a time, and views agree by their words

    @done
    Scenario: CH40b. Those pictures become an outfit on SOMA's own vertices, worn through a walk
      # 11-15, first cut (tools/soma_clothes.py in the test project). Each vertex of the body's skin is looked
      # up in the views that face it and see it (a ray to the camera meets nothing of the body): where the
      # picture shows cloth and not the mannequin's grey it is clothed and takes the cloth's colour; the mask
      # is tidied on the mesh (a bare vertex ringed by cloth is cloth, a small island is not); the clothed
      # skin, pushed 4 mm out, is the outfit, with the body's own weights. A t-shirt, jeans and shoes made
      # this way are worn by SOMA's body through the matcher's walk as a Character part (6,519 of 19,830
      # vertices). What the first cut lacks: the cloth is skin-tight (no silhouette is measured yet), the tops
      # of the shoulders and the soles are seen by no view, the colour is per vertex (soft), hems are jagged
      # along the body's own edges, toes show through shoes, and the views' colours are not matched.
      Then a drawn outfit is worn by a SOMA body in motion with no rigging step

    @done
    Scenario: CH41a. The cloth's stand-off read at the outlines
      # 11-15. Where a view sees a clothed vertex on the body's outline (its normal across the view), the
      # picture is walked outward along the normal until the cloth ends and the backdrop begins: that distance
      # is how far the cloth stands off there. Read at 3,585 outline vertices of the t-shirt, jeans and shoes
      # (2 mm at the tenth of them, 4 at the middle, 16 at three quarters, 35 at nine tenths), spread over each
      # garment through the mesh, never under 4 mm nor over 8 cm. The jeans now hang as trousers and the shirt
      # stands off the trunk. Wrong still: the shoes are lumpy (the toes' own shape reads as outline), the tops
      # of the shoulders are bare, and a reading is one distance per vertex, so a fold is not a fold.
      Then a garment's looseness comes from its picture

    Scenario: CH41. A garment's shape is measured, not assumed
      Given each layer's views and the views of the layer under it
      Then how far the cloth stands off the skin is read where the two outlines differ in each view (a sleeve's
        width against the arm's, a trouser leg's against the leg's), and spread round each limb between the
        views that see it, so a loose fit is loose
      And a view from above and one from below are drawn too, so shoulders and soles are covered
      And a garment's edge (a hem, a cuff, a collar) is cut where the picture puts it, not along the body's faces

    @done
    Scenario: CH42a. A garment file on SOMA's template vertices, worn with the skin under it left out
      # 11-15 (tools/soma_clothes.py in the test project: `measure` and `wear`). A garment is a JSON file: its
      # name and layer, the template vertices it covers (by SOMA's own numbering of its 18,056, found from
      # the body's vertices by their rest places), each one's stand-off, colour and how far it may stray, and
      # the pictures it came from. `wear` builds each garment's mesh from the file on a body (the covered skin
      # pushed out by the stand-off, the body's own weights) and writes the body with the skin that is wholly
      # under cloth left out (13,368 of 26,116 faces under a t-shirt, jeans, shoes and a jacket). Not yet: a
      # texture in the body's UVs (colour is per vertex), and a second SOMA shape to prove the fit carries.
      Then a garment is a file of numbers on SOMA's vertices, and a dressed body draws no skin under its clothes

    @done
    Scenario: CH43a. A second layer over the first
      # 11-15. A bomber jacket drawn over the t-shirt view by view ("add exactly one garment and keep
      # everything else"). It is the vertices where its picture differs from the picture of the layer under
      # it, and it stands off at least that layer's distance plus 3 mm (measured: 20 mm at the tenth of its
      # vertices, 42 at the middle, 73 at nine tenths), so the layers do not cross. Learnt: an added layer on
      # the back view came out as the garment's front (a zip and an open front) until the words said what is
      # seen from behind; and a dark garment over the dark mannequin needs a small difference to count.
      Then a jacket is worn over a shirt, each its own garment file

    @done
    Scenario: CH42b. The same garment files on a body of another build
      # 11-15. The body's build takes SOMA's own shape model (SOMA_neutral.npz: a component by so many of its
      # standard deviations); a second body moved along three components (up to 11 cm at a vertex, slimmer and
      # longer in the neck) wears the t-shirt, jeans, shoes and jacket measured on the template body, from the
      # same files, with nothing refitted, through the same matched walk beside the first. A shaped body's
      # vertices are matched to the template's numbers through its template-shaped twin (same export, same
      # order). Not done: the joints stay where the template has them (SOMA's own layer moves them with the
      # shape), so only small changes of build are right; and this is HG17's start, not its end.
      Then a garment made once dresses SOMA bodies of different builds

    @done
    Scenario: CH42c. A garment's colour is a picture in the body's own UVs
      # 11-15. `wear` bakes each garment's colour as a 2048 px picture in the body's first UV set (spread to
      # fill it): every texel of a covered triangle is a point of the body, looked up in the view that faces
      # its triangle most among the views that see all three corners. The jeans now show their fly, pockets
      # and wash and the jacket its zip and ribbing, where a colour per vertex gave a blur. Also: a small bare
      # patch walled in by cloth is counted as cloth, which closed the t-shirt's shoulder tops (the jacket's
      # still gape: their gap runs into the collar). Seen now that the detail is there: the views' colours
      # differ (the back of the jeans is paler than the front), so a seam shows where one view hands over to
      # the next; matching them garment by garment is not done.
      # Looked into after: the skin still seen at the shoulders is not a hole in the jacket. Both garment files
      # cover the shoulder tops; what shows is the slope between neck and shoulder, which the t-shirt's
      # neckline leaves bare (read too wide: that skin faces up, and the level views see it at a glance).
      # An upward-facing vertex among mostly covered skin is now counted as covered, which did not reach it.
      # Tried and taken back: matching the views' colours by the front's main colours (pale denim on the back
      # was taken for the shirt's white), and counting any changed pixel of a layer as its cloth (the jacket
      # then covered the hands).
      Then a garment carries its own picture, and its detail is the drawing's

    Scenario: CH42. A garment is kept as data on SOMA's topology
      Then a garment file holds, per body vertex it covers: that it is covered, its stand-off, and how far it
        may stray (for the cloth follower); and a texture in the body's own UVs, baked from the views with the
        views' colours matched to the front's garment by garment
      And the same file dresses every SOMA person (the shape model moves the vertices, the garment rides them)
      And a body's skin under a garment is not drawn, so nothing pokes through

    Scenario: CH43. Layers
      # The raccoon session's finding holds here too: an image edit keeps the outline it is given, so a layer
      # can be added and never taken away. Outfits are built up from the bare body.
      Then an outfit is an ordered list of garments, each drawn over the ones under it, each one's stand-off at
        least the one under it plus its own thickness, so layers never cross
      And a garment drawn alone on the bare mannequin can be worn under or over others by re-measuring its
        stand-off against what is under it

    @done
    Scenario: CH44a. A skirt hangs from the waist and moves through a walk
      # 11-15, first cut (sdk/body/tools/skirt.ts). The skirt is made for the body by measure: a tube of 56 by
      # 22 particles from an ellipse round the waist's skin samples down to a hem, wider as it falls, its top
      # two rings pinned to the hips and each ring below free to stray further. The cloth follower already
      # built carries it through the matched walk with the legs as capsules sized from the body's own skin
      # samples, and it is drawn as a mesh sequence over the walking body. Measured: edges stretched 19 per
      # cent at the 99th percentile. Two faults mended the same day: the seat came through the back (each ring
      # is now at least as wide as the body at its height down to the crotch, and the pelvis has a collider),
      # and it rode up and shortened by the end of the walk (less room to stray: 27 cm at most, was 39). Wrong
      # still: the thighs show through the front at the longest stride (sizing the rings to the thighs and
      # more room to stray were tried and taken back: the cloth bunched), and it sits on the hips, not the waist. Not yet: grown from a garment's edge or drawn from a picture's
      # outline (it is a plain tube), a hood or a coat's tails, and its colour from a garment file.
      Then cloth hangs free of the body and moves with it

    Scenario: CH44. Cloth that hangs free
      Then a skirt, a coat's tails, a hood, a scarf are meshes of their own, grown from the garment's edge on the
        body down to the outline the views show, weighted from the body at the edge and free below it
      And they move by the cloth follower already built (sdk/body/src/cloth.ts: each vertex strays from its
        skinned place by its own distance, with no collision shapes)

    @done
    Scenario: CH45a. One command from words to a garment
      # 11-15 (tools/wardrobe.sh NAME "what it is" [UNDER] in the test project). It writes each view's prompt
      # (what is seen from that side, what to keep), asks for the four pictures at once, says what they cost,
      # and measures the garment; with UNDER it adds a layer over a garment already made. Run once on a new
      # outfit (a red hoodie, black joggers, black shoes): 20 cents and 36 seconds to the garment file, then
      # worn through the walk. Learnt: black cloth reads as the mannequin unless the mannequin's grey is told
      # apart from black; the hood was drawn up though asked down. It is a script in the test project, not
      # yet a `raea` verb, and it makes no garment by hand or by procedure.
      Then a line of words becomes a garment file with its cost said

    @done
    Scenario: CH45b. `raea clothes`: garments from words, by procedure, and worn
      # 11-15. A shell verb (engine/cli/src/clothes.ts reads the words, wardrobe.ts does the work on the native raea;
      # the Blender scripts are tools/clothes, shipped with a release). The first use fetches SOMA-X's two body files
      # at a fixed commit, checked by SHA-256, into raea's models folder (soma/, with the Apache-2.0 licence), and makes
      # SOMA's body folder and the mannequin's four views there once (18 s on this Mac). `make NAME "words" [--over
      # A,B]` draws the four views at once (flux_2_pro, one picture a view) and measures them onto SOMA's template
      # vertices: assets/clothes/NAME.json, its pictures kept in assets/clothes/NAME/views as its source. Over a
      # garment made by procedure, or several at once, the mannequin is first drawn wearing them, and the layers beneath
      # are taken as one for the stand-off. `cover NAME --parts chest,torso,waist,upperArm:0.45 --colour #e8e4da` makes
      # one by procedure in the same file with no picture: the skin of the parts named (a limb as far along as its
      # share), with a one-ring hem allowance so garments on neighbouring parts overlap. `wear OUT_DIR A B C [--body
      # DIR]` writes a body folder (each garment's glb and picture, the body without the skin wholly under cloth, its
      # skeleton, joint stand-in and collar) and prints the composition's lines. `remeasure NAME` measures a garment
      # again from its kept pictures, for nothing. Run on an outfit: a t-shirt and shorts by procedure, a flannel
      # overshirt drawn over them for 20 cents in 35 seconds, all three worn on the headless body with the generated
      # head through the matched walk (sheet3d compositions/soma_wardrobe.tsx). Mended in the measuring on the way: each
      # garment's open edge is evened (it ran in a stair along the body's triangles; the body's own UV seams are left
      # where they are); a garment stands off each part no further than that part allows (a loose sleeve drawn hanging
      # below the arm had made a balloon of it); the top of a shoulder, and the slope by an open collar, is cloth when
      # there is cloth in front of it and behind it; a texel that the chosen view shows as backdrop or bare mannequin
      # takes the garment's measured colour (grey cuffs). Wrong still: the overshirt hangs to the hips in every
      # picture but wraps the hips when worn (a skin-offset garment can't hang free: CH44), the views disagree on
      # sleeve length (the back drew it to the elbow), and a sliver by the collar is still open. The verb makes one
      # garment a call; an outfit is several calls, innermost first.
      Then a garment is made by one command from words or by procedure, and an outfit is worn by one more

    Scenario: CH45. From words to a wardrobe
      Then `raea` takes a description of an outfit, draws its layers and views, builds the garments, and writes
        them beside the body, saying what each picture cost; the pictures are kept with the garment as its source
      And a garment can also be made by hand or by procedure in the same file format, with no picture at all

    @done
    Scenario: CH46a. A dressed body measured through a walk
      # 11-15, first cut (tools/clothes_check.py in the test project; skins the body and each garment as the
      # engine does, twist joints included, every 15th frame of a performance). Per garment: how deep its cloth
      # goes under the skin, how deep under the layer beneath, and how far its edges stretch from rest, with
      # the joints that carry the stretched edges. On the two-layer outfit through the matched walk: the
      # t-shirt, jeans and shoes 17 mm under the skin at worst, 1.7 per cent of edges over 1.5 of their rest
      # length (at the armpits, the crotch and the feet); the jacket 40 mm under the skin and 29 mm under the
      # shirt at worst, 6 per cent of edges over 1.5 (the chest, the forearms, the armpits). The depth is read
      # against the nearest vertex of the surface beneath, which at an armpit or the crotch can be another
      # limb's, so those two numbers overstate; the stretch is exact. Nothing is fixed by it yet: a garment's
      # mesh bridges arm and trunk where the body's does, and the rest pose's spread arms close in a walk.
      Then a garment's faults in motion are numbers, by joint

    Scenario: CH46. Checked like motion is
      Then a dressed body is run through the library's captures and measured: skin showing through cloth, cloth
        through cloth, and edges stretched past a limit, per garment

    @done
    Scenario: CH47a. Garments from sewing patterns, cut to SOMA's measurements and draped by a cloth of our own
      # 11-15, after the user saw `raea clothes make` on the body ("on the body it looks terrible"; "the 4 panels
      # are not consistent at all") and asked whether clothes are thought of wrong. They were: a garment painted
      # onto a shell of the skin can't show a cut, a hem or a fold, and four pictures drawn apart never agree.
      # Clothes are flat pieces cut to a pattern and sewn, their shape from the cut, gravity and the body.
      # GarmentCode (ETH Zurich, pygarment, MIT) writes patterns as programs of modular components (bodices,
      # sleeves, collars with a hood, a turtle neck or a lapel, cuffs, waistbands, trousers, seven skirts)
      # joined by seams, sized from 26 body measurements. Its simulator is NOT used: it is a fork of an old
      # NVIDIA Warp under a non-commercial licence (mainline Warp is Apache-2.0 since 1.6.2), and its sample
      # SMPL bodies are research-only; SOMA replaces them. Blender's own cloth with sewing springs failed four
      # ways (crumpled, slid off the shoulders). What worked, in the test project's scratch tools: SOMA measured
      # in GarmentCode's terms (bust 81, waist 73, hips 93 cm: slimmer than its average body); a design is
      # settings by path (sleeve.length=0.95 collar.component.style=Hood2Panels), which is what a sentence has
      # to become; the cut written as flat pieces, their place round the body and the seams as vertex pairs;
      # then a position-based cloth (every edge held to its length in the flat cut, each seam's pairs pulled to
      # one point, the cloth pushed out of the skin by the nearest point, gravity let in once the seams close)
      # on the CPU. A T-shirt (6,042 vertices) closes its seams by frame 30 (0.5 mm at the middle) and hangs
      # from the shoulders with 20 per cent stretch at the 95th percentile, in 2 minutes; trousers (15,368) in
      # 4.4, a hoodie with its hood and cuffs over them (12,541) in 3.4, no cloth inside the skin in any. Each is
      # weighted from SOMA's skin by nearest face and worn as a Character part through the matched walk, with
      # the skin wholly under it left out of the body (sheet3d compositions/soma_patterns.tsx). The flat cut is
      # each garment's UVs, so a fabric is one tiling picture with no seams between views. Faults: the hood is
      # drafted up and holds the hoodie high (above the waist); wide trouser legs, skinned from the nearest skin
      # and not simulated as they walk, stretch a sheet between the legs at a full stride (CH44's follower is
      # the fix); GarmentCode places pieces for its own body, so SOMA needs them lifted 7 cm while they are
      # sewn; no open-front shirt or pocket component exists yet.
      Then a garment is a pattern cut to the body and draped, and it walks with the body

    Scenario: CH47. Clothes are sewing patterns (proposed: the user decides)
      # Proposed 11-15 from CH47a. It would replace CH40-CH42's picture measuring as the way a garment's shape
      # is made; the image model would draw only fabric (one tiling swatch) and prints placed on a piece.
      Then a garment is a pattern program's settings, cut to the body's measurements, draped once per body
        shape at build, and kept as a skinned mesh with its flat cut as its UVs
      And words become settings (a language model picks components and values), never pictures of the body
      And layers drape in order, each over the one beneath, and creatures get their own pattern blocks

    Scenario: CH48. A garment model that dresses SOMA in anything (proposed)
      # The user's answer to CH47, 11-15: "clothing or what you wear could realistically be anything. I wonder
      # what we could do if we trained a small model like kimodo did or idol did, build off some existing
      # weights we ship and create something that builds more intelligently". Patterns make only what is cut
      # and sewn; armour, shoes, hats, a backpack or a costume are not patterns. Kimodo is good because it
      # learned in one fixed frame (SOMA's skeleton), so what it makes plays on our body unchanged: the garment
      # model would make garments in SOMA's rest pose and frame, so fit, skinning and layers come free. The data
      # comes from CH47a's tools run as an engine (GarmentCode's design sampler across SOMA's shapes, draped by
      # our cloth on the GPU, rendered from the mannequin's cameras, each example labelled exactly: its design,
      # its pattern, its 3D on the body, its pictures), plus hard-surface things drawn and made 3D by Idol and
      # cut from the body. GarmentCodeData itself is not used (draped on SMPL bodies, its licence unconfirmed).
      # The base weights would be Idol's (Apache-2.0, shipped): one picture of the mannequin dressed in anything
      # in, the garment alone out, in 3D on SOMA. Idol ships no training code, so its fine-tune is ours to write.
      Then a garment is made by a model from words or a picture, on SOMA's body, as a layer
      And the training data is ours, made by our own tools on SOMA, with where each example came from
      And its cost is said before any training is paid for

    @done
    Scenario: CH48a. The garment data engine: random designs cut to SOMA, draped on the GPU, pictured, labelled
      # 11-15, CH48's data half (tools/garments/engine.sh OUT N SEED). GarmentCode's sampler draws a design and
      # its own checks refuse about 60 per cent; each kept one is cut to SOMA and draped by CH47a's cloth on the
      # GPU (MLX): the body as a 7 mm signed distance grid signed by winding number (the nearest face's normal
      # put patches of "inside" in the air above the shoulders), every link solved at once, compiled. The
      # T-shirt that took 125 s on the CPU takes 7 s with the same seams (0.83 mm median) and stretch. Three
      # things threw garments off, each found from snapshots of the drape: the cloth was too soft under gravity
      # (+18 per cent stretch at the 95th percentile with 10 solver passes a substep: armholes opened over the
      # shoulders; 60 passes give +5); SOMA's bust had been read at the chest's most forward level (81 cm), not
      # the fullest girth under the arms (89.9), so fitted tops were cut too small; and GarmentCode's own drapes
      # pin labelled edges to planes (a waistband's top at the waist, a neckline inside the neck's sides, a
      # strapless top's edge at the armpit), without which gathered waistbands (up to 1.9 times the waist) and
      # strapless tops fall. Ours are one-sided planes kept on, as elastic, boning or the neck holds them, a
      # low-rise band pinned between waist and hips by its rise. Then 24 designs from 61 draws all hang right,
      # 4 to 33 s each (about 7,000 a day on this Mac, $0). Rendered properly, the cloth first looked pebbled:
      # the bake left it flat-shaded, so its glb had three vertices for every triangle (198,881 for 66,305) and
      # shaded as facets everywhere it was drawn; smooth, it is one surface. The fabric is also stiffer than
      # CH47a's (bending 0.3, squash 0.6 against 0.05 and 0.25), which buckled at the mesh's own scale; folds
      # stay and stretch is +6 per cent at the 95th percentile. An example is its design's settings (the labels), a
      # caption, the pattern, the flat pieces with seams and edge labels, the draped mesh, a skinned glb, four
      # pictures on the mannequin and the drape's numbers. Faults: GarmentCode's space is mostly dresses and
      # skirts (no open-front shirts, jackets, pockets, shoes or hats); one body shape so far; no collision
      # between the layers of one design; the captions are written from the settings, not seen.
      Then one command makes N random garments, each worn on SOMA and labelled with its design
      And a garment that slides off is caught by the drape's own check, not by eye

    Scenario: CH48b. The training data at scale, with periods, genres and things that are not sewn (proposed)
      # The user's choice, 11-15: scale the data and train now, before writing the missing pattern pieces. And,
      # in his words: "use examples of time periods, futurism, armor/suits/helmets/hats/etc". The engine's lean
      # mode (LEAN=1) keeps what training needs, about 0.4 MB a design: the design, its caption, the drape's
      # numbers and the draped mesh; pictures and glbs are made again from those for the examples that need
      # them. A caption from the settings alone ("fitted sleeveless top, trousers") can't name a period or a
      # genre, so captions also come from a picture of the garment on the mannequin, written by an image-reading
      # model asked what the garment is and what it evokes (Edwardian, 1970s, workwear, futurist, uniform), with
      # its price per caption said first. What can't be sewn comes from CH49.
      Then the training set holds sewn garments captioned by their settings and by what they look like
      And worn hard pieces from CH49, each with its words, its picture and its 3D on SOMA

    Scenario: CH49. Worn hard pieces: helmets, hats, armour, visors (proposed)
      # Patterns make only what is cut and sewn. A helmet, a hat, a breastplate or a visor is made the way the
      # user first proposed for clothes and the creature flow makes animals: a picture of SOMA's mannequin
      # (one view, its camera written down) is dressed by an image model, keeping the pose and the view; Idol
      # (Apache-2.0, run on this Mac) makes the dressed figure 3D; it is laid on SOMA by its camera and size; the
      # body is cut away (everything within a few millimetres of the skin, by the body's distance grid); what is
      # left is split by the bone it lies nearest, and each piece rides that bone rigidly, as the head's cap and
      # glasses ride Head (CH21). It differs from the rejected picture-measured clothes: those painted cloth onto
      # the skin from four separate pictures that never agreed; here one picture is made 3D whole, and only rigid
      # things are kept, so nothing has to fold.
      Then words make a helmet, a hat or armour that SOMA wears and moves with
      And each piece is its own part on the bone it covers, so pieces mix with sewn garments as layers
