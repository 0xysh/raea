// Composition and registerRoot: record what's registered so a renderer can list compositions
// (spec: "Declaring and registering", "The clock renderers use").
import { Children, isValidElement, type ComponentType, type ReactElement, type ReactNode } from "react";
import { useVideoConfig } from "@raea/app/clock";
import { Audio, Video } from "./media";
import { Region } from "./region";
import { Freeze, Loop, Sequence, Series } from "./timeline";

export interface CompositionProps<P extends Record<string, unknown> = Record<string, unknown>> {
  /** the composition's id: letters, digits and dashes (`raea render --composition` picks it) */
  id: string;
  /** the React component it renders */
  component: ComponentType<P>;
  /** in px */
  width: number;
  /** in px */
  height: number;
  /** frames a second */
  fps: number;
  /** its length in frames */
  durationInFrames: number;
  /** the props it's rendered with (JSON-like values only) */
  defaultProps?: P;
  /** the composition's controls: which of its defaultProps a person may tune, and how (docs/realtime.feature RT4).
   *  Each names a defaultProps key, whose value is the control's; the feed's Controls panel shows them on the
   *  timeline and a change reaches the live picture as that prop */
  controls?: Controls<P>;
  /** a hand edit laid over the composition (docs/craft-system.md, L): written back by the app's timeline */
  edit?: Edit;
}

/** How a change to a control reaches the picture. `live` (the default): the running stage draws it at once, as a
 *  prop. `reopen`: the stage opens the file again first, for what a running 3D world keeps from its first frame (a
 *  shape, a model's file, an instance count, a picture map, a shader). `resim`: everything from the frame it changes
 *  on is worked out again, for state carried from frame to frame (physics, particles, a game's fold). */
export type ControlCost = "live" | "reopen" | "resim";

interface ControlBase {
  /** what the panel calls it (the key when left out) */
  label?: string;
  /** the panel's group it sits in ("Grade", "Camera"); ungrouped controls come first */
  group?: string;
  /** how a change reaches the picture: "live" unless said */
  cost?: ControlCost;
  /** one line on what it changes, for the panel's tooltip and an agent reading the code */
  about?: string;
}

/** A number, dragged on a slider when it has both ends. */
export interface NumberControl extends ControlBase {
  kind: "number";
  /** the least it may be */
  min?: number;
  /** the most it may be */
  max?: number;
  /** what one step of the slider or an arrow key moves it by (above 0) */
  step?: number;
  /** its unit, shown after it ("px", "°", "s") */
  unit?: string;
}

/** On or off: a boolean. */
export interface SwitchControl extends ControlBase {
  kind: "switch";
}

/** One of a list: a string or a number from `options`. */
export interface ChoiceControl extends ControlBase {
  kind: "choice";
  /** what it may be, in the order the panel lists them (at least one, none twice) */
  options: readonly (string | number)[];
}

/** A colour: a hex string, "#rgb", "#rgba", "#rrggbb" or "#rrggbbaa". */
export interface ColourControl extends ControlBase {
  kind: "colour";
}

/** A string: a title, a caption, a name. */
export interface TextControl extends ControlBase {
  kind: "text";
  /** a box of several lines rather than one */
  multiline?: boolean;
  /** the most characters it may hold */
  maxLength?: number;
}

export type Control = NumberControl | SwitchControl | ChoiceControl | ColourControl | TextControl;

/** A composition's controls, by the defaultProps key each one tunes. */
export type Controls<P = Record<string, unknown>> = { readonly [K in keyof P & string]?: Control };

/** A control as a registered composition holds it: its cost said. */
export type RegisteredControl = Control & { cost: ControlCost };

/** A hand edit: `segments` are the composition's own frame ranges, [from, to), played one after another, so a trim,
 *  a split, a cut word or a moved span is a change to this list; the length becomes their sum. */
export interface Edit {
  /** the composition's own frame ranges, [from, to), played one after another */
  segments?: readonly (readonly [number, number])[];
  /** the composition drawn at this fraction of its size (above 0, at most 1), what is where kept: it is laid out at
   *  its width and height (useVideoConfig still says them) and the whole drawn smaller, so a 3D view traces fewer
   *  pixels. A draft's, where a smaller picture changes nothing that ships: the app's composition drafts and a scene's
   *  kept frames draw at 0.5 (docs/lightspeed.feature PF4) */
  scale?: number;
}

