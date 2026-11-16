# Sources: the local models, sample libraries, effect packs and MIDI, and how to get each onto the Mac

## What to do

- Take each sound from the source that fits its job, and know its licence before it goes in:

  | the job | the source |
  |---|---|
  | an instrument playing a part: piano, strings, brass, woodwinds, mallets, guitars, a drum kit | a sampled library: `raea sounds` first, then the packs below |
  | a timbre no library has ("a bowed saw, close") | TokenSynth, curated (`motion/music`) |
  | an effect, foley, an ambience | a recorded pack or the person's own recordings first; SAO for what none has, in layers (`motion/sound`) |
  | a voice over the picture | `<Speech>` (Pocket TTS, local); a voice model is a `raea gen speech` step, Fish's with the person's own FISH_API_KEY (`motion/gen`) |
  | a drummer's feel | a Groove MIDI performance played on a sampled kit |
  | a melody or harmony to arrange from the public domain | Mutopia, OpenScore |

- Use what raea's licence rule takes: CC0 and public domain, CC-BY (credited), MIT and Apache, and free licences that
  allow a commercial film (Sonniss's, Pixabay's). Never a non-commercial (NC), no-derivatives (ND), Sampling+,
  "personal use" or share-alike (SA) source: share-alike terms can reach the film made with it. A pack's licence is
  per pack and sometimes per sound; read its readme and keep it beside the files.
- Credit each CC-BY source where the piece is shown (its description or end credits). The line is in the
  `LICENCE.txt` that `raea sounds get` writes, or in the pack's readme.
- Keep a pack fetched by hand in one folder on the Mac, outside any project (`~/Music/raea-sounds/PACK/` with its
  licence file), and copy the files a piece uses into its `media/` (`media/sounds/PACK/`): a render reads only its
  own media, never a URL. With the raea app open, `raea library new sounds` and `raea library add sounds
  ~/Music/raea-sounds` index that folder once the person says yes in the app; then `raea library search "metal door
  slam" --kind audio --sound` finds a sound by what it sounds like.
