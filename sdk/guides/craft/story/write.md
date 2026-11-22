# Writing a story

A story is a module, `NAME.story.ts`, with one default export made by `story()`. tsc checks its references: a beat
that shows an event you didn't write, an agent you didn't declare, a setup nobody planted, won't compile.

```ts
import { story, value } from "raea/story";

export default story({
  title: "Halden Outdoor: the first tent",
  idea: { value: "nerve", side: "you're braver than your gear", counter: "only experts go out" },
  values: { nerve: value("afraid", "bold") },
  facts: { pitched: "the tent goes up in one pull" },
  agents: {
    june: { want: "a night outside", need: "to trust herself", role: "customer", mask: "hero" },
    tent: { role: "brand", mask: "gift" },
  },
  world: {                                            // what happens, in the order it happens
    stays: { who: "june", does: "watches the hills from her window", routine: true, turns: { nerve: -1 } },
    dusk: { who: "june", does: "drives out as the light goes", because: "stays", turns: { nerve: -1 }, dilemma: ["turn back", "keep going"] },
    pitch: { who: "june", with: ["tent"], does: "pulls the tent up in one go", because: "dusk", sets: ["pitched"], turns: { nerve: 3 }, ends: { june: "need" } },
  },
  tellings: {                                         // how each cut tells it
    "20s": [
      { id: "hook", shows: ["pitch"], hides: ["pitched"], asks: ["pitched"], setup: "how", feel: "curiosity", onScreen: "A tent glows on a dark hill." },
      { id: "before", shows: ["stays"], feel: "recognition" },
      { id: "dusk", shows: ["dusk"], feel: "concern" },
      { id: "pull", shows: ["pitch"], reveals: ["pitched"], pays: "how", feel: "delight" },
    ],
  },
  lenses: ["mckee", "storynomics"],
});
```

## The three layers

- **The world** is what happens: events in story order, each with who acts, why (`because`: the earlier event that
  caused it), what it makes true (`sets`), what it moves (`turns`: a value, -3 to 3) and what it settles (`ends`).
  An event that's just how things are is `routine`.
- **A telling** is how one cut tells it: beats in the order the audience sees them. A beat `shows` events (a hook can
  show the ending first), `hides` and `asks` what the audience doesn't know yet, `reveals` it later, plants a `setup`
  and `pays` it. Give each beat an `id`: a composition names its Sequence by it and `raea check` looks for it.
- **The render** is the film: `compileScript(story, telling)` makes the brief's script (a scene a beat, each naming its
  `beat`), and `<Beat times={...} id="hook">` in a composition is a Sequence timed from the telling.

## Into the project

The brief imports the story explicitly: `brief({ ..., story })` (raea keeps it as written). A check imports it too,
`checks/story.ts` (see [check](check.md)). Nothing finds a story by itself: what uses it says so.

## Rules that save a rewrite

- Write the world before any telling. Two cuts of one world (a 15 s and a 45 s) share their causes.
- Every event after the first has a cause or is routine. "And then" is the most common hole.
- A value is a pair of poles ("buried", "free"), not a topic. The `idea.value` is the one the story is about.
- Facts are keys, not sentences: `believes`, `needs`, `sets`, `proof`, `asks` all name declared facts.
- A claim (`claim`) names its `proof`, facts an event makes true. Never write proof nothing shows.