export interface RegisteredComposition {
  /** the composition's id: letters, digits and dashes (`raea render --composition` picks it) */
  id: string;
  /** the React component it renders */
  component: ComponentType<any>;
  /** in px */
  width: number;
  /** in px */
  height: number;
  /** frames a second */
  fps: number;
  /** its length in frames */
  durationInFrames: number;
  /** the props it's rendered with (JSON-like values only) */
  defaultProps: Record<string, unknown>;
  /** its controls, by defaultProps key, in the order they're declared ({} when it has none) */
  controls: Readonly<Record<string, RegisteredControl>>;
}

const ID_PATTERN = /^[A-Za-z0-9-]+$/;
const KINDS: readonly Control["kind"][] = ["number", "switch", "choice", "colour", "text"];
const COSTS: readonly ControlCost[] = ["live", "reopen", "resim"];
const HEX = /^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/;

/** The controls checked against the defaultProps they tune: each names one of its keys, says a kind and a cost
 *  raea knows, and its value fits that kind. `fail(field, why)` throws. */
function validControls(given: unknown, props: Record<string, unknown>, fail: (field: string, why: string) => never): Record<string, RegisteredControl> {
  if (given === undefined) return {};
  if (given === null || typeof given !== "object" || Array.isArray(given) || Object.getPrototypeOf(given) !== Object.prototype) {
    fail("controls", "must be an object of controls by defaultProps key");
  }
  const out: Record<string, RegisteredControl> = {};
  for (const [key, c] of Object.entries(given as Record<string, unknown>)) {
    const at = `controls.${key}`;
    if (!Object.prototype.hasOwnProperty.call(props, key)) {
      const keys = Object.keys(props);
      fail(at, `names no defaultProps key: a control tunes a prop the composition has a value for (defaultProps has ${keys.length ? keys.join(", ") : "none"})`);
    }
    if (c === null || typeof c !== "object" || Array.isArray(c)) fail(at, `must be an object with a kind (${KINDS.join(", ")})`);
    const spec = c as Record<string, unknown>;
    if (!KINDS.includes(spec.kind as Control["kind"])) fail(`${at}.kind`, `must be one of ${KINDS.join(", ")}, got ${JSON.stringify(spec.kind)}`);
    if (spec.cost !== undefined && !COSTS.includes(spec.cost as ControlCost)) fail(`${at}.cost`, `must be one of ${COSTS.join(", ")}, got ${JSON.stringify(spec.cost)}`);
    for (const f of ["label", "group", "about", "unit"] as const) {
      if (spec[f] !== undefined && typeof spec[f] !== "string") fail(`${at}.${f}`, `must be a string, got ${JSON.stringify(spec[f])}`);
    }
    const v = props[key];
    const got = JSON.stringify(v);
    switch (spec.kind as Control["kind"]) {
      case "number": {
        for (const f of ["min", "max", "step"] as const) {
          if (spec[f] !== undefined && !(typeof spec[f] === "number" && Number.isFinite(spec[f]))) fail(`${at}.${f}`, `must be a finite number, got ${JSON.stringify(spec[f])}`);
        }
        const { min, max, step } = spec as { min?: number; max?: number; step?: number };
        if (min !== undefined && max !== undefined && min > max) fail(at, `has min ${min} above max ${max}`);
        if (step !== undefined && !(step > 0)) fail(`${at}.step`, `must be above 0, got ${step}`);
        if (typeof v !== "number" || !Number.isFinite(v)) fail(at, `is a number, but defaultProps.${key} is ${got}`);
        if ((min !== undefined && (v as number) < min) || (max !== undefined && (v as number) > max)) fail(at, `is ${min ?? "-∞"} to ${max ?? "∞"}, but defaultProps.${key} is ${got}`);
        break;
      }
      case "switch":
        if (typeof v !== "boolean") fail(at, `is a switch, but defaultProps.${key} is ${got} (it takes true or false)`);
        break;
      case "choice": {
        const options = spec.options;
        if (!Array.isArray(options) || options.length === 0 || !options.every((o) => typeof o === "string" || (typeof o === "number" && Number.isFinite(o)))) {
          fail(`${at}.options`, "must be a non-empty list of strings or numbers");
        }
        const opts = options as (string | number)[];
        const twice = opts.find((o, i) => opts.indexOf(o) !== i);
        if (twice !== undefined) fail(`${at}.options`, `lists ${JSON.stringify(twice)} twice`);
        if (!opts.includes(v as string | number)) fail(at, `is one of ${opts.map((o) => JSON.stringify(o)).join(", ")}, but defaultProps.${key} is ${got}`);
        break;
      }
      case "colour":
        if (typeof v !== "string" || !HEX.test(v)) fail(at, `is a colour, but defaultProps.${key} is ${got} (it takes "#rgb", "#rgba", "#rrggbb" or "#rrggbbaa")`);
        break;
      case "text": {
        const { maxLength } = spec as { maxLength?: unknown };
        if (maxLength !== undefined && !(Number.isInteger(maxLength) && (maxLength as number) > 0)) fail(`${at}.maxLength`, `must be a positive integer, got ${JSON.stringify(maxLength)}`);
        if (spec.multiline !== undefined && typeof spec.multiline !== "boolean") fail(`${at}.multiline`, `must be true or false, got ${JSON.stringify(spec.multiline)}`);
        if (typeof v !== "string") fail(at, `is text, but defaultProps.${key} is ${got}`);
        if (maxLength !== undefined && (v as string).length > (maxLength as number)) fail(at, `holds at most ${maxLength} characters, but defaultProps.${key} has ${(v as string).length}`);
        break;
      }
    }
    out[key] = { ...(spec as unknown as Control), cost: (spec.cost as ControlCost | undefined) ?? "live" };
  }
  return out;
}

