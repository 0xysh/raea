---
title: "raea setup-audio"
summary: "Install or check the local audio models: Stable Audio Open for foley and TokenSynth for instruments."
order: 86
sources: ["apps/tools/src/sound.rs"]
---

`raea sao` and `raea tokensynth` install their models the first time they're used. `raea setup-audio` does it on
purpose, ahead of time, and says where each lives and how much disk it takes. Name one model to set up only that one.

```sh
raea setup-audio
raea setup-audio --check
raea setup-audio tokensynth
```

Each model's script (`share/raea/tools/sao/setup.sh` and `share/raea/tools/tokensynth/setup.sh` beside an installed
raea) installs it into `~/.cache/raea-sao` or `~/.cache/raea-tokensynth` (`RAEA_SAO` and `RAEA_TOKENSYNTH` move them):

- **SAO, about 2 GB:** Stable Audio Open Small's Core ML models from a Hugging Face mirror at a pinned revision, each
  file checked against its SHA-256, t5-base's tokenizer, a small Python environment, and a T5 encoder converted once.
  Its licence is Stability AI's Community License, not an open one: free, commercially too, for a person or
  organisation under USD 1 million in annual revenue; above that, Stability's enterprise licence
  ([stability.ai/license](https://stability.ai/license)). The sounds it makes are yours.
- **TokenSynth, about 6 GB:** its code from GitHub, a Python environment with torch, and its weights, DAC and CLAP
  from Hugging Face, then a converted fp16 runtime. MIT.

Both need a network connection and [uv](https://docs.astral.sh/uv/) on the PATH; TokenSynth also needs `git`. When a
setup fails, the error names its script: run that with `sh` to see why (exit status 127 is uv or git missing, 75 is
another setup running in the same folder). `RAEA_AUDIO_AUTOSETUP=0` stops a render from installing a model itself,
and `RAEA_OFFLINE` refuses the download.

<!-- generated:begin by tools/manual-ref/cli.ts from `raea help --json`; edit only the flag descriptions -->

## Usage

```sh
raea setup-audio [--check] [sao|tokensynth]   (install or check the local audio models)
```

Headless: it needs nothing running and opens no window. Takes up to 1 argument besides its flags.

## Flags

| Flag | Takes | What it does |
| --- | --- | --- |
| `--check` | – | Say whether each model is installed, where it lives and how much disk it takes, without installing anything. |

<!-- generated:end -->
