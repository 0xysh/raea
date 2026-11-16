// The host's side of `raea clothes` (clothes.ts) on the native raea. SOMA-X's two body files are fetched once at a
// fixed commit (NVlabs/SOMA-X, Apache-2.0, checked by their SHA-256 and kept with the licence in raea's models
// folder, soma/), SOMA's body and the mannequin's views are made from them once by Blender (tools/clothes/body.py,
// mannequin.py), each view is dressed by `raea gen image`, and garments are measured and worn by
// tools/clothes/garment.py. A project's garments are assets/clothes/NAME.json, with their pictures in
// assets/clothes/NAME/.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, realpathSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, isAbsolute, join, relative } from "node:path";
import { gradeToHead } from "./grade";

type Env = Record<string, string | undefined>;
type Said = { said: string };

const COMMIT = "01ab5b5c0876058bd27d34b1607d74b2e55e0cf4";
const SOMA_X: Record<string, string> = {
  "SOMA_template_rig.usda": "a952cb7bdf4ccd801e355d53122d395b985b69d0d9181439edad5081bb171856",
  "SOMA_neutral.npz": "5829af4a5333abdba5ad449360b868faacee99d395971e62f35c0e7169432b63",
};
const VIEWS = ["front", "back", "left", "right"];
// what every picture must keep, so it lies over the mannequin pixel for pixel and the measure can read it
const KEEP =
  "Keep EXACTLY the pose, the proportions, the place of every limb and finger, the size in the frame and the camera view. Where no clothing covers it the mannequin stays featureless dark grey. Flat, even, unlit colour: no cast shadows, no highlights, plain light grey #C8C8C8 background, no text.";
const seen = (v: string) =>
  v === "front"
    ? "seen from the FRONT"
    : v === "back"
      ? "seen from BEHIND: we see its back, the back of its head and its heels, and only the back of each garment (no front opening, zip or buttons)"
      : `seen exactly from its ${v} side, in profile`;

/** raea's models folder (raea_io_home::models): ~/.local/share/raea-models, %LOCALAPPDATA%\raea\models on Windows */
const modelsDir = (env: Env) => (process.platform === "win32" && env.LOCALAPPDATA ? join(env.LOCALAPPDATA, "raea", "models") : join(env.HOME ?? "", ".local/share/raea-models"));

/** A script of tools/clothes: in RAEA_HOME, beside a release's bin/raea (share/raea/tools), or in the tree a built raea is in */
function tool(raea: string, env: Env, file: string): string {
  const bin = dirname(realpathSync(raea));
  const found = [env.RAEA_HOME && join(env.RAEA_HOME, "tools/clothes", file), join(bin, "../share/raea/tools/clothes", file), join(bin, "../../tools/clothes", file)].find((p) => p && existsSync(p));
  if (!found) throw new Error(`no tools/clothes/${file} beside ${bin} (a release keeps it in share/raea/tools/clothes)`);
  return found;
}

/** A tools/clothes script in a windowless Blender (`raea blender run`); the lines it says for raea ("clothes: ...") */
function blender(raea: string, env: Env, file: string, args: string[]): string[] {
  const r = spawnSync(raea, ["blender", "run", tool(raea, env, file), ...args], { encoding: "utf8", maxBuffer: 256 << 20 });
  const all = `${r.stdout ?? ""}${r.stderr ?? ""}`;
  if (r.status !== 0) throw new Error(`tools/clothes/${file} failed in Blender:\n${all.trim().split("\n").slice(-12).join("\n")}`);
  return all.split("\n").filter((l) => l.startsWith("clothes: ")).map((l) => l.slice(9));
}

/** SOMA-X's body files and licence in raea's models folder, fetched the first time and checked by their SHA-256 */
function somaX(dir: string): { rig: string; shapes: string } {
  mkdirSync(dir, { recursive: true });
  const get = (url: string, to: string) => {
    const r = spawnSync("curl", ["-fsSL", "--retry", "2", "-o", to, url], { encoding: "utf8" });
    if (r.status !== 0) throw new Error(`couldn't fetch SOMA's body from ${url}: ${(r.stderr ?? "").trim()}`);
  };
  for (const [file, sha] of Object.entries(SOMA_X)) {
    const to = join(dir, file);
    if (existsSync(to)) continue;
    get(`https://media.githubusercontent.com/media/NVlabs/SOMA-X/${COMMIT}/assets/${file}`, `${to}.part`);
    const got = createHash("sha256").update(readFileSync(`${to}.part`)).digest("hex");
    if (got !== sha) {
      rmSync(`${to}.part`, { force: true });
      throw new Error(`SOMA-X's ${file} came back changed (SHA-256 ${got}, not ${sha}); nothing was kept`);
    }
    renameSync(`${to}.part`, to);
  }
  if (!existsSync(join(dir, "LICENSE"))) get(`https://raw.githubusercontent.com/NVlabs/SOMA-X/${COMMIT}/LICENSE`, join(dir, "LICENSE"));
  return { rig: join(dir, "SOMA_template_rig.usda"), shapes: join(dir, "SOMA_neutral.npz") };
}

