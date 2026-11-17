#!/bin/sh
# Stable Audio Open Small (stabilityai/stable-audio-open-small, Stability AI Community License) on Core ML on a Mac (the
# packages are mlboydaisuke/Stable-Audio-Open-Small-CoreML), else on ONNX Runtime (the same weights as three fp32 graphs,
# atauser001/stable-audio-open-small-onnx: 2 GB, with onnxruntime-gpu and its CUDA libraries where nvidia-smi finds a
# GPU, else onnxruntime; RAEA_SAO_BACKEND=coreml|onnx picks) for raea-render's asset store: `make("sao", prompt, seed, {seconds})`
# renders one sound locally (run.py, copied in beside the models). Installs into $RAEA_SAO (default ~/.cache/raea-sao),
# outside the repo, as TokenSynth's lives in ~/.cache/raea-tokensynth. raea-render runs this itself the first time a sound is
# asked for and nothing is installed; `raea setup-audio` runs it on purpose.
#
# What it fetches, once: the DiT (1.36 GB), the VAE decoder (156 MB) and the number embedder (0.4 MB) at a pinned
# revision, each checked against its SHA-256; t5-base's tokenizer.json (1.4 MB), so runs never touch the network; and
# a small venv (numpy, coremltools, tokenizers: ~95 MB). The published fp16 T5 encoder overflows, so the T5 is raea's
# own fp32 conversion (t5_to_coreml.py, 419 MB): made here once in a throwaway environment with torch and t5-base's
# weights (892 MB, deleted after), unless another install has it to copy.
#
# $RAEA_SAO_FROM names an existing install to copy models from instead of downloading them (default ~/.cache/raea-sao
# when installing somewhere else). Copies are APFS clones (cp -c): no extra disk until one side changes.
#   tools/sao/setup.sh
set -e
DIR=${RAEA_SAO:-$HOME/.cache/raea-sao}
HERE=$(cd "$(dirname "$0")" && pwd)
FROM=${RAEA_SAO_FROM:-$HOME/.cache/raea-sao}
REPO=mlboydaisuke/Stable-Audio-Open-Small-CoreML
REV=8fa80903adebfeb62fc885c45505ac81628f3320
T5REV=a9723ea7f1b39c1eae772870f3b547bf6ef7e6c1
ONNXREPO=atauser001/stable-audio-open-small-onnx
ONNXREV=8f9544df5933b9ed4a7e8ed7fa6f77b3dd48ddce
BACKEND=${RAEA_SAO_BACKEND:-}
if [ -z "$BACKEND" ]; then
  if [ "$(uname)" = Darwin ]; then BACKEND=coreml; else BACKEND=onnx; fi
fi
mkdir -p "$DIR"
DIR=$(cd "$DIR" && pwd)
[ "$(cd "$FROM" 2>/dev/null && pwd)" = "$DIR" ] && FROM=
# one setup at a time: a render and a CLI asking for the first sound together would both start one
if ! mkdir "$DIR/.setup-lock" 2>/dev/null; then
  echo "sao setup: another setup is running in $DIR (remove $DIR/.setup-lock if it isn't)" >&2
  exit 75
fi
trap 'rm -rf "$DIR/.setup-lock"' EXIT
trap 'exit 130' INT TERM
cd "$DIR"
say() { echo "sao setup: $*" >&2; }
# a file's SHA-256 (sha256sum on Linux, shasum on a Mac)
sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# a file of the installation: kept if it's here, cloned from $FROM if that has it, else downloaded; checked against
# its SHA-256 when one is given. fetch DEST URL [SHA256]
fetch() {
  dest=$1 url=$2 sum=$3
  [ -s "$dest" ] && return 0
  mkdir -p "$(dirname "$dest")"
  if [ -n "$FROM" ] && [ -s "$FROM/$dest" ]; then
    cp -c "$FROM/$dest" "$dest.part" 2>/dev/null || cp "$FROM/$dest" "$dest.part"
  else
    say "downloading $dest"
    curl -fL --retry 3 --progress-bar -o "$dest.part" "$url"
  fi
  if [ -n "$sum" ] && [ "$(sha256 "$dest.part")" != "$sum" ]; then
    rm -f "$dest.part"
    say "$dest doesn't match its SHA-256"
    exit 1
  fi
  mv -f "$dest.part" "$dest"
}

# the venv: only what run.py imports. An existing one that already has them is left alone
if [ "$BACKEND" = onnx ]; then
  runtime="onnxruntime==1.23.2"
  # an NVIDIA GPU: the CUDA build, with CUDA's and cuDNN's libraries as wheels (run.py preloads them)
  command -v nvidia-smi >/dev/null 2>&1 && nvidia-smi -L >/dev/null 2>&1 && runtime="onnxruntime-gpu[cuda,cudnn]==1.23.2"
  [ -n "${RAEA_SAO_CUDA:-}" ] && runtime="onnxruntime-gpu[cuda,cudnn]==1.23.2"
  if ! .venv/bin/python -c "import numpy, onnxruntime, tokenizers" 2>/dev/null; then
    say "making the venv ($runtime)"
    [ -x .venv/bin/python ] || uv venv -q --python 3.11 .venv
    VIRTUAL_ENV="$DIR/.venv" uv pip install -q --python .venv/bin/python "numpy==2.4.6" "tokenizers==0.23.3" "$runtime"
  fi