- raea reads WAV, MP3, FLAC and AAC (.m4a). Convert Ogg (most of Kenney's packs) and Opus to WAV first (`ffmpeg -i
  in.ogg out.wav`).

## Getting SAO and TokenSynth onto the Mac

Both run on this computer and install themselves the first time a render, `raea sao` or `raea tokensynth` asks for
a sound. To do it ahead of time and see where they are:

```sh
raea setup-audio --check   # what's installed, where, and how much disk
raea setup-audio           # installs what's missing: TokenSynth, then SAO
raea setup-audio sao       # one of them
```

- **What it fetches.** Each model has a script (`tools/sao/setup.sh` and `tools/tokensynth/setup.sh` in a checkout,
  `share/raea/tools/` beside an installed raea) that installs into `~/.cache/raea-sao` and `~/.cache/raea-tokensynth`
  (RAEA_SAO and RAEA_TOKENSYNTH move them):
  - SAO, about 2 GB: Stable Audio Open Small's Core ML conversion (the DiT, the VAE decoder and the number embedder)
    from a Hugging Face mirror at a pinned revision, each file checked against its SHA-256; t5-base's tokenizer; a
    small Python environment; and raea's own fp32 T5 encoder, converted once in a throwaway environment whose torch
    and t5-base weights (about 900 MB) are deleted after.
  - TokenSynth, about 6 GB: its code from GitHub, a Python environment with torch (about 1.1 GB), the model and its
    unconditional twin (713 MB each), DAC (307 MB) and CLAP (2.35 GB) from Hugging Face, the text encoders CLAP
    fetches, then a converted fp16 runtime and Core ML models.
- **What it needs.** A network connection, `uv` (Astral's Python manager) on the PATH, and `git` for TokenSynth.
  Neither script installs them. Install uv first (`curl -LsSf https://astral.sh/uv/install.sh | sh`, or Homebrew).
- **The licences.** TokenSynth's code and weights are MIT. Stable Audio Open Small is under Stability AI's Community
  License, which isn't an open licence: it's free to use, commercially too, for a person or organisation with under
  USD 1 million in annual revenue (above that, Stability's enterprise licence), Stability asks commercial users to
  register (stability.ai/license), and what it makes belongs to the person. Setup doesn't show this licence, so say
  it to the person before the first SAO sound goes into a commercial piece of theirs.
- **When it fails.** The error names the script and the folder; run that script with `sh` to see its own output.
  `exit status: 127` is uv or git missing. `75` is another setup running in the same folder: wait for it, or remove
  the folder's `.setup-lock` once nothing is running. RAEA_OFFLINE refuses the download, and RAEA_AUDIO_AUTOSETUP=0
  turns the self-install off, so a render names the command instead. Running setup-audio again picks up where a
  broken install stopped: finished files are kept.
- A render that meets a model not yet installed waits while it installs (minutes, once). Run setup-audio before the
  first render, not inside one.

## What `raea sounds` fetches for you

`raea sounds WORDS` searches four libraries with no network (`kick`, `hihat` and the other words producers use find
their parts). `raea sounds get PACK/INSTRUMENT` downloads one instrument's files once (into `~/.cache/raea-sounds`),
copies them into `media/sounds/PACK-INSTRUMENT/` with a `LICENCE.txt`, and prints the `Sampler` (or the `Player`s)
that plays them.

| pack | licence | what it has |
|---|---|---|
| `salamander` | CC-BY 3.0 (credit) | a grand piano, sampled every minor third |
| `fluid` | CC-BY 3.0 (credit) | the 128 General MIDI instruments of the FluidR3 font, every note A0 to C8; no drum kit |
| `tonejs-instruments` | public domain and CC-BY, per instrument (its `LICENCE.txt` says) | 20: cello, violin, contrabass, French horn, trumpet, trombone, tuba, saxophone, flute, clarinet, bassoon, harp, guitars, organ, harmonium, piano, xylophone |
| `vcsl` | CC0 | 104 from the Versilian Community Sample Library: pianos, harpsichords, harps, organs, recorders, marimba, vibraphone, glockenspiel, kalimbas; bass drums, snares, toms, hi-hat, cymbals, claps, shakers, congas, timpani; a siren, a ratchet, an anvil and other effects |

The ids are exact: `raea sounds kick` lists `vcsl/bass-drum-1`, and `vcsl/bass-drum` isn't one. A pitched
instrument comes as one take a note, at its middle dynamic; a drum or a hit as its first 12 takes (`--all` for
every one). VCSL names a hit's dynamic and round robin in the file (`_v2` soft to `_v7` hard, `_rr1`, `_rr2`), so a
part can pick a softer take for a softer note.

## Packs to fetch by hand

Each of these is a download from its own page into `~/Music/raea-sounds/PACK/`, then the files a piece uses copied
into its `media/`.

| source | licence | where | what it has |
|---|---|---|---|
| VSCO 2 Community Edition | CC0 | github.com/sgossner/VSCO-2-CE (releases) | an orchestra: solo and section strings with articulations, woodwinds, brass, percussion, keys; WAV with SFZ maps, about 3 GB |
| VCSL, the whole library | CC0 | github.com/sgossner/VCSL | every dynamic and round robin of what `raea sounds` gives one take of, with SFZ maps |
| Salamander Grand Piano V3 | CC-BY 3.0 | github.com/sfzinstruments/SalamanderGrandPiano | the same piano at 16 velocities, 48 kHz, SFZ |
| sfzinstruments' lists | per pack, on the list | sfzinstruments.github.io (pianos, strings, drums) | take the CC0, public domain and CC-BY ones: Karoryfer's free packs (Big Rusty, Swirly and Unruly drums, Frankensnare, Meatbass, Bigcat Cello; mostly CC0, older ones CC-BY 3.0: read each readme), SM Drums (public domain), Splendid Grand Piano (public domain), DrumGizmo's DRS and Muldjord kits and Naked Drums (CC-BY 4.0) |
| University of Iowa Musical Instrument Samples | "may be downloaded and used for any projects, without restrictions" | theremin.music.uiowa.edu/mis.html | single notes of orchestral instruments at several dynamics, recorded dry: give them a room's reverb |
| FreePats | per bank: CC0, CC-BY, or GPL with an exception for music made with it | freepats.zenvoid.org | General MIDI families, world instruments, percussion; SFZ and SF2 |
| MuseScore General | MIT (keep its notice) | ftp.osuosl.org/pub/musescore/soundfont/MuseScore_General/ | a General MIDI bank with drum kits, SF2 (206 MB) or SF3, for playing a MIDI file as it is |
| Kenney's audio packs | CC0 | kenney.nl/assets/category:Audio | Impact Sounds, Interface Sounds, UI Audio, Sci-fi Sounds, RPG Audio, Casino Audio, Digital Audio, Music Jingles, voice-over packs; mostly Ogg |
| OpenGameArt | per asset: take CC0, CC-BY and OGA-BY | opengameart.org | game effects and loops, such as "512 Sound Effects (8-bit style)" (CC0) |
| Freesound | per sound: take CC0 and CC-BY only | freesound.org (a free account to download) | field recordings, foley and effects of every kind |
| Sonniss #GameAudioGDC bundles | royalty-free for commercial use, no credit; never passed on as sounds (a library, a pack, a template) and never used to train a model | sonniss.com/gameaudiogdc | over 200 GB of professional foley, impacts, ambiences and field recordings, a bundle a year |
| Pixabay sound effects | Pixabay's Content License: free, commercial use, no credit; never sold or shared on their own | pixabay.com/sound-effects | effects and ambiences |
| Groove MIDI Dataset | CC-BY 4.0 (credit Google Magenta) | magenta.tensorflow.org/datasets/groove | 1,150 MIDI performances by 10 drummers: 13.6 hours of beats and fills in 18 styles, 3 MB |
| Mutopia Project | per piece: public domain or CC-BY (skip the BY-SA ones) | mutopiaproject.org | 2,100 classical pieces as MIDI and LilyPond |
| OpenScore Lieder | CC0 | github.com/OpenScore/Lieder | 1,350 art songs as MuseScore scores (MuseScore exports MIDI) |

Not these, whatever they sound like: Sonatina Symphonic Orchestra and the Virtual Playing Orchestra (Sampling+ and
mixed terms: no use in an ad), the Philharmonia's samples (terms unclear), Spitfire LABS (no samples to take), the
MusyngKite and FatBoy fonts beside FluidR3 (no licence), Tone.js's own drum kits (CC BY-NC-SA), the BBC's sound
effects (personal and research use), Zapsplat and Mixkit (their own terms), Salamander Drumkit and other CC-BY-SA
kits, and Lakh MIDI (copyrighted songs). Freesound's API is free only for non-commercial use, so raea doesn't search
Freesound for the person: they download its sounds themselves.

## Loading a pack in a composition

- **One-shots** (an effect pack, a field recording): the file in `media/sfx/PACK/`, then `<Audio
  src={staticFile("sfx/PACK/door-slam.wav")} />` in a Sequence at its frame (`motion/sound`), or a `Tone.Player`
  for a game's one-shot.
- **A multisampled instrument** (the SFZ packs): raea has no SFZ reader yet. The `.sfz` file is text: each `<region>`
  names a `sample=` file, its `pitch_keycenter` (the note it is), `lovel`/`hivel` (the velocities it plays) and
  `seq_position` (its round robin). Copy the files for the notes and dynamics the part needs, map them by note, and
  give each dynamic its own Sampler, picked by the note's velocity:

  ```tsx
  // SOFT and LOUD: { C3: staticFile("sounds/vsco-cello/..."), ... }, written from the .sfz's regions
  const soft = new Tone.Sampler({ urls: SOFT, release: 0.6 }).connect(Tone.buses.music);
  const loud = new Tone.Sampler({ urls: LOUD, release: 0.6 }).connect(Tone.buses.music);
  const cello = (note: string, dur: string, t: number, vel: number) => (vel < 0.6 ? soft : loud).triggerAttackRelease(note, dur, t, vel);
  ```

- **A MIDI file** (a Groove MIDI performance, a Mutopia piece): a composition can't read a `.mid` yet. Turn it into
  a JSON of notes, then play the notes on Samplers and Players:

  ```sh
  uv run --with mido python3 -I -c '
  import json, sys, mido
  t, notes = 0.0, []
  for m in mido.MidiFile(sys.argv[1]):
      t += m.time
      if m.type == "note_on" and m.velocity:
          notes.append({"time": round(t, 4), "midi": m.note, "vel": round(m.velocity / 127, 3)})
  json.dump(notes, open(sys.argv[2], "w"))
  ' media/grooves/funk-92.mid media/grooves/funk-92.json
  ```

  ```tsx
  import groove from "./media/grooves/funk-92.json"; // [{ time, midi, vel }]: seconds, at the drummer's 92 bpm
  // General MIDI drum notes: 36 kick, 38 and 40 snare, 42 and 44 closed hi-hat, 46 open (the dataset lists the rest)
  const KIT: Record<number, string> = { 36: "kick", 38: "snare", 40: "snare", 42: "hat", 44: "hat", 46: "open" };
  const stretch = 92 / Tone.getTransport().bpm.value; // the drummer's tempo onto the cue's
  new Tone.Part((t, n) => kit.player(KIT[n.midi]).start(t), groove.filter((n) => n.midi in KIT).map((n) => ({ ...n, time: n.time * stretch }))).start("4m");
  ```

  A pitched track's notes go to a Sampler as `Tone.Frequency(n.midi, "midi").toNote()`.
- **A SoundFont** (`.sf2`, `.sf3`): raea can't play one. To hear a MIDI file the person hands over as it is, render
  it with FluidSynth (`brew install fluid-synth`, then `fluidsynth -ni -F song.wav -r 48000 MuseScore_General.sf2
  song.mid`) and play the WAV as `<Audio bus="music">`; to write parts, use the sampled libraries instead.