/** SOMA's body (soma/body: body.glb, skeleton.json) and the mannequin's four views (soma/mannequin), made once */
function soma(raea: string, env: Env): { rig: string; body: string; mannequin: string; made: string[] } {
  const dir = join(modelsDir(env), "soma"), x = somaX(dir), made: string[] = [];
  const body = join(dir, "body"), mannequin = join(dir, "mannequin");
  if (!["body.glb", "skeleton.json", "joint.glb"].every((f) => existsSync(join(body, f)))) made.push(...blender(raea, env, "body.py", [x.rig, x.shapes, body]));
  if (!VIEWS.every((v) => existsSync(join(mannequin, `mannequin_${v}.png`)))) made.push(...blender(raea, env, "mannequin.py", [join(body, "body.glb"), mannequin]));
  return { rig: x.rig, body, mannequin, made };
}

type Garment = { name: string; layer: number; vertices: number[]; stand: number[]; over?: string; topology?: string; source?: { views?: string } };
const readGarment = (file: string) => JSON.parse(readFileSync(file, "utf8")) as Garment;

/** a garment and what it is over, innermost first */
function chainOf(clothes: string, name: string, depth = 0): string[] {
  const file = join(clothes, `${name}.json`);
  if (!existsSync(file)) throw new Error(`no garment ${name} (assets/clothes/${name}.json): raea clothes lists the ones made`);
  // (over several at once: "tee+shorts")
  const g = readGarment(file), under = g.over && depth < 8 ? g.over.split("+").flatMap((o) => chainOf(clothes, o, depth + 1)) : [];
  return [...new Set(under.filter((n) => n !== name)), name];
}

/** What a new garment is drawn over: the pictures to draw on and measure against, and the layer beneath as one
 * garment file. One garment drawn from pictures is its own pictures; a garment made by procedure (no pictures), or
 * several at once, is drawn: the mannequin wearing them, innermost first, by Blender. */
function beneath(raea: string, env: Env, s: { rig: string; body: string }, clothes: string, own: string, over: string[]): { views: string; file: string; said: string[] } {
  const one = over.length === 1 ? readGarment(join(clothes, `${over[0]}.json`)) : undefined;
  if (one?.source?.views && existsSync(join(clothes, over[0], "views", "front.jpg"))) return { views: join(clothes, over[0], "views"), file: join(clothes, `${over[0]}.json`), said: [] };
  const names = [...new Set(over.flatMap((n) => chainOf(clothes, n)))], files = names.map((n) => join(clothes, `${n}.json`));
  const worn = join(own, "under"), views = join(own, "under", "views");
  blender(raea, env, "garment.py", ["wear", join(s.body, "body.glb"), s.rig, worn, ...files]);
  blender(raea, env, "mannequin.py", [join(worn, "body.glb"), views, "1024", ...names.map((n) => join(worn, `${n}.glb`))]);
  // the layers beneath as one: every vertex any of them covers, at the farthest any stands off
  const gs = files.map(readGarment), stand = new Map<number, number>();
  for (const g of gs) g.vertices.forEach((v, k) => stand.set(v, Math.max(stand.get(v) ?? 0, g.stand[k])));
  const ids = [...stand.keys()].sort((x, y) => x - y), file = join(worn, "beneath.json");
  writeFileSync(file, JSON.stringify({ name: over.join("+"), layer: Math.max(...gs.map((g) => g.layer)), vertices: ids, stand: ids.map((v) => stand.get(v)) }));
  return { views, file, said: [`drew the mannequin wearing ${names.join(", ")} to draw over (${views})`] };
}

function list(cwd: string) {
  const dir = join(cwd, "assets/clothes");
  const files = existsSync(dir) ? readdirSync(dir).filter((f) => f.endsWith(".json")).sort() : [];
  const garments = files.map((f) => readGarment(join(dir, f))).filter((g) => g.topology?.startsWith("SOMA"));
  return { garments: garments.map((g) => ({ name: g.name, layer: g.layer, vertices: g.vertices.length, over: g.over })) };
}

