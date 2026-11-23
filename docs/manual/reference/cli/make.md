---
title: "raea make"
summary: "Whether a scene, kit or shot made to its plan is done: what's missing, each gate, the judge's rounds, decided in code; the agent's Stop hook holds it there."
order: 44
sources: ["apps/make/src/lib.rs", "apps/make/src/status.rs", "core/brief/src/make.rs", "apps/check/src/make_gate.rs"]
---

`raea make status` reads the job on a project's brief.ts (`make: scene({...})`, `kit({...})` or `shot({...})` from
`raea/brief`) and says what's still missing, from what's on disk and never from what the agent says:

- the kind's deliverables (a scene's three or more stills; a kit's GLB, LOD1 and thumbnail per object, its lineup
  and kit.json; a shot's still or move per shot), or the plan's own `deliverables`;
- NOTES.md, ending with a "What the skill got wrong" section;
- each composition the plan names has a render or check newer than it that found no error (the project's record);
- a fresh critic (`raea judge`) has judged a round newer than the newest deliverable, and each blocking defect it
  named is fixed or answered in NOTES.md by its name; once the judge is calibrated against your eye, two passes in a
  row as well.

A job is done when all of that holds, and also at its round cap (8 by default) or when three rounds in a row haven't
beaten the best before them: then it's delivered short, with what's missing listed. raea's Stop hook (`raea hook`)
asks it at the end of every turn in a project whose brief has `make`, and keeps an agent working until it's done; the
same open items block a session three times at most. The loop it holds is in docs/make.feature.

```sh
raea make status
raea make status ~/Documents/raea/acme/projects/flight-desk-kit --json
```

<!-- generated:begin by tools/manual-ref/cli.ts from `raea help --json`; edit only the flag descriptions -->

## Usage

```sh
raea make status [DIR] [--json]   (a scene, kit or shot held to its plan, `make` on brief.ts: what's missing, each gate, the judge's rounds, and whether it's done, decided in code; the agent's Stop hook holds it there)
```

Headless: it needs nothing running and opens no window. Takes any number of arguments besides its flags.

## Flags

| Flag | Takes | What it does |
| --- | --- | --- |
| `--json` | – | Print the status as JSON: missing, gates, judge, done, short and why. |

<!-- generated:end -->
