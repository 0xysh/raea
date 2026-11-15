Feature: Creatures: a four-legged body in the character SDK
  Set by the user 2026-11-14, with the trashpanda project (a stylized raccoon that walks, sneaks, crawls, stands and
  walks on two legs, climbs a tree, dangles from a wire, vaults, kicks off a wall, holds things, and goes limp): "if
  you feel that a four legged creature has merit in the character sdk then also make your PRs alongside the other
  work to include them and what you find".

  It has merit. The character SDK is one skeleton (MakeHuman's game_engine: 22 body joints and 30 finger joints) and
  one way of moving it (captures retargeted joint by joint, a motion matcher that picks frames, IK only to hold a
  contact). None of that reaches an animal: there is no capture library for a raccoon, a dog or a cat on our rigs,
  and retargeting a person's clip to four legs is not a thing. An earlier game's dog and cat were posed by a solver
  written inside that project, and called stiff. So an animal needs the other half of what docs/hands.feature
  began: motion made from what the body is doing, with no capture of it. This file is that for the whole body.

  What was found, building the raccoon:
  - Skeleton first holds for a built model as it does for a drawn one. The joints are set down by measure; the body
    is then laid round them as solids, so every limb is where its bone is and the weights have something true to
    start from.
  - Where a limb joins the trunk decides whether the skin tears, more than the weights do. With the knee and elbow
    touching the belly at rest, a voxel union fuses skin across them, and no weighting saves an edge whose ends ride
    on joints that move 10 cm apart: 0.4% of edges torn at a walk. Moving the knees and elbows clear, so a leg joins
    the trunk only at its hip or shoulder, took the walk to 0.05% with the same weights. Smoothing weights harder
    made it worse (0.2% to 0.4%): it spreads a far-moving joint's pull into the trunk.
  - Bone heat gives the belly to the elbows and knees (they sit just inside it, the spine far above). The trunk's
    skin has to be given to the spine outright: each point to the two spine joints it lies between.
  - Feet kept in the world, not in the gait's cycle, are what stop sliding: a planted foot is a point that does not
    move until its leg's turn comes. Measured slide is 0 mm in every move, turning and stopping included. A cycle
    played under a moving body (that earlier solver) slides whenever speed or heading changes.
  - One rigging (the hips' and shoulders' frames, four limb ends, the head's turn, the tail) is enough to pose every
    move, and blending riggings rather than poses lets one move ease into the next with the limbs still solved.
  - A small animal's legs are short for its stride: at standing height a foreleg reaches 8 to 11 cm ahead of its
    shoulder. Strides have to come from the leg's length (a walk's stance travels about one leg length), or the
    elbow locks every step.
  - Fur as shells (16 copies of the skin pushed out along its normals, each cut by one height map at its own height)
    draws on a skinned model in the raster renderer at no visible cost: 232k triangles, 11 ms a frame at 1280x720.
  - A limp body made of points must not gain speed from being pushed out of a solid: the point's last place moves
    with the push. Without that, a body that flops half over a box leaves at 16 m/s.

  Rule: Built

    @done
    Scenario: CR0. Where it lives
      # sdk/body/src/creature, engine-free TypeScript beside the person's body layer, exported as
      # `creature` from raea/body. The moves that need a world (CR7 on) are in the trashpanda project until they
      # have settled (its game/ folder).
      Then the plan:
        | crate | file                           | lines | job                                                                  |
        | sdk   | sdk/body/src/creature/q.ts     | 40    | the body layer's maths under short names; a bend from places; a turn from axes |
        | sdk   | sdk/body/src/creature/body.ts  | 45    | a rig's measures: sockets, bone lengths, where each foot belongs, reach |
        | sdk   | sdk/body/src/creature/loco.ts  | 185   | the gait by speed and posture; feet planted in the world and stepped   |
        | sdk   | sdk/body/src/creature/pose.ts  | 225   | the rigging, the solver, the blend between riggings, the tail on springs |
        | sdk   | sdk/body/test/creature.test.ts | 60    | slide, gait, every joint posed, feet squared up on a stop             |

    @done
    Scenario: CR1. A creature's rig is read from its joints
      # A rig is joints by name with a parent and a place at rest, every joint resting unturned (the engine places
      # each by its own transform), and `marks` for the points that end a bone and carry no skin (nose, tail's tip,
      # toes). Names: root, spine1..3, chest, neck1, neck2, head, jaw, ear_l/r, lid_l/r, tail1..N; fore legs fl/fr
      # _scap, _upper, _lower, _wrist, _hand, digits _d1a/_d1b.._d5a/_d5b; hind legs hl/hr _upper, _lower, _hock, _paw.
      Given a rig.json with those joints
      When bodyOf reads it
      Then it has each leg's socket, bone lengths, reach and where its foot belongs under the body

    @done
    Scenario: CR2. A planted foot does not move
      # Each foot is a point in the world. It stays until its leg's turn in the gait, then swings to where the body
      # will need it half a stance after it lands, aimed again every frame of the swing (a turn mid-step still
      # lands right). A foot left more than 15 cm (fore) or 19 cm (hind) from its place goes at once.
      Given a body walking, trotting, galloping, sneaking, crawling and walking on two legs, turning all the while
      Then no planted foot moves between two frames
      # measured 2026-11-14: 0.0 mm in all six (sdk/body/test/creature.test.ts; tools/locotest.ts in the project)

    @done
    Scenario: CR3. The gait is the speed's
      # Stride, the share of a stride each foot stands, each leg's place in the stride and the lift are blended by
      # speed: a walk (lateral sequence, 0.66 standing) to 0.8 m/s, a trot (diagonal pairs, 0.5) to 2.1, a bounding
      # gallop (hind feet together, 0.36) above. Lowered, steps shorten and stand longer; on two legs the hind feet
      # alternate.
      Then stride grows and standing share falls from walk to trot to gallop

    @done
    Scenario: CR4. One rigging poses every move
      # The solver takes the hips' and shoulders' frames, four limb ends (a place, a turn, how far the wrist or heel
      # is rolled over it, how far the fingers are closed, where the elbow or knee points), the head's turn and the
      # tail's joints, and gives every joint's place and turn: the back laid link by link between the two frames,
      # the shoulder blade sliding with its leg, elbow and knee bent in a plane, the neck sharing the head's turn.
      Given the body on the ground, lowered, and up on its hind legs
      Then every joint is a number in every frame, and at a walk and a trot every leg reaches its foot

    @done
    Scenario: CR5. Standing still, it squares up
      Given a body that galloped and stopped
      Then within three seconds each foot is within 3 cm of where it belongs, brought there one at a time

    @done
    Scenario: CR6. The tail is late
      # Its joints are points pulled toward where a stiff tail would be, stiffest at the root, kept their lengths and
      # off the ground; how it is carried (up at a gallop, low sneaking, down behind on two legs) is the posture's.
      Then the tail follows the hips' turns after them and never passes through the ground

  Rule: Built in the project, to come into the SDK

    Scenario: CR7. Moves off the ground share the rigging
      # In the project (game/player.ts): in the air (nose along its path, hands reaching for the ground coming down),
      # a vault (hands down on the thing's top, hind legs tucked through), a pull-up onto a ledge (hands on top, hind
      # feet scrabbling on the face), a kick off a wall, climbing a trunk (diagonal pairs, hands that stay where
      # they grip), hanging from a wire by the hands (a pendulum the player pumps, hand over hand along it), and
      # gone limp. Each builds a rigging; a move eases into the next over 6 to 34 frames by blending riggings.
      # A tour of all of them runs headless with every frame posed (tools/playtest.ts in the project).
      # To do here: the world they ask of (ground height, what is ahead, a ledge, a trunk, a wire) as an interface.
      Given a level that answers those questions
      Then a creature jumps, vaults, pulls up, kicks off, climbs, hangs and lands on it

    Scenario: CR8. Gone limp, and up again
      # In the project: nine points on springs (hips and shoulders left and right, the head, four feet), each foot
      # kept within its leg's reach of its socket; pushed out of solids without gaining speed; when it lies still
      # the body gets up by blending from the limp rigging to the standing one.
      Then a fall from 2 m tumbles, comes to rest in under 2 s and stands up where it lies

    Scenario: CR9. It takes things
      # In the project: on four legs a thing is taken in the mouth; up on its hind legs both hands close on it and
      # turn it over; thrown from either. To do: docs/hands.feature's holds and finger curl on a creature's hand
      # (five digits of two joints), so the fingers lie on the thing rather than at a fixed curl.
      Then a thing held is in the mouth or between the hands in every frame, and the fingers touch it

  Rule: To do

    Scenario: CR10. The skin holds in every move
      # Measured 2026-11-15 (tools/rigcheck.ts in the project: edges past 0.4x or 1.6x their length by more than
      # 8 mm, as a share of all edges): stand 0.003%, walk 0.05%, turn 0.06%, rise 0.04% pass; trot 0.23%, sneak
      # 0.18%, crawl 0.87%, gallop 1.2%, two legs 1.5% do not, all at the hip and shoulder. A leg folded hard against
      # the body needs more than linear blending at its socket: a helper joint in the crease, or a corrective shape.
      Then no move tears more than 0.1% of the skin's edges

    Scenario: CR11. A gallop with a back
      # 11-15: every leg now reaches its foot at a gallop (a stance of 0.29 of the stride, the body 14 to 16% lower
      # at speed; it was 87 of 1,200 leg-frames short). Still rough: the back's arch following the legs, the hind
      # legs' reach past the fore, and a real moment with all four feet off the ground.
      Then at a gallop every leg reaches its foot and the back's arch follows the legs

    Scenario: CR12. Other animals on the same solver
      # The solver reads only the rig's names and measures. A cat and a dog (an earlier game's rigs carry the same
      # leg joints, without digits) should need their gait's numbers and nothing else.
      Given a cat's and a dog's rig
      Then both pass CR2 to CR5 with their own stride and standing shares

    Scenario: CR13. In the engine
      # As docs/hands.feature moved the hands to Rust (raea-sys-body), a creature's solver belongs beside it once
      # the TypeScript has settled, with <Creature> in raea/character drawing a rig from a rigging.
      Then <Creature rig pose /> draws a four-legged body, and character.creature answers a rigging's pose