function isJsonLike(v: unknown, seen: Set<unknown>): boolean {
  if (v === null || typeof v === "string" || typeof v === "boolean") return true;
  if (typeof v === "number") return Number.isFinite(v);
  if (typeof v !== "object") return false;
  if (seen.has(v)) return false;
  seen.add(v);
  const ok = Array.isArray(v)
    ? v.every((x) => isJsonLike(x, seen))
    : Object.getPrototypeOf(v) === Object.prototype && Object.values(v).every((x) => isJsonLike(x, seen));
  seen.delete(v);
  return ok;
}

function validate(props: CompositionProps<any>, taken: readonly RegisteredComposition[]): RegisteredComposition {
  const name = typeof props.id === "string" && props.id !== "" ? `Composition "${props.id}"` : "A Composition";
  const fail = (field: string, why: string): never => {
    throw new Error(`${name}: ${field} ${why}`);
  };
  if (typeof props.id !== "string" || !ID_PATTERN.test(props.id)) {
    fail("id", `must be letters, digits and hyphens, got ${JSON.stringify(props.id)}`);
  }
  if (taken.some((c) => c.id === props.id)) fail("id", "is already used by another composition in this root");
  if (typeof props.component !== "function" && (typeof props.component !== "object" || props.component === null)) {
    fail("component", "must be a React component");
  }
  for (const field of ["width", "height", "durationInFrames"] as const) {
    const v = props[field];
    if (!Number.isInteger(v) || v <= 0) fail(field, `must be a positive integer, got ${v}`);
  }
  if (typeof props.fps !== "number" || !(props.fps > 0) || !Number.isFinite(props.fps)) {
    fail("fps", `must be a positive number, got ${props.fps}`);
  }
  const defaultProps = props.defaultProps ?? {};
  if (!isJsonLike(defaultProps, new Set()) || Array.isArray(defaultProps)) {
    fail("defaultProps", "must be an object of JSON-like values (no functions, class instances or cycles)");
  }
  const segments = props.edit?.segments;
  if (segments !== undefined) {
    if (!Array.isArray(segments) || segments.length === 0) fail("edit.segments", "must be a non-empty list of [from, to] frame ranges");
    for (const seg of segments) {
      const ok = Array.isArray(seg) && seg.length === 2 && Number.isInteger(seg[0]) && Number.isInteger(seg[1]) && seg[0] >= 0 && seg[1] > seg[0] && seg[1] <= props.durationInFrames;
      if (!ok) fail("edit.segments", `has ${JSON.stringify(seg)}: each is [from, to) with 0 <= from < to <= ${props.durationInFrames}`);
    }
  }
  const scale = props.edit?.scale;
  if (scale !== undefined && !(typeof scale === "number" && scale > 0 && scale <= 1)) fail("edit.scale", `must be a number above 0 and at most 1, got ${scale}`);
  const [w, h] = scale === undefined || scale === 1 ? [props.width, props.height] : [Math.max(1, Math.round(props.width * scale)), Math.max(1, Math.round(props.height * scale))];
  const controls = validControls(props.controls, defaultProps as Record<string, unknown>, fail);
  const cut = segments ? edited(props.component, segments) : props.component;
  return {
    id: props.id,
    component: w === props.width && h === props.height ? cut : scaled(cut, props.width, props.height, w, h),
    width: w,
    height: h,
    fps: props.fps,
    durationInFrames: segments ? segments.reduce((n, [a, b]) => n + b - a, 0) : props.durationInFrames,
    defaultProps,
    controls,
  };
}

