#!/bin/bash
# What makes raea need a Rust build, and what each cause becomes in TypeScript (docs/nobuild.feature NB0, NB7).
#
#   tools/nobuild.sh builds [DAYS]   why raea was built over the last DAYS (14): every antsy run whose command is a cargo or
#                                    lane build, sorted by its stated purpose (--for) into the baseline's classes; with
#                                    NOBUILD_ROWS=1 each class's purposes too, to read by hand
#   tools/nobuild.sh scan            every cause in the code (RAEA_* environment reads, shader constants, cargo features
#                                    of the binaries tools build, declared Bevy plugins), each joined with its decision
#                                    in docs/nobuild/decisions.tsv, written to docs/nobuild/inventory.tsv; prints what
#                                    has no decision yet and the counts by kind and status
#
# A decision row (tab-separated): kind:name, then its class, the TypeScript form it becomes, its check, its status
# (open, doing, done, stays). Classes: frame (changes what a frame shows or costs: a prop), view (a debug view: NB2),
# stat (a trace, a timing, a counter: NB2), machine (paths, hosts, keys, threads, decoders: stays an environment
# setting), switch (a kill switch kept for an A/B: removed once decided, or a prop), feature, plugin, shader, glue.
set -uo pipefail
RAEA=${RAEA_ROOT:-$HOME/raea}
DOCS=$RAEA/docs/nobuild
mkdir -p "$DOCS"

