@spec
Feature: Sound for picture: the local models on any Mac, open libraries and packs one command away, music composed
  Asked for on 2026-11-16: rewrite the music and sound notes so that SAO and TokenSynth can be downloaded, every open
  instrument library and sound-effect pack raea knows is easy to reach, and music is never a default: "it tends to
  result in drones and pads and not actually design real complex music, scoring, sound effects, diegetic and
  non-diegetic."

  Where it stands (2026-11-16): `raea setup-audio` installs both models through tools/sao/setup.sh and
  tools/tokensynth/setup.sh, and `raea sounds` reaches four libraries (sdk/raea/library/sounds/catalog.json:
  salamander, fluid, tonejs-instruments, vcsl). The guides teach deciding what each scene is heard as, spotting the
  cut and composing for it (sdk/guides/craft/motion/music.md, sound.md), and list every vetted source with its licence
  and how to fetch it (motion/sources.md). What those guides had to write down as a gap is below.

  Rule: The local models install on any Mac, and say what they are

    Scenario: SN1. setup-audio works on a clean Mac
      Given a Mac with no uv, no git and no Python of its own
      When `raea setup-audio` runs
      Then each setup gets uv and git through tools/ensure.sh, which every release ships, instead of the PATH
      And TokenSynth's code is cloned at a pinned commit and its weights fetched at a pinned revision, each file
        checked against its SHA-256, as SAO's already are
      And a missing tool, a full disk or a failed download is named in the error with the one command that fixes it
      And it is proven from a release build on a fresh macOS user account, with the sizes it reports

    Scenario: SN2. A model's licence is shown and kept
      Given Stable Audio Open Small is under Stability AI's Community License, which isn't an open licence
      When setup-audio installs a model
      Then it prints the model's licence and where to read it, and writes it beside the model as LICENCE.txt
      And `raea setup-audio --check` names each model's licence
      And the person decides once whether SAO's sounds go into their commercial pieces, before the first one does

    Scenario: SN3. The app offers the local models before a piece needs them
      Given the raea app on a Mac without the models
      When a piece first needs SAO or TokenSynth
      Then the app says what will be downloaded (its size, licence and folder) and installs it on a yes, with progress
      And a render never blocks on a download of gigabytes it didn't say it would make

  Rule: Open libraries and packs are one command away

    Scenario: SN4. raea sounds reaches the vetted packs
      Given the packs motion/sources.md lists as fetched by hand
      When `raea sounds get` names one, an instrument of it or a sound in it (VSCO 2 CE's celli, Kenney's impacts,
        Groove MIDI's funk grooves)
      Then it downloads only that, once, into ~/.cache/raea-sounds, checked against a pinned SHA-256
      And it copies what the piece uses into media/ with its LICENCE.txt and the credit line it needs
      And it converts what raea can't read (Ogg, Opus) to WAV on the way in
      And a pack whose licence forbids passing it on (Sonniss's bundles, Pixabay's sounds) is never mirrored: raea
        links to it and indexes the person's own download
      And nothing non-commercial, no-derivatives, Sampling+ or share-alike is in the catalogue
      And Freesound is searched from raea only under a commercial licence for its API from Freesound's owners

    Scenario: SN5. A sampled instrument keeps its dynamics and round robins
      Given a library with several velocity layers and round robins a note (VCSL, VSCO 2 CE, Salamander V3)
      When `raea sounds get` fetches it, or a composition loads its SFZ map
      Then every layer comes, and one Sampler plays the layer each note's velocity asks for and turns through the
        round robins
      And `humanize` on a Part, a Sequence or a Loop moves each note by a seeded few milliseconds, the same every render

    Scenario: SN6. A MIDI file plays on sampled instruments
      Given a MIDI file: a Groove MIDI drum performance, or a public-domain score exported from MuseScore
      When a composition imports it (`import groove from "./media/grooves/funk-92.mid"`)
      Then it gets the file's tracks and notes in raea/music's MidiFile shape, to play on Samplers and Players
      And a General MIDI drum track maps onto a kit by note number
      And a SoundFont bank (.sf2, .sf3) can play it in raea, without a render outside it

  Rule: Music is composed for the cut, and a drone is caught

    Scenario: SN7. raea check says when a cue is a drone
      Given a render whose music bus holds a cue longer than 15 s
      When `raea check` measures it
      Then it reports, by timecode, a cue whose lead has fewer than 5 distinct pitches in 8 bars, whose chord holds
        longer than 2 bars, with fewer than 2 note onsets a beat across its parts, a velocity spread under 8 (of
        127), one timbre throughout, or an ending that fades rather than landing on a cut
      And its thresholds are set from approved scores and known drones before it ranks anything above P3
