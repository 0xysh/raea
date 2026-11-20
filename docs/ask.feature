@spec
Feature: Ask: questions about a piece's picture and sound, answered by the decision model and paid from the person's plan
  Shane, 2026-11-20: "a decision reviewer type api that calls and utilizes their raea subscription ... for checks and
  things, or extending checks to be done quickly based on audio and video, maybe even being able to ask a question
  about how does this sound based on the visuals". Planned "so that it sufficiently understands first what is
  required of it, what types of options come standard, how ask works and can be improved over time". And: "the agent
  or human may want to have a check automated in their own pipelines that asks, or ... fingerprint checks to also call
  an ask call to double check, or ... a script that collates a bunch of ask calls in parallel and processes the needed
  clips (which should be the case anyway)".

  The model this file sets:
    1. The decision model is clef-omni on Workers AI: typed questions (yes or no, a choice, a graded level) about text,
       up to 4 stills, 4 sounds of 5 minutes and 2 videos of 60 s at 2 fps, answered in probabilities, never in free
       text; the same request gets the same answer. It costs by the media it reads (about $0.006 a minute of video;
       sound costs little), not by the questions, so every question about the same media goes in one call.
    2. One way to ask, for every caller: `raea ask` on the command line, a person's own pipeline or CI, a script that
       asks many questions over many clips, `raea check`, the fingerprint double-checking its own flags, the app's
       agents, its critique and game checks, and `raea judge`. One question format, one client, one parallel runner.
    3. Ask understands a question before it spends: what the question needs (picture, sound, both, the words heard, the
       words on screen, the brief), whether this file can answer it, and what the piece is. A question that can't be
       answered isn't asked; it says why, or its variant is asked instead.
    4. It's paid from the person's raea plan by default (Pro's and Studio's monthly pool, which stops when it's spent),
       else from their own Cloudflare account; with neither it refuses and says how to get one.
    5. Local measures come first. Clef gets what can't be measured, only for windows that changed, cached by content.
       Timing (sync, freezes, dropouts) stays local: clef samples video at 2 fps and was weakest there (11-18).
    6. An answer is advice until its question is calibrated on the person's own labels; every answer is kept, so each
       question earns its threshold, a better wording can be measured against the old one, and a well-calibrated
       question can later be taught to a free local head.

  Background:
    Given clef's four callers today, each with its own client and all on the person's own Cloudflare keys: the app's
      critique (apps/app host/critique.ts: the export's first minute at 480 px with its sound as mono MP3, the rubric's
      questions for the brief's modes), its game check (host/playcheck.ts), `raea judge` (apps/judge clef.rs, review
      sheets on a frozen rubric) and the playtest DSL (sdk/app playtest.ts, which holds clef's verified limits)
    And raea-api's `POST /v1/clef/ask` (raea-platform PR #15, not deployed): an account's questions charged to its pool
      (Pro $2, Studio $20 a month, stopping when spent), cached in R2 by request, under a daily ceiling across everyone;
      images and videos only
    And the check packs (render, hook, storyboard, brand: 22 checks), each clef question kept with its `label_words`,
      its AUC, its threshold at 10% false positives and its status (advisory or warn), labelled from people's notes
    And the fingerprint's evidence rows, where another eye's readings (`source: "eye:NAME"`) train a head and never
      count as the person's (FP6d), its AUC and quantile (raea-sys-fingerprint head.rs) and its gates (16 labels of
      each kind, 3 projects, a held-out AUC of 0.8: train.rs)
    And taste lines (core/brief taste.rs): words, a check (one of six forms, a named check, a `.ts` check), values and a
      level, held by `raea check PROJECT` (apps/direction check.rs) through raea's named checks (systems.rs `named()`)
    And what clef has shown: its yes is soft (0.15 to 0.51 on real defects), so a threshold is per question; it answers
      what's asked, so the rubric's sound question, which assumes a voice, called a promo with no voice "weak" until
      `sound-silent` was asked in its place (docs/craft-system.md)

  Rule: AK1. One question format and one client

    Scenario: AK1. Questions, and the standard library
      Given the plan
        | crate         | file                       | lines | job                                                                    |
        | raea-core-ask | core/ask/src/lib.rs        | 40    | the crate's exports                                                    |
        | raea-core-ask | core/ask/src/question.rs   | 220   | a question: id, ask, type (yes, choose, grade, rate) with its options or levels, sees and hears, uses, when, window, label_words, calibration; its version, a hash of its words, type and options; TypeScript by ts-rs |
        | raea-core-ask | core/ask/src/library.rs    | 200   | the standard sets: rubric (read from sdk/guides/craft/rubric.md, one source), av, defects, platform, decide; a set or a question by id |
        | raea-core-ask | core/ask/src/limits.rs     | 60    | clef's limits (questions, ids, options, levels, stills, videos, sounds, sizes, tokens, price), checked before anything is sent |
        | raea-core-ask | core/ask/library/av.json   | 80    | picture with sound: the sound fits what's on screen, the music's mood matches the picture, the music fights the voice, the voice suits who's on screen, the ambience matches the place, effects mark moments |
        | raea-core-ask | core/ask/library/defects.json | 70 | the fingerprint's axes with no labels yet: muffled voice, harsh music, glitch, fake, barren, unreadable text, shaky camera |
        | raea-core-ask | core/ask/library/platform.json | 160 | the check packs' clef questions, with their label words and calibration |
        | raea-core-ask | core/ask/library/decide.json | 50  | the best of these for the brief; the nearest to a reference; whether a fix changed what a viewer sees |
      Then a question is clef's own shape: `yes` (a probability of yes), `choose` (2 to 255 options), `grade` (ordered
        levels, poor to excellent unless given, read as the expected level) or `rate` (2 to 10 levels)
      And `when` says what a question needs to be asked (modes; a voice heard or none; a game), and `swap` names the
        question asked in its place when it doesn't apply, so `sound` to `sound-silent` is data, not code
      And every answer reads as a problem score from 0 to 1 whatever its type (a yes's probability on the answer that
        flags, a choice's on its flagging options, one less a grade's level over its top), so one calibration fits all
      And the limits are raea-api's where they're stricter than clef's (ids of 64 characters, 16 options), so a call that
        passes them passes on every route

    @needs:AK1
    Scenario: AK1b. One clef client, routed to the person's plan
      Given the plan
        | crate        | file                    | lines | job                                                                    |
        | raea-io-clef | io/clef/src/lib.rs      | 40    | the crate's exports                                                    |
        | raea-io-clef | io/clef/src/client.rs   | 160   | one call: state, questions, stills, videos and sounds as data URLs; its answers and cost; apps/judge clef.rs moved here and widened |
        | raea-io-clef | io/clef/src/media.rs    | 180   | the media cut with raea's FFmpeg: a window as 480 px H.264, at most 60 s; its sound as mono 16 kHz MP3; a still as JPEG at most 1,600 px |
        | raea-io-clef | io/clef/src/route.rs    | 160   | the route: the app's host (~/.raea/app-host.json, /host/ask) when signed in with a pool; RAEA_AGENT_KEY to raea-api; the person's Cloudflare keys; else a refusal that says how to get one |
        | raea-io-clef | io/clef/src/cache.rs    | 80    | answers by request hash in ~/.raea/ask/cache, so an unchanged question on unchanged media costs nothing |
      Then a call goes the first way that's open: the app (signing in stays the app's alone), an agent key with the
        `ask` scope (a pipeline, CI, raea cloud), the person's own `CLOUDFLARE_ACCOUNT_ID` and `CLOUDFLARE_API_TOKEN`
      And with none it refuses: "sign in to raea on Pro or Studio, set RAEA_AGENT_KEY, or add your Cloudflare keys
        (docs/settings.md)"
      And `raea judge` asks through it, and apps/judge's own clef.rs is gone

  Rule: AK2-AK3. `raea ask`

    @needs:AK1b
    Scenario: AK2. Ask understands the question before it spends
      Given the plan
        | crate        | file                    | lines | job                                                                    |
        | raea-app-ask | apps/ask/src/prepare.rs | 260   | the file's facts (length, sound, words heard, words on screen, the render's manifest and record) and the project's brief (audience, believe, mode, voice); each question kept, swapped or dropped with why; the state paragraph |
      Then a question about sound on a file with none, or a stills-only render, isn't asked, and says it waits for one
      And a piece where no one speaks is asked `sound-silent` in place of `sound`, and told it has no voice
      And the state says what the piece is (its modes, the brief's audience and belief, its storyboard's logline, what
        raea measured), as critique.ts writes it today
      And `--dry` prints exactly what would be sent (the state, each call's questions, the media and its size) and an
        estimated cost, and spends nothing

    @needs:AK2
    Scenario: AK3. The runner, the record and the verb
      Given the plan
        | crate        | file                    | lines | job                                                                    |
        | raea-app-ask | apps/ask/src/lib.rs     | 60    | the crate's exports and its usage                                      |
        | raea-app-ask | apps/ask/src/verbs.rs   | 260   | `raea ask FILE [QUESTION]` (--choose, --grade, --rate, --set, --std, --from/--to, --at, --with, --brief, --dry, --json, --gate, --jobs), `--batch FILE`, `ask list` |
        | raea-app-ask | apps/ask/src/runner.rs  | 280   | the batch planned (grouped by file, window and media), each clip cut once, calls in parallel (4 at once unless --jobs), at most 64 questions each, answers streamed as they come; a pool spent midway keeps what came and reports the rest as waiting |
        | raea-app-ask | apps/ask/src/record.rs  | 160   | every answer in ~/.raea/ask/answers.jsonl (question and version, media hash, window, project, item, answer, cost, route); the cut media kept by hash in ~/.raea/ask/media, at most 2 GB, oldest out first |
      When the person or an agent runs `raea ask out/piece.mp4 "Does the music fight the voice?"`
      Then it answers in words with the probability, the window it looked at, what it cost and the pool left, and says
        it's advice until the question is calibrated
      And `raea ask --batch asks.json` (or JSON lines on stdin; each entry a file, a window, and questions, a set or
        standard ids) cuts each clip once and asks everything in parallel, and `raea check`, the critique and the
        fingerprint call the same runner
      And `--json` is a stable schema for pipelines, and `--gate` exits 1 when a calibrated question fails (`--gate any`:
        when any answer passes its threshold)
      And sdk/raea's `raea/ask` gives scripts `ask` and `askMany`, typed from the generated questions, over
        `raea ask --batch --json`, from a `.ts` check, a `raea run` script or plain Node
      And docs/manual/reference/cli/ask.md is its reference, written in the same change

  Rule: AK4. On the person's plan

    @needs:AK3
    Scenario: AK4. The pool, through the app or an agent key
      Then raea-api's `/v1/clef/ask` takes sounds (up to 4 MP3s) beside stills and videos, says the pool left with
        each answer, and agent keys gain an `ask` scope that asks and can do nothing else
      And the app's host answers `/host/ask` by adding its device token and passing the call on, or 409 when the
        account has no pool, so the CLI goes on to the next route
      And the app's critique and game check ask through `raea ask --json`, and their own clef code is gone
      And the app's agents get an `ask` tool (HOST) with `raea ask`'s inputs

  Rule: AK5. Standing questions in `raea check`

    @needs:AK3
    Scenario: AK5. A taste line that asks
      Then a taste line with `check: "ask"` is asked in its own words ("The music never fights the voice."), and
        `check: "ask:ID"` asks a standard question; its `values` can set the type, options, sees, hears and window
      And `raea check PROJECT` asks every such line of a composition in one batch on its newest render, waits without
        a render, says what it cost, and asks nothing again on an unchanged render
      And an answer is a finding at the line's level once its question is calibrated, and a note until then
      And `--no-ask` leaves them out

  Rule: AK6-AK7. Better with use

    @needs:AK3
    Scenario: AK6. Labels
      Given the plan
        | crate        | file                    | lines | job                                                                    |
        | raea-app-ask | apps/ask/src/labels.rs  | 220   | an item's notes matched to the questions' label words (a match is yes, an approval with none is no); `raea ask verdict ANSWER yes|no`; an agent's own look kept as another eye |
      Then the person's labels count toward calibration, and an agent's are kept as `eye:agent` and never counted

    @needs:AK6
    Scenario: AK7. Calibrate, and try new words
      Given the plan
        | crate        | file                      | lines | job                                                                    |
        | raea-app-ask | apps/ask/src/calibrate.rs | 240   | per question version: its AUC and threshold at 10% false positives from the person's labels, through the fingerprint's gates; `ask try "new words" --like ID` replayed on the kept media |
      Then a question with 16 labels of each kind from 3 projects and a held-out AUC of 0.8 or more is calibrated, at
        its threshold
      And one under 0.6 after 32 labels says it doesn't separate, with its misses, so its words can be fixed
      And `raea ask try` asks the new words on the labelled answers' kept media and prints both AUCs, and a standard
        question's words change only with a better AUC on raea's bench
      And a standard question ships with raea's calibration from that bench, and the person's own replaces it once it
        passes the gates

  Rule: AK8. The fingerprint asks too

    @needs:AK3
    Scenario: AK8. A double check on the fingerprint's flags
      Given the plan
        | crate                | file                     | lines | job                                                                    |
        | raea-app-ask         | apps/ask/src/double.rs   | 160   | a score's flagged moments, each asked in its axis's own words on its own window; the answers beside the flags and kept as evidence |
        | raea-app-fingerprint | apps/fingerprint/src/verbs.rs | 15 | `score --ask` and fingerprint.ts's `ask` hand the flagged moments to raea-app-ask |
      Then `raea fingerprint score FILE --ask` (or `ask: "flags"` in fingerprint.ts) asks each flagged moment, and
        `ask: "unsure"` also each moment between an axis's plausible and strong bands
      And each flag says "clef agrees (0.71)" or "clef doesn't see it (0.08)", and each answer is an `eye:clef`
        evidence row that trains the head and never counts as the person's
      And it costs by the flags, never the whole piece
      And the app's rule_check asks the rules nothing measures, and the fingerprint with `--ask unsure`, before it asks
        the agent for its own verdicts, so the agent opens only the frames clef flags

  Rule: AK9. Handing a question to a local head

    @needs:AK7,AK8
    Scenario: AK9. A measurement: can a calibrated question be learned locally
      Given one calibrated question with its kept answers and the person's labels
      When a fingerprint head is trained on its windows' SigLIP or CLAP columns from clef's answers
      Then its held-out AUC on the person's labels is set beside clef's
      And only if it matches does `raea check` ask the local head first and clef only in its unsure band