/** The component played through an edit's segments: each one a Sequence at its place in the cut, holding the
 *  composition shifted so its own frame `from` lands there. Sound follows, since it's placed by the same Sequences. */
function edited(Component: ComponentType<any>, segments: readonly (readonly [number, number])[]): ComponentType<any> {
  const at: number[] = [];
  segments.reduce((n, [a, b]) => (at.push(n), n + b - a), 0);
  function Edited(props: Record<string, unknown>) {
    return (
      <>
        {segments.map(([a, b], i) => (
          <Sequence key={i} from={at[i]} durationInFrames={b - a} layout="none">
            <Sequence from={-a} layout="none">
              <Component {...props} />
            </Sequence>
          </Sequence>
        ))}
      </>
    );
  }
  Edited.displayName = `Edited(${Component.displayName ?? Component.name ?? "Composition"})`;
  return Edited;
}

/** The component laid out at its own size (`width` by `height`, which useVideoConfig reports inside it) in a box
 *  drawn at `w` by `h`: an edit's scale. */
function scaled(Component: ComponentType<any>, width: number, height: number, w: number, h: number): ComponentType<any> {
  const style = { position: "absolute", left: 0, top: 0, width, height, transform: `scale(${w / width}, ${h / height})`, transformOrigin: "0 0" } as const;
  function Scaled(props: Record<string, unknown>) {
    return (
      <div style={style}>
        <Sequence layout="none" width={width} height={height}>
          <Component {...props} />
        </Sequence>
      </div>
    );
  }
  Scaled.displayName = `Scaled(${Component.displayName ?? Component.name ?? "Composition"})`;
  return Scaled;
}

/** A control's value as the frame is drawn (docs/realtime.feature RT4c): the composition's prop of that name, the
 *  value a person set on the live stage or, in a render, its defaultProps. For a part deep in the tree, so the value
 *  needn't be passed down as a prop; the root component can read its props as any component does. */
export function useControl<T = unknown>(name: string): T {
  const { defaultProps } = useVideoConfig();
  if (!Object.prototype.hasOwnProperty.call(defaultProps, name)) {
    const keys = Object.keys(defaultProps);
    throw new Error(`useControl("${name}"): the composition has no prop "${name}" (its defaultProps has ${keys.length ? keys.join(", ") : "none"})`);
  }
  return defaultProps[name] as T;
}

/** Declares a composition inside the root. It draws nothing itself; a renderer finds it through registerRoot. */
export function Composition<P extends Record<string, unknown>>(_props: CompositionProps<P>) {
  return null;
}

// Components of this package whose rendering reads the clock (they use hooks), so the walk never calls them.
const HOOKED: ReadonlySet<unknown> = new Set<unknown>([
  Sequence, Series, Series.Sequence, Loop, Freeze, Region, Video, Audio,
]);

