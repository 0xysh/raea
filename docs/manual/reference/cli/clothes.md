---
title: "raea clothes"
summary: "Clothes for SOMA's body from words: make has an image model (flux_2_pro, one picture a view, at most N cents each, 10 unless said) dress SOMA's grey mannequin from the front, back, left and right, then measures the pictures onto SOMA's own vertices as assets/clothes/NAME.json, its pictures in assets/clothes/NAME/."
order: 104
sources: ["engine/cli/src/cli.rs"]
---

Clothes for SOMA's body from words: make has an image model (flux_2_pro, one picture a view, at most N cents each, 10 unless said) dress SOMA's grey mannequin from the front, back, left and right, then measures the pictures onto SOMA's own vertices as assets/clothes/NAME.json, its pictures in assets/clothes/NAME/.

<!-- generated:begin by tools/manual-ref/cli.ts from `raea help --json`; edit only the flag descriptions -->

## Usage

```sh
raea clothes [make NAME "WHAT IT IS, AS WORN" [--over UNDER[,UNDER...]] [--max-cents N] | remeasure NAME | cover NAME --parts PART[:SHARE],... --colour #RRGGBB [--stand MM] [--over UNDER] | wear OUT_DIR GARMENT... [--body DIR] [--head HEAD_DIR]]   (clothes for SOMA's body from words: make has an image model (flux_2_pro, one picture a view, at most N cents each, 10 unless said) dress SOMA's grey mannequin from the front, back, left and right, then measures the pictures onto SOMA's own vertices as assets/clothes/NAME.json, its pictures in assets/clothes/NAME/; --over draws it over garments already made (innermost first), as the next layer, a garment made by procedure drawn on the mannequin first; remeasure measures a garment again from its kept pictures (after raea's measuring changed), nothing drawn or paid for; cover makes one by procedure in the same file, no picture: the skin of the parts named (hips, waist, torso, chest, neck, head, upperArm, foreArm, hand, thigh, shin, foot, toes; a limb's SHARE is how far along it from its top), in one colour, MM off the skin (6 unless said); wear puts garments on a body, innermost first (each GARMENT a name or its .json): OUT_DIR/NAME.glb each, with OUT_DIR/body.glb the body without the skin wholly under cloth; --body DIR another SOMA body folder (a headless one for a head part); --head HEAD_DIR the head that joins it (head.json), whose skin the body's is graded to at the collar; alone, the project's garments. The first use fetches SOMA's body (NVlabs/SOMA-X, Apache-2.0) into raea's models folder and needs Blender: raea setup blender)
```

Headless: it needs nothing running and opens no window. Takes any number of arguments besides its flags.

## Flags

| Flag | Takes | What it does |
| --- | --- | --- |
| `--body` | `DIR` | With `wear`: dress this SOMA body folder (body.glb, skeleton.json) instead of SOMA's own, such as a headless body that a generated head joins at the collar. |
| `--colour` | `#RRGGBB` | With `cover`: the garment's one colour. |
| `--head` | `HEAD_DIR` | With `wear`: the head folder (head.json, head.glb) that joins a headless body at the collar. The body's skin takes the head's neck colour and is graded to the head's own skin just under the collar, so no step of colour shows at the join. |
| `--max-cents` | `N` | With `make`: the most one of the four pictures may cost, in cents (10 unless said). |
| `--over` | `UNDER` | With `make`: draw the garment over garments already made, innermost first, comma-separated, as the next layer. With `cover`: lay it over one garment, a layer's thickness off it. |
| `--parts` | `PART` | With `cover`: the body's parts the garment covers, comma-separated; a limb takes a share, how far along it from its top (`foreArm:0.5` is the half nearest the elbow). |
| `--stand` | `MM` | With `cover`: how far the cloth stands off the skin, in millimetres (6 unless said). |

<!-- generated:end -->

## An outfit, layer by layer

```sh
raea clothes cover tee --parts chest,torso,waist,upperArm:0.45 --colour "#e8e4da"
raea clothes cover shorts --parts hips,thigh:0.55 --colour "#2b3a55" --stand 7
raea clothes make overshirt "a red and black buffalo-check flannel overshirt, worn open" --over tee,shorts
raea clothes wear assets/characters/flannel tee shorts overshirt
```

The first two are made by procedure and cost nothing. The third has an image model draw the mannequin's four views over the first two, one picture a view (about 20 cents), and measures them onto SOMA's vertices; look at its pictures in `assets/clothes/overshirt/views` before you wear it. `wear` writes a body folder and prints the lines that put it in a composition: `registerBody` with the folder, and each garment's `.glb` as one of `<Character>`'s `parts`.

A garment is a file on SOMA's template vertices, so the same file dresses every SOMA body. It lies on the skin at a measured stand-off: cloth that hangs free of the body (a skirt, a coat's tails, a shirt hanging over the hips) is not made by this command yet.