/** what one picture cost: the gen log's last "cents" (its total; the ones before are its steps') */
const centsOf = (log: string) => Number([...readFileSync(log, "utf8").matchAll(/"cents":\s*(\d+(?:\.\d+)?)/g)].pop()?.[1] ?? 0);

/** The four views dressed by the image model at once, each its own call (a sheet of views, or a second picture to
 * follow, loses the registration), kept as OWN/views/VIEW.jpg; what they cost, in cents */
function draw(raea: string, cwd: string, own: string, sources: (v: string) => string, layered: boolean, words: string, cents: number): number {
  const views = join(own, "views");
  mkdirSync(views, { recursive: true });
  const jobs = VIEWS.flatMap((v) => {
    const src = sources(v);
    if (!existsSync(src)) throw new Error(`${src}: the ${v} view to draw on is missing`);
    const prompt = layered
      ? `This dressed mannequin is ${seen(v)}. Add exactly one thing and keep everything else pixel for pixel: ${words}, worn over what it already wears. ${KEEP}`
      : `This grey mannequin is ${seen(v)}. Turn it into the same mannequin, still ${seen(v)}, wearing ${words}; only the surface changes where clothing covers it. ${KEEP}`;
    rmSync(join(own, `gen_${v}`), { recursive: true, force: true });
    return [prompt, src, join(own, `gen_${v}`), String(cents)];
  });
  const run = 'r=$1; shift; while [ $# -gt 0 ]; do ( "$r" gen image "$1" --model flux_2_pro --image "$2" --aspect 1:1 -o "$3" --max-cents "$4" > "$3.log" 2>&1 ) & shift 4; done; wait';
  spawnSync("bash", ["-c", run, "clothes", raea, ...jobs], { encoding: "utf8" });
  let spent = 0;
  for (const v of VIEWS) {
    const dir = join(own, `gen_${v}`), pic = existsSync(dir) ? readdirSync(dir).find((f) => /^out\.(jpe?g|png|webp)$/.test(f)) : undefined;
    if (!pic) throw new Error(`no picture for the ${v} view: ${relative(cwd, `${dir}.log`)} says why`);
    copyFileSync(join(dir, pic), join(views, `${v}.jpg`));
    spent += centsOf(`${dir}.log`);
  }
  return spent;
}

/** make draws and measures; again (remeasure) measures the pictures already kept, drawing nothing */
function make(raea: string, cwd: string, env: Env, a: { name: string; words?: string; over?: string; cents?: number; again?: boolean }): Said {
  const clothes = join(cwd, "assets/clothes"), own = join(clothes, a.name), views = join(own, "views"), file = join(clothes, `${a.name}.json`);
  let over = a.over ? a.over.split(",") : [];
  if (a.again) {
    if (!existsSync(file)) throw new Error(`no garment ${a.name} (assets/clothes/${a.name}.json): raea clothes lists the ones made`);
    const g = readGarment(file);
    if (!g.source?.views || !VIEWS.every((v) => existsSync(join(views, `${v}.jpg`)))) throw new Error(`${a.name} has no pictures to measure (${relative(cwd, views)}): one made by procedure is made again with raea clothes cover`);
    over = g.over ? g.over.split("+") : [];
  }
  for (const o of over) if (!existsSync(join(clothes, `${o}.json`))) throw new Error(`no garment ${o} (assets/clothes/${o}.json): raea clothes lists the ones made`);
  if (over.includes(a.name)) throw new Error("a garment can't be drawn over itself");
  const s = soma(raea, env);
  mkdirSync(own, { recursive: true });
  const under = over.length ? beneath(raea, env, s, clothes, own, over) : undefined;
  const cents = a.again ? 0 : draw(raea, cwd, own, (v) => (under ? join(under.views, `${v}.jpg`) : join(s.mannequin, `mannequin_${v}.png`)), !!under, a.words ?? "", a.cents ?? 10);
  const said = blender(raea, env, "garment.py", ["measure", join(s.body, "body.glb"), s.rig, views, file, ...(under ? [under.file, under.views] : [])]);
  if (!existsSync(file)) throw new Error(`the pictures didn't measure into a garment (${relative(cwd, views)})`);
  const worn = [...new Set(over.flatMap((o) => chainOf(clothes, o))), a.name].join(" ");
  return {
    said: [
      ...s.made,
      ...(under?.said ?? []),
      a.again ? `${a.name}: measured again from its four pictures (${relative(cwd, views)}), nothing drawn` : `${a.name}: four pictures, ${cents} cents (${relative(cwd, views)})`,
      ...said,
      `${relative(cwd, file)}. raea clothes wear assets/characters/NAME ${worn} puts it on SOMA's body; look at its pictures before you do.`,
    ].join("\n") + "\n",
  };
}