// A call to a hook in a component's source: `use(` or `useSomething(` (also after a dot, as bundlers write it).
const HOOK_CALL = /\buse(?:[A-Z][\w$]*)?\s*\(/;

const CLOSE: Record<string, string> = { "(": ")", "[": "]", "{": "}" };
// what a `/` follows when it starts a regex literal rather than dividing
const REGEX_AFTER = /[(,=:[!&|?{};+\-*%<>~^]/;
const REGEX_WORD = /\b(?:return|typeof|case|do|else|in|of|new|delete|void|throw|yield|await)$/;

/** Where the string, template literal, comment or regex literal at `i` ends (the index after it); -1 when it's code. */
function opaqueEnd(s: string, i: number): number {
  const c = s[i];
  if (c === '"' || c === "'") {
    for (let j = i + 1; j < s.length; j++) {
      if (s[j] === "\\") j++;
      else if (s[j] === c || s[j] === "\n") return j + 1;
    }
    return s.length;
  }
  if (c === "`") {
    for (let j = i + 1; j < s.length; j++) {
      if (s[j] === "\\") j++;
      else if (s[j] === "`") return j + 1;
      else if (s[j] === "$" && s[j + 1] === "{") j = closeOf(s, j + 1) - 1;
    }
    return s.length;
  }
  if (c !== "/") return -1;
  if (s[i + 1] === "/") {
    const e = s.indexOf("\n", i);
    return e < 0 ? s.length : e + 1;
  }
  if (s[i + 1] === "*") {
    const e = s.indexOf("*/", i + 2);
    return e < 0 ? s.length : e + 2;
  }
  let k = i - 1;
  while (k >= 0 && /\s/.test(s[k])) k--;
  if (k >= 0 && !REGEX_AFTER.test(s[k]) && !REGEX_WORD.test(s.slice(Math.max(0, k - 7), k + 1))) return -1; // division
  let inClass = false;
  for (let j = i + 1; j < s.length; j++) {
    if (s[j] === "\\") j++;
    else if (s[j] === "[") inClass = true;
    else if (s[j] === "]") inClass = false;
    else if (s[j] === "/" && !inClass) {
      let end = j + 1;
      while (end < s.length && /[a-z]/i.test(s[end])) end++;
      return end;
    }
    else if (s[j] === "\n") return j + 1;
  }
  return s.length;
}

/** The index after the bracket that closes the one at `i`, past strings, comments and the like. */
function closeOf(s: string, i: number): number {
  let depth = 0;
  for (let j = i; j < s.length; ) {
    const e = opaqueEnd(s, j);
    if (e >= 0) {
      j = e;
      continue;
    }
    if (CLOSE[s[j]]) depth++;
    else if (s[j] === ")" || s[j] === "]" || s[j] === "}") {
      depth--;
      if (depth === 0) return j + 1;
    }
    j++;
  }
  return s.length;
}

/** Where an arrow function's expression body starting at `i` ends: at a `,`, `;` or closing bracket outside brackets. */
function expressionEnd(s: string, i: number): number {
  for (let j = i; j < s.length; ) {
    const e = opaqueEnd(s, j);
    if (e >= 0) {
      j = e;
      continue;
    }
    if (CLOSE[s[j]]) {
      j = closeOf(s, j);
      continue;
    }
    if (s[j] === "," || s[j] === ";" || s[j] === ")" || s[j] === "]" || s[j] === "}") return j;
    j++;
  }
  return s.length;
}

/**
 * A function's source without the bodies of the functions written inside it: a component given inline
 * (`component={() => ...}`), a render prop or a callback runs when React or the caller runs it, not when this function
 * is called, so a hook there isn't this function's. Each inner `=>` body and `function` body becomes empty.
 */
function ownSource(s: string): string {
  // this function's own body: after its first `=>`, or inside its first `{`, outside its parameters' brackets
  let body = -1;
  for (let j = 0; j < s.length && body < 0; ) {
    const e = opaqueEnd(s, j);
    if (e >= 0) j = e;
    else if (s.startsWith("=>", j)) body = j + 2;
    else if (s[j] === "{") body = j + 1;
    else if (s[j] === "(" || s[j] === "[") j = closeOf(s, j);
    else j++;
  }
  if (body < 0) return s;
  let out = s.slice(0, body);
  for (let j = body; j < s.length; ) {
    const e = opaqueEnd(s, j);
    if (e >= 0) {
      out += s.slice(j, e);
      j = e;
      continue;
    }
    if (s.startsWith("=>", j)) {
      let k = j + 2;
      while (k < s.length && /\s/.test(s[k])) k++;
      out += "=>0";
      j = s[k] === "{" ? closeOf(s, k) : expressionEnd(s, k);
      continue;
    }
    if (s.startsWith("function", j) && !/[\w$]/.test(s[j - 1] ?? "") && !/[\w$]/.test(s[j + 8] ?? "")) {
      const open = s.indexOf("(", j);
      let k = open < 0 ? s.length : closeOf(s, open);
      while (k < s.length && /\s/.test(s[k])) k++;
      if (s[k] === "{") {
        out += "function(){}";
        j = closeOf(s, k);
        continue;
      }
    }
    out += s[j];
    j++;
  }
  return out;
}

function looksHooked(fn: Function): boolean {
  if (HOOKED.has(fn)) return true;
  let source = "";
  try {
    source = ownSource(Function.prototype.toString.call(fn));
  } catch {
    return true; // can't read it: don't risk running it
  }
  return HOOK_CALL.test(source) || /\bword\s*\(/.test(source);
}

/** Calls a component that shows no hooks, quietly; undefined when it fails (then it's treated as content). */
function expand(fn: (p: unknown) => ReactNode, props: unknown): { ok: true; node: ReactNode } | { ok: false } {
  const quiet = console.error;
  console.error = () => {};
  try {
    return { ok: true, node: fn(props) };
  } catch {
    return { ok: false };
  } finally {
    console.error = quiet;
  }
}

interface Walk {
  found: RegisteredComposition[];
  /** the latest end among the top-level Sequences with a finite duration */
  latestEnd: number;
}

/** The end frame of a top-level Sequence (or Series), when its duration is finite. */
function sequenceEnd(type: unknown, props: Record<string, unknown>): number | undefined {
  if (type === Sequence) {
    const from = typeof props.from === "number" ? props.from : 0;
    const d = props.durationInFrames;
    return typeof d === "number" && Number.isFinite(d) ? from + d : undefined;
  }
  if (type === Series) {
    let end = 0;
    for (const child of Children.toArray(props.children as ReactNode)) {
      if (!isValidElement(child)) continue;
      const p = child.props as { durationInFrames?: unknown; offset?: unknown };
      const d = p.durationInFrames;
      if (typeof d !== "number" || !Number.isFinite(d)) return undefined;
      end += (typeof p.offset === "number" ? p.offset : 0) + d;
    }
    return end;
  }
  return undefined;
}

/**
 * Finds the Compositions in a root's element tree without a renderer (the main entry must run in the engine's V8,
 * which has no server renderer). It never runs a component that uses hooks: there it stops and treats the element as
 * content. Other function components are expanded by calling them; fragments, providers and plain elements are
 * searched through their children.
 */
function collect(node: ReactNode, walk: Walk, depth: number, topLevel: boolean): void {
  if (depth > 200) throw new Error("registerRoot: the root's element tree is too deep to search for Compositions");
  if (node === null || node === undefined || typeof node === "boolean") return;
  if (Array.isArray(node)) {
    for (const child of node) collect(child, walk, depth + 1, topLevel);
    return;
  }
  if (!isValidElement(node)) return;
  const props = node.props as Record<string, unknown> & { children?: ReactNode };
  const type = node.type as unknown;
  if (type === Composition) {
    walk.found.push(validate(props as unknown as CompositionProps<any>, walk.found));
    return;
  }
  const end = sequenceEnd(type, props);
  if (end !== undefined && topLevel) walk.latestEnd = Math.max(walk.latestEnd, end);
  if (typeof type === "function") {
    const isClass = !!(type as { prototype?: { isReactComponent?: unknown } }).prototype?.isReactComponent;
    if (isClass || looksHooked(type)) return; // content
    const out = expand(type as (p: unknown) => ReactNode, props);
    if (out.ok) collect(out.node, walk, depth + 1, topLevel);
    return;
  }
  collect(props.children, walk, depth + 1, topLevel);
}

/** What a root with no Composition registers as: itself, at these settings. */
const IMPLICIT = { id: "main", fps: 30, width: 1080, height: 1920, durationInFrames: 300 } as const;

let root: ComponentType | null = null;
let compositions: RegisteredComposition[] = [];

/** Names the root component the renderer starts from, and records its compositions. Call it once. */
export function registerRoot(given: ComponentType | ReactElement): void {
  if (root) throw new Error("registerRoot was already called; a program has exactly one root");
  // registerRoot(<Composition ... />) is read as registerRoot(() => <Composition ... />), so an element given where the
  // component goes isn't rendered as a type (React's #130)
  const Root: ComponentType = isValidElement(given) ? () => given : given;
  if (typeof Root !== "function" && (typeof Root !== "object" || Root === null)) {
    throw new TypeError("registerRoot: give it the root component");
  }
  const walk: Walk = { found: [], latestEnd: 0 };
  // the root itself is always looked inside, unless it reads the clock (then it's all content)
  if (!looksHooked(Root)) {
    const out = typeof Root === "function" && !(Root as { prototype?: { isReactComponent?: unknown } }).prototype?.isReactComponent
      ? expand(Root as (p: unknown) => ReactNode, {})
      : { ok: false as const };
    if (out.ok) collect(out.node, walk, 0, true);
  }
  if (walk.found.length === 0) {
    walk.found.push({
      ...IMPLICIT,
      durationInFrames: walk.latestEnd > 0 ? Math.ceil(walk.latestEnd) : IMPLICIT.durationInFrames,
      component: Root,
      defaultProps: {},
      controls: {},
    });
  }
  root = Root;
  compositions = walk.found;
}

/** The compositions the registered root declares, in order. */
export function getCompositions(): RegisteredComposition[] {
  return compositions.slice();
}

/** Forgets the registered root and its compositions (for tests; exported from the test entry only). */
export function clearRegistry(): void {
  root = null;
  compositions = [];
}
