#!/bin/bash
# raea cloud's agent box on a Fly Sprite (docs/cloud-render.feature CR11, CR12): raea's Linux release, ACP agents and
# sdk/cloud's server on the Sprite's own HTTPS URL, its renders sent to the farm's GPUs (deploy/modal/farm.py). A Sprite
# bills the CPU and memory it actually uses, sleeps when nothing talks to it and keeps its disk, so an agent that mostly
# waits on its model costs little; the GPUs it needs come from the farm and stop when the render does. From sdk/cloud:
#
#   deploy/sprites/box.sh up NAME      the Sprite, made or completed, and its server started
#   deploy/sprites/box.sh status NAME  its URL, raea, the server's health
#   deploy/sprites/box.sh token NAME   its RAEA_CLOUD_TOKEN, for the TypeScript client
#   deploy/sprites/box.sh key NAME     a raea agent key (repos and storage scopes) from stdin, or the clipboard, onto
#                                      the box as RAEA_AGENT_KEY: its agents clone raea projects (raea-repo clone OWNER
#                                      PROJECT DIR, deploy/git/repo.sh), push, and send renders' media by hash (CR22, CR23)
#
# RAEA_VERSION (0.1.6) and AGENTS ("kimi-code claude-acp") pick what goes on; the farm comes from
# ~/.config/raea-cloud/farm.env (RAEA_CLOUD_GPU_URL, RAEA_CLOUD_GPU_TOKEN). The Sprite's URL stays private to the org.
set -euo pipefail

# The org is the one raea owns, written once in ~/.config/raea-cloud/sprites.env (RAEA_SPRITES_ORG=...), never taken
# from the environment, and never one listed in private/cloud/sprites-denied.txt (orgs that belong to other businesses).
# (No apostrophe may go in a ${VAR:?message}: it opens a quote that swallows the lines after it, as it once did this
# guard's, and a box went into the wrong org.)
CONF=${RAEA_SPRITES_ENV:-$HOME/.config/raea-cloud/sprites.env}
ORG=$( [ -f "$CONF" ] && sed -n 's/^RAEA_SPRITES_ORG=//p' "$CONF" | tail -1 || true)
[ -n "$ORG" ] || { echo "box: write RAEA_SPRITES_ORG=<the Sprites org raea owns> in $CONF (sprite org list)"; exit 2; }
DENIED=$(git -C "$(dirname "$0")" rev-parse --show-toplevel 2>/dev/null)/private/cloud/sprites-denied.txt
if [ -f "$DENIED" ] && grep -qxF "$ORG" "$DENIED"; then
  echo "box: $ORG belongs to another business ($DENIED); raea boxes go in an org raea owns"
  exit 2
fi
NAME=${2:-}
[ -n "$NAME" ] || { echo "box: name the Sprite"; exit 2; }
RAEA_VERSION=${RAEA_VERSION:-0.1.6}
AGENTS=${AGENTS:-kimi-code claude-acp}
NODE=24.22.0
HERE=$(cd "$(dirname "$0")/../.." && pwd)
FARM=${RAEA_FARM_ENV:-$HOME/.config/raea-cloud/farm.env}

# the Sprite's login shell warns about line editing on every exec; only the command's own status counts
quiet() { grep --line-buffered -v 'bind: warning' || true; }
x() { sprite exec -o "$ORG" -s "$NAME" -- bash -c "$*" 2>&1 | quiet; }
# a file onto the Sprite over exec's stdin (the CLI's own upload times out on large files)
put() { sprite exec -o "$ORG" -s "$NAME" -- bash -c "mkdir -p \"\$(dirname $2)\" && cat > $2" <"$1" 2>&1 | quiet; }

