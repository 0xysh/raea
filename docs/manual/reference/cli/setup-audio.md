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
  Its licence is Stability AI's Community License
  ([stability.ai/community-license-agreement](https://stability.ai/community-license-agreement)), not an open one.
  As between you and Stability, you own the sounds it makes, as far as the law allows; they may not be used to
  create or improve another generative AI model, and Stability's acceptable use policy applies to them. What else
  the licence asks, of raea and of a commercial user, raea hasn't written down yet: read it before an SAO sound goes
  into a commercial piece.
- **TokenSynth, about 6 GB:** its code from GitHub, a Python environment with torch, and its weights, DAC and CLAP
  from Hugging Face, then a converted fp16 runtime. MIT.

Both need a network connection and [uv](https://docs.astral.sh/uv/) on the PATH; TokenSynth also needs `git`. On a
Mac without them, `share/raea/tools/ensure.sh uv` and `ensure.sh git` install raea's pinned builds (uv 0.12.24 and
git 2.56.0, each download checked against its SHA-256) and print where; put those folders on the PATH:

```sh
B=$(dirname "$(realpath "$(command -v raea)")"); T=$B/../share/raea/tools; [ -d "$T" ] || T=$B/../../tools  # an install, or a checkout
PATH="$(dirname "$(bash "$T/ensure.sh" uv)"):$(dirname "$(bash "$T/ensure.sh" git)"):$PATH" raea setup-audio
```

When a setup fails, the error names its script: run that with `sh` to see why. Exit status 127 is uv not found; 1
with `xcrun: error: invalid active developer path` is macOS's stand-in git on a Mac without the Command Line Tools
(use the git `ensure.sh` installs); 75 is another setup running in the same folder. `RAEA_OFFLINE` refuses the
download. `RAEA_AUDIO_AUTOSETUP=0` stops a render from installing a model itself, and stops `raea setup-audio` and
`ensure.sh` too: with it set, run the model's script yourself (`sh share/raea/tools/sao/setup.sh`).

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
