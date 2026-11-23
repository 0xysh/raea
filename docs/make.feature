@spec
Feature: Make: scenes, kits and shots built to a frozen plan and held to a review loop the agent can't skip
  Shane, 2026-11-23, after a deep dive into apekshik's AnyScene, AnyKit and AnyShot (github.com/apekshik, MIT): "lets
  take all three learn form them and adopt that review loop internally wholesale. i think this is ley for us. all 11
  points seem relevant to adopt or itnegrate intoexisting systems, or converting into a typescript planning and review
  sdk as part of it". Decided the same day: the contract goes on brief.ts; the gate uses `raea ask`; internal and cloud
  are built together. Plan: ~/.claude/plans/dreamy-floating-dijkstra-of-771a18a0-2207-4b03-80e7-ddbec117bc80.md.

  What the three repos showed (their builder/CRAFT.md and agent/KIT.md say it outright): the quality came from a loop,
  not a model or a setting. Render it and look; name the problem precisely; fix it in a script so a rebuild keeps it;
  check the fix from a view that shows it (a camera 30 cm off the floor shows a floating chair a wide shot hides); get
  an independent critic, because the builder stops seeing its own flaws. And a Stop hook that won't let the agent end
  until every deliverable exists and the critic passes, with a cap and a stall rule so it can't loop forever.

  The model this file sets:
    1. One loop, three kinds of job: a scene (a dressed, lit place from references or a brief), a kit (a set of props
       that belong together, from style references) and a shot (stills or camera moves of a built scene, from words).
    2. The plan is the contract: `make` on the project's brief.ts (core/brief make.rs), typed from Rust, written by the
       agent before it builds, and read by Rust. Every object carries its real size and whether it came from the
       reference or was invented; every dial its value, where the value came from, and what it meant here.
    3. Done is decided in code (`raea make status`), never by the agent saying so, and enforced where the agent stops:
       the Stop hook of every agent raea installs hooks for, locally and on raea cloud.
    4. Three tiers of gate, each gating once it has earned it: measured checks always; `raea ask` questions one by one,
       once each is calibrated on the person's labels; a fresh critic's fixes always (fixed or answered), its score
       once `raea judge calibrate` agrees with the person's eye.
    5. Every job ends with its notes, including what the skill got wrong for it, so the next job starts from them.

  Background:
    Given raea's scene loop as advice (sdk/guides/craft/bevy/scenes.md, from the 11-06 research): brief, routing,
      build big to small, `raea render --review` (hero, front, right, top, iso and low cameras in colour and clay;
      core/review), `raea judge` (frozen rubric; clef, kimi or claude; pass, done, stall in code; apps/judge),
      `raea scene lint` (raea-sys-contact) and `raea gen check`
    And `raea ask` (docs/ask.feature): typed clef questions with a ledger that calibrates each question on labels
    And the raea plugin's hooks (skills/skills/hooks/hooks.json -> `raea hook claude`, apps/check hook.rs), whose Stop
      branch already blocks a turn's end to hand over what the person sent from the app
    And the project's record (.raea/record/, core/facts record.rs): what each render and check found, and when

  Rule: the plan and done (MK0-MK3)


    Scenario: MK0. The contract: `make` on brief.ts
      Given a brief.ts whose default export has `make: kit({...})`, `scene({...})` or `shot({...})` (raea/brief)
      Then core/brief reads it as `Make`: its kind, references, style (one paragraph: era, palette, materials, wear and
        one fictional maker), dials, objects, shots, deliverables and a round cap (8 by default)
      And each object has an id, a label, one size (`h`, `dims` [w, d, h] or `max` for things that lie flat), a route
        (solid, rule, gen, library, picture), a prompt, `source` (reference or invented), a `crop` in reference 0, a
        place (x, y, z, rot) for a scene, the generator it was picked from and why, and the fixes made by script
      And each dial is `{ value, source: customer | reference, means }`, from its kind's set (scene: density, tidiness,
        wear, wetness, time_of_day, weather, atmosphere, power, warmth, look; kit: count, variety, detail, wear,
        wetness, warmth, look; shot: res, seconds, lens, handheld, look)
      And the TypeScript is generated from those types (sdk/raea/src/brief.gen.ts), so tsc checks what Rust reads
      And `doctor` names what the plan still lacks: no objects, an object with no size or no route, a dial outside its
        kind's set, a generated object with no prompt

    @needs:MK0
    Scenario: MK1. `raea make status`: what's missing, decided in code
      Given a project whose brief has `make`
      When `raea make status [DIR] [--json]` runs
      Then it lists what's missing for the kind:
        | kind  | deliverables (under out/, overridable with `deliverables`)                                              |
        | scene | stills/ with 3 or more pictures                                                                          |
        | kit   | kit/<id>/<id>.glb, kit/<id>/<id>_lod1.glb and kit/thumbs/<id>.png per object, kit/lineup.png, kit/kit.json |
        | shot  | shots/<name>.png or shots/<name>.mp4 per shot                                                            |
      And NOTES.md beside the brief, with a "What the skill got wrong" section
      And each gate's state: the record's last render or check of every composition the plan names is newer than the
        file and found no error; the critic has judged a round newer than the newest deliverable; each blocking defect
        of that round is fixed (the round passed) or answered in NOTES.md by its id
      And the judge's rounds, passes in a row and stall, read from review/judge.log.jsonl as `raea judge` keeps them
      And `done` is true when nothing is missing and every gate is clear, and also when the job is capped (its rounds
        reached the cap) or stalled; a capped or stalled job is delivered short, with what's missing listed

    @needs:MK1
    Scenario: MK2. The Stop hook: the agent can't end a job that isn't done
      Given an agent session (Claude Code, codex, gemini, kimi) with raea's hooks, in a project whose brief has `make`
      When its turn is about to end and `raea make status` isn't done
      Then the hook blocks the stop with "Not done:" and what's missing, and the agent keeps going
      And a session blocked three times with the same list goes through (the agent is going in circles; the status
        says so), as PL7's keeper stops after asks that change nothing
      And a project without `make` is never blocked, and a brief that doesn't load blocks nothing (the hook says why)

    @needs:MK2
    Scenario: MK3. Gates in plan.org
      Given the project's plan.org (docs/plan.feature)
      Then `raea make status --plan` writes each gate as a check line under the job's task (sdk/task `setCheck`), so
        the person, the app and the cloud see the same list the hook holds the agent to

  Rule: the three tiers of gate (MK4-MK6)

    @needs:MK1
    Scenario: MK4. Defects as calibrated ask questions
      Given the judge's yes/no defects (apps/judge rubric.rs: floating, intersecting, blobby, stand_in, clones,
        flat_colour, plastic, ...) and the loop's own (a cohesion outlier on a reference sheet, a real brand, garbled
        hero text, a disconnected assembly, opaque glass, a camera through a wall)
      Then each is an ask question with versioned words (core/ask), answered on the gate's sheet and kept in the ledger
      And it gates once `raea ask calibrate` passes it on the person's labels (16 problem, 16 clean, held-out AUC 0.8);
        until then its answer shows in the status as advice

    @needs:MK1
    Scenario: MK5. A rubric per kind for the fresh critic
      Then `raea judge --kind scene|kit|shot` asks the kind's categories (scene: layout, props, screens and signage,
        materials, light, atmosphere, plausibility, composition; kit: cohesion, reference fidelity, brief, mesh,
        texture, assembly, scale, labels; shot: framing, move, focus, clearance, continuity, grade)
      And the critic is told the bar is a production base an artist then refines, judged at normal viewing distance,
        no points off for choices that differ from the reference, at most five fixes, most damaging first

    @needs:MK5
    Scenario: MK6. The critic's score gates once it is calibrated
      Given `raea judge calibrate` reporting agreement with the person's eye for a kind
      Then that kind's jobs also need two fresh passes in a row to be done; before that, fixed-or-answered is enough

  Rule: measured placement (MK7-MK9)

    Scenario: MK7. Grounding at contact points
      Given a part whose box bottom isn't where it touches down (splayed feet, a handle, an origin offset)
      Then `raea scene lint` casts rays down from its lowest vertices, ignoring itself, volumes and decals, and reports
        `floating_contact` with the gap and its `{dy}` fix; hanging kinds (signs, pendants, lamps on cords) only with
        --all; a fix only ever settles a part down

    Scenario: MK8. The clip check, for every level
      Then the triangle check FLOOR3 used (projects/floor3 tools/clips.ts) is `raea scene lint --clips`: intersections
        with depth, z-fighting faces, flicker inside one model, and nothing within 3 mm (floating)

    Scenario: MK9. Re-drop after every move
      Then a part moved sideways by a fix is settled onto what's under its new place in the same pass, and stools drop
        from below the table top, so nothing hovers at table height

  Rule: kits (MK10-MK13)

    @needs:MK1
    Scenario: MK10. References made in one style, checked on one sheet
      Then each object's reference is `raea gen image --edit` with the style images as input, the plan's style
        paragraph and the scan template, and `raea make sheet refs` puts them all on one sheet before any 3D
      And outliers (another style, a real brand, cropped, two objects) are regenerated first: the cheapest place to fix
        cohesion

    @needs:MK10
    Scenario: MK11. Both generators, picked on clay
      Then every reference goes to Tripo H3.1 (face limit 150k heroes, 60k small) and Meshy, and `raea gen pick ID...`
        renders reference | Tripo beauty | Tripo clay | Meshy beauty | Meshy clay under one light, and records the
        pick and a one-line why (the earlier ~150 props: Meshy ~70%, small items and printed labels; Tripo the big
        hard-surface heroes)

    @needs:MK11
    Scenario: MK12. Prep and LOD
      Then `raea gen check -o` fits each pick (front -Y, real metres, bottom-centre, textures at 2K or less) and
        `--lod1` writes a quarter-triangle copy, each with its .glb.json saying how it was made

    @needs:MK12
    Scenario: MK13. A kit is a component set
      Then a finished kit is shared/components/<kit>/ (one component per object with its fixture), with a `<Lineup>`
        composition (scale bar, labels), thumbnails and kit.json, so `raea component add` puts it in any scene

  Rule: shots (MK14-MK17)

    Scenario: MK14. Camera moves, focus and the handheld walk
      Then raea/camera has static, dolly, truck, pedestal, crane, pan, tilt, orbit, dollyZoom (the lens solved per
        frame to hold the subject) and path (Catmull-Rom), focus track, point and rack with holds, and
        `handheld({ profile: "walk" })`: steps at 1.8 Hz, an aim on a spring, small reframes, nothing above 1.9 Hz

    @needs:MK14
    Scenario: MK15. Blocking: framing sheets and a lens chart
      Then `raea render --framing` stamps each shot at t = 0, 0.5 and 1 on one sheet, and `--lenschart SHOT` its first
        frame at 18, 24, 35, 50, 85 and 135 mm

    @needs:MK14
    Scenario: MK16. Clearance on every frame
      Then `raea scene stage --clearance 0.45` finds every frame where the camera is nearer a solid than that, leaving
        out volumes, particles, decals, glass panes meant to be passed and anything under 5 cm

    @needs:MK14
    Scenario: MK17. Shots from words and from a floor plan
      Then the shot-language guide maps a crew's words to moves (push in, truck, reveal, rack focus, walkthrough at
        0.4-0.6 m/s and 1.5-1.7 m), and a floor plan with a drawn path is registered to an ortho top render and its
        path becomes `path` keys; grade looks (clean, cinematic, gritty, vintage) are raea/post presets

  Rule: the plan, drawn (MK18)

    @needs:MK0
    Scenario: MK18. `raea make plan`: a floor plan, a grey box and a comparison
      Then it draws floor-plan.png (labelled footprints, invented ones dashed, lights, the reference camera's cone),
        renders a grey box of the plan from the reference camera (raea/room and boxes, `raea render --review`) and
        puts the reference beside it, for the person to approve before anything is generated

  Rule: raea cloud (MK19-MK20)

    @needs:MK2
    Scenario: MK19. Three skills on raea cloud
      Then raea cloud has "Scene from references" (stops: plan, spend, picks, finals), "Kit from style" (plan,
        reference sheet, spend, lineup) and "Shots of a scene" (shot list, framing, finals), each stop's checks calling
        `raea make status --json` on the box

    @needs:MK19
    Scenario: MK20. Retry-safe jobs and a live view
      Then a project is keyed by a hash of its inputs: the same ask returns the finished project, attaches to the
        running one or resumes it without paying again for what's already generated
      And every picture the agent opens goes to the live watch with a preview, and a heartbeat every 20 s repeats the
        milestones so far for anyone who joins late