case "${1:-}" in
up)
  [ -f "$FARM" ] || { echo "box: $FARM names no farm (RAEA_CLOUD_GPU_URL, RAEA_CLOUD_GPU_TOKEN)"; exit 2; }
  sprite list -o "$ORG" | grep -qx "$NAME" || sprite create -o "$ORG" --skip-console --label raea --label raea-cloud "$NAME"
  # raea's runtime libraries, and Mesa's lavapipe: the box draws previews on its CPU, its renders go to the farm
  x "sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq && sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
     ca-certificates curl git git-lfs zstd xz-utils procps ripgrep libasound2t64 libfontconfig1 libvulkan1 mesa-vulkan-drivers vulkan-tools >/dev/null"
  x "node -v 2>/dev/null | grep -q '^v24' || { curl -fsSL https://nodejs.org/dist/v$NODE/node-v$NODE-linux-x64.tar.xz | sudo tar -xJ -C /opt \
     && sudo ln -sf /opt/node-v$NODE-linux-x64/bin/node /opt/node-v$NODE-linux-x64/bin/npm /opt/node-v$NODE-linux-x64/bin/npx /usr/local/bin/; }"
  x "[ \"\$(/opt/raea/bin/raea --version 2>/dev/null)\" = $RAEA_VERSION ] || { set -e; f=raea-$RAEA_VERSION-linux-x86_64.tar.gz; \
     u=https://github.com/shinyobjectz/raea/releases/download/v$RAEA_VERSION; cd /tmp && curl -fsSL -o \$f \$u/\$f && curl -fsSL -o \$f.sha256 \$u/\$f.sha256 \
     && sha256sum -c \$f.sha256 && sudo rm -rf /opt/raea && sudo mkdir -p /opt/raea && sudo tar -xzf \$f -C /opt/raea --strip-components=1 && rm -f \$f \$f.sha256; }"
  # sdk/cloud's server, client and wrapper, as the image has them (image/Dockerfile)
  pack=$(mktemp)
  COPYFILE_DISABLE=1 tar -czf "$pack" --no-mac-metadata -C "$HERE" server src image deploy/git package.json package-lock.json
  put "$pack" /tmp/raea-cloud.tgz
  rm -f "$pack"
  x "sudo mkdir -p /opt/raea-cloud && sudo chown \$(id -u):\$(id -g) /opt/raea-cloud && tar -xzf /tmp/raea-cloud.tgz -C /opt/raea-cloud && rm /tmp/raea-cloud.tgz \
     && cd /opt/raea-cloud && npm ci --omit=dev --no-fund --no-audit --loglevel=error && node server/agents.ts install $AGENTS \
     && chmod +x image/*.sh deploy/git/* && sudo ln -sf /opt/raea-cloud/image/raea.sh /usr/local/bin/raea \
     && sudo ln -sf /opt/raea-cloud/deploy/git/repo.sh /usr/local/bin/raea-repo && git lfs install --skip-repo >/dev/null"
  # the box's commits say which box made them
  x "git config --global user.name 'raea cloud $NAME' && git config --global user.email '$NAME@box.raea.io' && git config --global init.defaultBranch main"
  # the box's own token, made once and kept on its disk; the farm's, from this Mac
  x "[ -s ~/.raea-cloud.env ] || { umask 077; echo RAEA_CLOUD_TOKEN=\$(openssl rand -hex 32) > ~/.raea-cloud.env; }"
  put "$FARM" /tmp/farm.env
  x "umask 077; grep -v '^RAEA_CLOUD_GPU_' ~/.raea-cloud.env > ~/.raea-cloud.env.new; cat /tmp/farm.env >> ~/.raea-cloud.env.new; rm /tmp/farm.env; mv ~/.raea-cloud.env.new ~/.raea-cloud.env"
  for a in $AGENTS; do x "raea skills install --agent $a >/dev/null 2>&1 || echo 'box: raea skills for $a not installed'"; done
  # the server, as the Sprite's HTTP service on 8080: raea on lavapipe (docs/cloud.feature CL13), renders to the farm
  start=$(mktemp)
  cat >"$start" <<'EOF'
#!/bin/bash
set -a
. ~/.raea-cloud.env
set +a
export PORT=8080 RAEA_CLOUD_DEVICE=cpu WGPU_BACKEND=vulkan __EGL_VENDOR_LIBRARY_DIRS=/nonexistent
export VK_DRIVER_FILES=$(ls /usr/share/vulkan/icd.d/lvp_icd*.json | head -1)
mkdir -p ~/work
exec node /opt/raea-cloud/server/main.ts
EOF
  put "$start" /opt/raea-cloud/image/sprite-start.sh
  rm -f "$start"
  x "chmod +x /opt/raea-cloud/image/sprite-start.sh && { sprite-env services list | grep -q '\"raea-cloud\"' \
     && sprite-env services restart raea-cloud >/dev/null || sprite-env services create raea-cloud --cmd /opt/raea-cloud/image/sprite-start.sh --http-port 8080; }"
  "$0" status "$NAME"
  ;;
status)
  sprite url -o "$ORG" -s "$NAME" 2>&1 | quiet
  x "/opt/raea/bin/raea --version; . ~/.raea-cloud.env; curl -fsS -H \"authorization: Bearer \$RAEA_CLOUD_TOKEN\" localhost:8080/v1/health | head -c 600; echo"
  ;;
token) x ". ~/.raea-cloud.env; echo \$RAEA_CLOUD_TOKEN" ;;
key)
  # never echoed: from a pipe, or the clipboard when there's none
  if [ -t 0 ]; then k=$(pbpaste); else k=$(cat); fi
  k=$(printf %s "$k" | tr -d '[:space:]')
  case "$k" in raea_ak_*) ;; *) echo "box: that isn't a raea agent key (raea_ak_...)"; exit 2 ;; esac
  f=$(mktemp)
  (umask 077; printf 'RAEA_AGENT_KEY=%s\n' "$k" >"$f")
  put "$f" /tmp/agent.env
  rm -f "${f:?}"
  x "umask 077; grep -v '^RAEA_AGENT_KEY=' ~/.raea-cloud.env > ~/.raea-cloud.env.new; cat /tmp/agent.env >> ~/.raea-cloud.env.new; rm /tmp/agent.env; mv ~/.raea-cloud.env.new ~/.raea-cloud.env \
     && sprite-env services restart raea-cloud >/dev/null && echo 'box: agent key on, the server restarted'"
  ;;
*)
  sed -n '2,16p' "$0"
  exit 2
  ;;
esac
