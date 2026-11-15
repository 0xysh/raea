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
      # Not in the vocabulary yet: take, put, throw, hand over, use (the Rust takes are not called from here),
      # and do (a line of text made into a clip, which waits on CH11). A controller's unmet ask is not logged
      # yet (CH15). A standing character sometimes plays a capture with its arms raised: the library's stand
      # clips want curating for SOMA.
      Then a script and a controller give acts in one vocabulary and the matcher carries them out

    @done
    Scenario: CH18a. The takes run on a SOMA body
      # 11-15. The Rust arms and hands (docs/hands.feature) find a joint by its role on either rig, so reach,
      # grab, carry and throw run on SOMA with no second copy. The audit's six takes on SOMA's skin: holds
      # within 2 to 8 mm and no body skin in a carried thing, as on the old rig; the fingers are not tuned to
      # SOMA's hand (hand skin 22 mm into a box, no pads on a cup, 41 to 47 mm into the table as a hand leaves).
      # The joint limits have a SOMA table (the body SDK's, measured over the library on SOMA). The fingers'
      # rest curl is close to the old rig's (the thumb's first bone lies 31 degrees off the hand against 69),
      # so the misses are not a matter of rest pose alone: not found yet.
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

  Rule: Clothes: drawn on the mannequin by an image model, kept as data on SOMA's own skin

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

    Scenario: CH45. From words to a wardrobe
      Then `raea` takes a description of an outfit, draws its layers and views, builds the garments, and writes
        them beside the body, saying what each picture cost; the pictures are kept with the garment as its source
      And a garment can also be made by hand or by procedure in the same file format, with no picture at all

    Scenario: CH46. Checked like motion is
      Then a dressed body is run through the library's captures and measured: skin showing through cloth, cloth
        through cloth, and edges stretched past a limit, per garment
