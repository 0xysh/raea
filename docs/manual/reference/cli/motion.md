---
title: "raea motion"
summary: "Body motion made on this Mac by Kimodo (NVIDIA, on MLX): a SOMA clip from a line of words and, with --keys, Kimodo constraints (full-body keyframes, hand or foot targets, a root path, as sdk/body/tools/between.ts writes them), as the clip sdk/body/tools/fromkimodo.ts reads."
order: 105
sources: ["engine/cli/src/cli.rs"]
---

Body motion made on this Mac by Kimodo (NVIDIA, on MLX): a SOMA clip from a line of words and, with --keys, Kimodo constraints (full-body keyframes, hand or foot targets, a root path, as sdk/body/tools/between.ts writes them), as the clip sdk/body/tools/fromkimodo.ts reads.

<!-- generated:begin by tools/manual-ref/cli.ts from `raea help --json`; edit only the flag descriptions -->

## Usage

```sh
raea motion "LINE" [--keys KEYS.json] [--seconds 5 | --frames N] [--seed 1] [--steps 100] [-o CLIP.json]   (body motion made on this Mac by Kimodo (NVIDIA, on MLX): a SOMA clip from a line of words and, with --keys, Kimodo constraints (full-body keyframes, hand or foot targets, a root path, as sdk/body/tools/between.ts writes them), as the clip sdk/body/tools/fromkimodo.ts reads; the first use fetches the motion model (1.1 GB) into raea's models folder, and the first new line its text encoder (8.8 GB: Llama 3 8B in 8 bits with LLM2Vec's adapters); a line's words are read once and kept. Licences: Kimodo's weights NVIDIA's Open Model License, Meta Llama 3 the Meta Llama 3 Community License, LLM2Vec's adapters MIT)
```

Headless: it needs nothing running and opens no window. Takes up to 1 argument besides its flags.

## Flags

| Flag | Takes | What it does |
| --- | --- | --- |
| `--frames` | `N` | The clip's length in frames at 30 a second, 2 to 600. Instead of `--seconds`. |
| `--keys` | `KEYS.json` | Kimodo constraints the motion must meet: full-body keyframes, a hand or foot at a place on a frame, a path for the hips. `bun sdk/body/tools/between.ts` writes them between two poses of the body library. |
| `--seconds` | `5` | The clip's length in seconds (5 unless said). |
| `--seed` | `1` | Which of the motions the line could be: another seed is another take of the same line. A seed here is not that seed on Kimodo's PyTorch code (the noise is drawn differently). |
| `--steps` | `100` | Denoising steps (Kimodo's 100). Fewer is faster and rougher. |
| `-o` | `CLIP.json` | Where the clip goes: `{fps, names, root, turns, contacts, ...}`, Kimodo's 77 SOMA joints as each one's turn in the world from the T-pose. Without it, the line's words name the file in the working folder. |

<!-- generated:end -->

## Example

A stage's `do` line made into a clip: the body stands at the start and the end, and the motion between comes from the
words.

```sh
bun sdk/body/tools/between.ts skeleton.json LIBDIR aMale2_A1_Stand:0 aMale2_A1_Stand:0 stand.json 4 0,0 0
raea motion "A person waves at the camera with their right hand" --keys stand.json --seconds 4 --seed 2 -o wave.clip.json
```

A clip takes about 8 seconds on an M5 Pro. `bun sdk/body/tools/makedo.ts` does the whole of it for a stage's lines:
three seeds a line, each checked against the captures the body hands on to, and the one that does the most kept in the
project's made clips.