builds() {
  local days=${1:-14}
  local hist=${ANTSY_HOME:-$HOME/.antsy}/history.jsonl
  [ -f "$hist" ] || { echo "nobuild: no antsy history at $hist" >&2; exit 1; }
  # the classes in order: the first whose words a purpose has is its class
  jq -rs --argjson days "$days" --arg rows "${NOBUILD_ROWS:-0}" '
    (now - $days * 86400) as $from
    | [.[] | select(.ts >= $from) | select((.cmd // "") | test("cargo (build|run|test|check)\\b|lane build"))]
    | map(. + {class: ((.for // "") | ascii_downcase |
        if test("\\b(a/b|tune|tuning|try|tried|trying|debug|temporar|trace|bisect|time it|timing|timed|measure|threshold|knob|env|logging|eprintln|probe|experiment|variant|without)") then "a value, a toggle, a debug view or a timing"
        elif test("(<[a-z][a-z0-9]*|\\bprops?\\b|component|plugin|parity host|\\bhost\\b|feature|bridge|bevy[_ ]|keyed rows|reflected|rows\\b)") then "a TS-facing feature that needed engine glue"
        elif test("(shader|wgsl|wesl|solari|metalfx|denois|ssao|\\bao\\b|light|shadow|bounce|cache|ray|path tracer|bloom|fog|exposure)") then "shader and lighting work"
        elif test("(test|compile|clippy|lint|\\bl1\\b|\\bci\\b|release|land\\b|landing|rc[0-9]|dupes|fold|help)") then "tests, lint, release and landing"
        else "other engine work" end)})
    | "\(length) builds, \((map(.secs) | add // 0) / 3600 * 10 | floor / 10) h, \(map(.session) | unique | length) sessions, the last \($days) days",
      (group_by(.class) | sort_by(-length)[] |
        "  \(length | tostring | (" " * (5 - length)) + .)  \((map(.secs) | add) / 3600 * 10 | floor / 10) h  \(.[0].class)",
        (if $rows == "1" then (map(.for // "") | unique[] | "        " + .[0:140]) else empty end))' "$hist"
}

# the causes, one a line: kind<TAB>name<TAB>where
causes() {
  cd "$RAEA" || exit 1
  # every RAEA_* variable the code names (its first mention), raea's own and its patches to vendored crates: any quoted
  # literal, as many are read through a helper (a switch(name), a settings table) rather than env::var at the name
  # (git grep: tracked files only, so no target/ or node_modules)
  git grep -nE '"RAEA_[A-Z0-9_]+"' -- '*.rs' ':!vendor' 'vendor/bevy_solari' 'vendor/blitz-dom' \
      'vendor/blitz-paint' 'vendor/parley_engine' 'vendor/bevy_pbr' 'vendor/bevy_ui' |
    awk -F: '{ w = $1 ":" $2; s = $0; while (match(s, /"RAEA_[A-Z0-9_]+"/)) { print "env\t" substr(s, RSTART + 1, RLENGTH - 2) "\t" w; s = substr(s, RSTART + RLENGTH) } }' |
    sort -t$'\t' -k2,2 -u
  # constants in raea's and the vendored Solari's shaders (bench/ is LightBench's own)
  git grep -nE '^[[:space:]]*const [A-Z][A-Z0-9_]*[[:space:]]*(:|=)' -- 'engine/*.wgsl' 'engine/*.wesl' 'systems/*.wgsl' 'systems/*.wesl' \
      'vendor/bevy_solari/*.wgsl' 'vendor/bevy_solari/*.wesl' |
    sed -E 's/^([^:]+):([0-9]+):[[:space:]]*const ([A-Z][A-Z0-9_]*).*/shader\t\3\t\1:\2/' | sort -t$'\t' -k2,2 -k3,3 -u
  # optional features of every binary raea ships or its sessions build (each a binary that differs from the shipped one)
  local t
  for t in engine/cli/Cargo.toml engine/page/host/Cargo.toml engine/page/window/Cargo.toml apps/game/Cargo.toml; do
    awk -v f="$t" '/^\[features\]/{p=1; next} /^\[/{p=0} p && /^[a-z0-9_-]+ *=/ && $1 != "default" {print "feature\t" f "#" $1 "\t" f ":" NR}' "$t"
  done
  # places that tell someone to run cargo: the skill (the submodule's own files), and a message or doc in the code
  { grep -rnE 'cargo (build|run|install)' skills/skills 2>/dev/null; git grep -nE '"[^"]*cargo (build|run)[^"]*"' -- '*.rs' ':!bench' ':!tools'; } |
    awk -F: '{ print "advice\t" $1 ":" $2 "\t" $1 ":" $2 }'
  # what the scan can't find, written down as it's found (docs/nobuild/manual.tsv)
  [ -f docs/nobuild/manual.tsv ] && grep -v '^#' docs/nobuild/manual.tsv
  # Bevy plugins a project may declare (engine/bevy-plugins' declared features: each needs tools/bevy-plugins.sh build)
  awk '/>>> declared features/{p=1; next} /<<< declared features/{p=0} p && /^[a-z0-9_-]+ *=/{split($0, a, " "); print "plugin\t" a[1] "\tengine/bevy-plugins/Cargo.toml:" NR}' engine/bevy-plugins/Cargo.toml
}

scan() {
  local dec=$DOCS/decisions.tsv out=$DOCS/inventory.tsv
  [ -f "$dec" ] || printf '# kind:name\tclass\tts_form\tcheck\tstatus\n' > "$dec"
  causes > "$DOCS/.causes.tsv"
  # join each cause with its decision (kind:name); a cause with none is "open", class "?"
  awk -F'\t' -v OFS='\t' '
    FNR == NR { if ($0 !~ /^#/ && NF >= 2) d[$1] = $2 OFS $3 OFS $4 OFS $5; next }
    { k = $1 ":" $2; if (k in d) print $1, $2, $3, d[k]; else print $1, $2, $3, "?", "", "", "open" }
  ' "$dec" "$DOCS/.causes.tsv" > "$out.tmp"
  { printf 'kind\tname\twhere\tclass\tts_form\tcheck\tstatus\n'; sort "$out.tmp"; } > "$out"
  rm -f -- "${out:?}.tmp" "${DOCS:?}/.causes.tsv"
  echo "nobuild: $(($(wc -l < "$out") - 1)) causes in ${out#"$RAEA"/}"
  awk -F'\t' 'NR > 1 { n[$1 " " $4]++ } END { for (k in n) printf "  %4d  %s\n", n[k], k }' "$out" | sort -k2
  awk -F'\t' 'NR > 1 { s[$7]++ } END { for (k in s) printf "  status %s: %d\n", k, s[k] }' "$out" | sort
  local undecided
  undecided=$(awk -F'\t' 'NR > 1 && $4 == "?"' "$out" | wc -l | tr -d ' ')
  [ "$undecided" = 0 ] || echo "nobuild: $undecided causes have no decision (class ?) in docs/nobuild/decisions.tsv"
}

case ${1:-} in
  builds) shift; builds "$@" ;;
  scan) scan ;;
  *) sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
