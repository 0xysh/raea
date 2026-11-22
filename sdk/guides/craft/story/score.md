# Scoring a story

A story that passes its checks holds together. Whether it's good is a different question, answered three ways.

## 1. Read it through lenses

`readThrough(story, [LENSES.mckee, LENSES.pas], telling)` gives each lens's map (which beats are which stage), its
findings and its proposals: what's missing, as a beat to add (`{ stage, after, onScreen, feel, why }`). A proposal is
a starting point for a beat, not a beat. Read [router](router.md) to pick the lens.

## 2. Ask the critic

The critic is a small model (about 0.6B parameters, on this machine) trained on exact labels: valid stories, and the
same stories with a beat cut, two beats swapped or a stranger's beat spliced in, each labelled by `checkStory`. It
reads a script as prose, numbered by beat, and answers `{ breaks: [{ beat, check, level }] }` under a schema, so its
answer is always data. Use it where there's no story data yet: a script someone wrote, a transcript, a cut you were
handed. Its held-out numbers against an untrained model and a frontier judge are in docs/story.md.

The generator is its partner: given a story with a beat missing, it writes the beat under that story's own schema
(its references can only be the story's keys), and the beat it writes is checked like any other.

## 3. Hold it to a real shape

A fingerprint of real films that did well (raea fingerprint) says what their shape was: how fast the problem
arrives, when the product is first seen, how long the proof holds. A story's telling can be scored against it before
anything is drawn. Shape is evidence, not a rule: hold it loosely, and never copy a film's beats.

## What none of these can tell you

Whether it's funny, whether the line lands, whether the person will like it. Show the script to the person before
the film, and believe what they say over any score.
