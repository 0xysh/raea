#!/bin/bash
# Two ways raea's Rust went wrong on Windows (11-17: a new workspace came out broken, set-up looped), both held at zero:
#
#  1. canonicalize() through std. On Windows it gives a verbatim path, \\?\C:\..., which Git for Windows can neither
#     read nor lock, Rolldown can't resolve, and which shares no root with the C:\... git and everything else print:
#     `raea workspace init` wrote an include of ../../../\\?\C:/... into the workspace's git config. dunce::canonicalize
#     gives C:\... wherever a path can be said so, and is std's own call on the Mac and Linux.
#  2. The home folder read from HOME alone. Windows gives an app opened from the Start menu or Explorer no HOME, so
#     ~/.raea, the caches and the inbox landed under whatever folder raea ran in. raea_io_home::user_home() reads HOME,
#     then USERPROFILE; a line that reads USERPROFILE itself passes too.
#
#   tools/windows-paths.sh    check: each offending line, and exit 1 when there's one
#
# Comment lines don't count. Not checked: vendored crates (not ours), and for the home folder two test tools for the
# Mac and Unix only, each its own workspace (Unix PATH strings, ~/Library/Caches): sdk/ui/conformance and sdk/ui/parity.
#
# WAITING: a file whose fix is on a branch of its own that hasn't landed yet, so the check holds at zero everywhere
# else meanwhile. A waiting file with nothing left to fix fails, so the list only shrinks:
#   engine/render/src/lib.rs   the texture folder's canonicalize (windows-paths-render)
WAITING=(engine/render/src/lib.rs)
set -u
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || exit 2
NOT_OURS=(':!vendor/*' ':!*/vendor/*')
MAC_TOOLS=(':!sdk/ui/conformance/*' ':!sdk/ui/parity/*')
code() { grep -v -E '^[^:]+:[0-9]+:[[:space:]]*//'; }
canon=$(git grep -n -E '\.canonicalize\(\)|(^|[^A-Za-z0-9_:])(std::)?fs::canonicalize\(|tokio::fs::canonicalize\(' -- '*.rs' "${NOT_OURS[@]}" | code)
home=$(git grep -n -E 'env::var(_os)?\("HOME"\)' -- '*.rs' "${NOT_OURS[@]}" "${MAC_TOOLS[@]}" | code | grep -v USERPROFILE)
status=0
waits=()
for w in ${WAITING[@]+"${WAITING[@]}"}; do
  if grep -q "^$w:" <<< "$canon"$'\n'"$home"; then
    canon=$(grep -v "^$w:" <<< "$canon"); home=$(grep -v "^$w:" <<< "$home"); waits+=("$w")
  else
    echo "windows-paths: $w has nothing left to fix; take it off WAITING (tools/windows-paths.sh)"
    status=1
  fi
done
if [ -n "$canon" ]; then
  echo "windows-paths: canonicalize() through std gives \\\\?\\C:\\... on Windows; use dunce::canonicalize(&path):"
  printf '  %s\n' "${canon//$'\n'/$'\n'  }"
  status=1
fi
if [ -n "$home" ]; then
  echo "windows-paths: HOME alone is unset on Windows; use raea_io_home::user_home() (HOME, then USERPROFILE):"
  printf '  %s\n' "${home//$'\n'/$'\n'  }"
  status=1
fi
[ $status = 0 ] && echo "windows-paths: ok (every canonicalize through dunce, every home folder through user_home${waits[0]+; waiting: ${waits[*]}})"
exit $status