function cover(raea: string, cwd: string, env: Env, a: { name: string; parts: string; colour: string; stand: number; over?: string }): Said {
  const clothes = join(cwd, "assets/clothes"), file = join(clothes, `${a.name}.json`), under = a.over ? join(clothes, `${a.over}.json`) : undefined;
  if (under && !existsSync(under)) throw new Error(`no garment ${a.over} (assets/clothes/${a.over}.json): raea clothes lists the ones made`);
  if (a.over === a.name) throw new Error("a garment can't be laid over itself");
  const s = soma(raea, env);
  mkdirSync(clothes, { recursive: true });
  const said = blender(raea, env, "garment.py", ["cover", join(s.body, "body.glb"), s.rig, file, a.parts, a.colour, String(a.stand), ...(under ? [under] : [])]);
  if (!existsSync(file)) throw new Error("no garment was written");
  return { said: [...s.made, ...said, `${relative(cwd, file)}. raea clothes wear assets/characters/NAME ${a.over ? `${a.over} ` : ""}${a.name} puts it on SOMA's body.`].join("\n") + "\n" };
}

function wear(raea: string, cwd: string, env: Env, a: { out: string; garments: string[]; body?: string; head?: string }): Said {
  const at = (p: string) => (isAbsolute(p) ? p : join(cwd, p));
  const files = a.garments.map((g) => (g.endsWith(".json") ? at(g) : join(cwd, "assets/clothes", `${g}.json`)));
  for (const f of files) if (!existsSync(f)) throw new Error(`no garment ${relative(cwd, f)}: raea clothes lists the ones made`);
  // innermost first: a garment drawn over another goes after it
  const layers = files.map((f) => readGarment(f).layer);
  if (layers.some((l, k) => k > 0 && l < layers[k - 1])) throw new Error(`put the garments innermost first (their layers are ${layers.join(", ")})`);
  const s = soma(raea, env), body = a.body ? at(a.body) : s.body, out = at(a.out);
  if (!existsSync(join(body, "body.glb"))) throw new Error(`${a.body}: not a body folder (no body.glb)`);
  mkdirSync(out, { recursive: true });
  const said = blender(raea, env, "garment.py", ["wear", join(body, "body.glb"), s.rig, out, ...files]);
  // the dressed body is a body folder of its own: its skeleton, its joints' stand-in and a headless body's collar
  for (const f of ["skeleton.json", "joint.glb", "collar.json"]) if (existsSync(join(body, f))) copyFileSync(join(body, f), join(out, f));
  // a head that joins the body at the collar: the body's skin graded to the head's (grade.ts)
  if (a.head) {
    const json = join(at(a.head), "head.json");
    if (!existsSync(json)) throw new Error(`${a.head}: not a head folder (no head.json)`);
    said.push(gradeToHead(join(out, "body.glb"), json));
  }
  const served = relative(join(cwd, "assets"), out);
  const use = served.startsWith("..")
    ? ""
    : `\nIn a composition: registerBody(skeleton, staticFile("${served}")), then <Character body=... parts={[${files.map((f) => `{ src: staticFile("${served}/${basename(f, ".json")}.glb") }`).join(", ")}]} />`;
  return { said: [...s.made, ...said].join("\n") + use + "\n" };
}

/** host op "clothes": list, make, remeasure, cover or wear */
export function clothesHost(raea: string, cwd: string, env: Env, a: Record<string, unknown>): unknown {
  switch (a.op) {
    case "list":
      return list(cwd);
    case "make":
      return make(raea, cwd, env, a as Parameters<typeof make>[3]);
    case "remeasure":
      return make(raea, cwd, env, { name: String(a.name), again: true });
    case "cover":
      return cover(raea, cwd, env, a as Parameters<typeof cover>[3]);
    case "wear":
      return wear(raea, cwd, env, a as Parameters<typeof wear>[3]);
    default:
      throw new Error(`raea clothes: no step ${String(a.op)}`);
  }
}