elif ! .venv/bin/python -c "import numpy, coremltools, tokenizers" 2>/dev/null; then
  say "making the venv"
  [ -x .venv/bin/python ] || uv venv -q --python 3.11 .venv
  VIRTUAL_ENV="$DIR/.venv" uv pip install -q --python .venv/bin/python "numpy==2.4.6" "coremltools==9.0" "tokenizers==0.23.3"
fi

# the tokenizer from the install, never the Hugging Face cache (the spike's runner read ~/.cache/huggingface)
fetch t5-tokenizer/tokenizer.json "https://huggingface.co/google-t5/t5-base/resolve/$T5REV/tokenizer.json" \
  d2acde0d8d71dd30a711834b07781b9c89feaac33fd332f60507699282740066

if [ "$BACKEND" = onnx ]; then
  # the graphs at a pinned revision, each against the SHA-256 its model card publishes; its licence and NOTICE (what
  # the conversion changed) beside them
  O=https://huggingface.co/$ONNXREPO/resolve/$ONNXREV
  fetch onnx/cond.onnx "$O/cond.onnx" 847822809975d0c36a665594fad0312c4c225147910657820e9936b09014629c
  fetch onnx/dit.onnx "$O/dit.onnx" 8e9c8b14baac4a26159d5ecf3d365ef5a097f321541f9548d62d136354a23d8b
  fetch onnx/dec.onnx "$O/dec.onnx" 96029859b4d563f3aff266e97e715190e8d53278c6e27ab94095535a0da4c911
  fetch onnx/LICENSE "$O/LICENSE"
  fetch onnx/NOTICE "$O/NOTICE"
  cp "$HERE/run.py" "$DIR/run.py"
  date -u +%Y-%m-%dT%H:%M:%SZ > "$DIR/.ready"
  du -sh "$DIR/onnx" "$DIR/.venv" >&2
  say "ready in $DIR (ONNX)"
  exit 0
fi

HF=https://huggingface.co/$REPO/resolve/$REV
P=Data/com.apple.CoreML
while read -r name mlmodel weights; do
  fetch "coreml-ref/StableAudio$name.mlpackage/Manifest.json" "$HF/StableAudio$name.mlpackage/Manifest.json"
  fetch "coreml-ref/StableAudio$name.mlpackage/$P/model.mlmodel" "$HF/StableAudio$name.mlpackage/$P/model.mlmodel" "$mlmodel"
  fetch "coreml-ref/StableAudio$name.mlpackage/$P/weights/weight.bin" "$HF/StableAudio$name.mlpackage/$P/weights/weight.bin" "$weights"
done <<EOF
NumberEmbedder 14ecb9a0f6c3d04c3a611352343d7dea0c5b0720c9f8b56d87b8eae794b9c64c 8ae6ab6d7bf960678564163be68ad300e245030b56eb5db10b2ab3e9090220d8
VAEDecoder 9614b1ec8ba90995c1b8e797da19ba21fc4acc67b0dfe0bbb28527dcbfc0a262 a8a1bdb84b854f38e2ef1d3adb5860a03303a442f8eaa8d567aeb606e66a4fc8
DiT a118772c17fb831a6eda6a1ec2527f535d004064e27a8eb4597d5ae6d33a0576 c5444973f1a2bb3878a33049a4c559a1097de77fca8dc358aba38ee72033e97d
EOF

T5=coreml/T5EncoderFP32.mlpackage
# made under another name until it's whole (coremltools insists on the .mlpackage extension)
PART=coreml/T5EncoderFP32.part.mlpackage
if [ ! -s "$T5/Manifest.json" ]; then
  mkdir -p coreml
  rm -rf "$PART"
  if [ -n "$FROM" ] && [ -s "$FROM/$T5/Manifest.json" ]; then
    cp -cR "$FROM/$T5" "$PART" 2>/dev/null || cp -R "$FROM/$T5" "$PART"
  else
    # torch, transformers and t5-base's weights only for this one conversion, in uv's throwaway environment and a
    # Hugging Face cache that's deleted after (on failure too, by the trap): none of it stays in the install
    say "converting t5-base's encoder to Core ML at fp32 (a one-time ~1 GB download, deleted after)"
    trap 'rm -rf "$DIR/.setup-lock" "$DIR/.t5-hf"' EXIT
    HF_HOME="$DIR/.t5-hf" nice -n 15 uv run -q --no-project --python 3.11 --with "torch==2.7.1" --with "transformers==5.19.0" \
      --with "coremltools==9.0" --with "numpy==2.4.6" python -I "$HERE/t5_to_coreml.py" --revision "$T5REV" \
      --tokenizer t5-tokenizer/tokenizer.json --out "$PART"
    rm -rf .t5-hf
  fi
  mv "$PART" "$T5"
fi

cp "$HERE/run.py" "$DIR/run.py"
date -u +%Y-%m-%dT%H:%M:%SZ > "$DIR/.ready"
du -sh "$DIR/coreml-ref" "$DIR/coreml" "$DIR/.venv" >&2
say "ready in $DIR"
