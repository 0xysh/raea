// React Three Fiber run headless, a frame at a time: its own root on a stand-in canvas and renderer (raea draws; three
// never does), with frameloop "never" so nothing ticks on its own. Frame f of the composition is R3F's clock at f/fps
// seconds: the scene is rendered with frame f's children, then advance() runs its useFrame callbacks once with the
// step since the last frame. Going backwards (a seek, a scrub) starts the root again from frame 0 and steps forward, so
// a frame's scene is the same whichever frame came before it: a render equals its play (raea's first rule).
import * as THREE from "three";
import { advance, createRoot, extend, flushSync, type ReconcilerRoot, type RenderProps, type RootState } from "@react-three/fiber";
import { Component, createElement, type ReactNode } from "react";
import { standInRenderer } from "./renderer";
import { nextRandom, seedOf } from "./seed";

export { standInRenderer, warnOnce } from "./renderer";

extend(THREE as unknown as Parameters<typeof extend>[0]);

function standInCanvas(width: number, height: number): HTMLCanvasElement {
  const noop = () => {};
  return { style: {}, width, height, addEventListener: noop, removeEventListener: noop, getBoundingClientRect: () => ({ left: 0, top: 0, width, height, x: 0, y: 0, right: width, bottom: height }) } as unknown as HTMLCanvasElement;
}

export interface Session {
  root: ReconcilerRoot<HTMLCanvasElement>;
  state: RootState;
  /** the last frame stepped */
  frame: number;
  /** the settings it was configured with, as JSON: a change starts it again */
  config: string;
  /** its useFrame callbacks when it last stepped: more on a redraw means part of it mounted late (it suspended) */
  subs: number;
  /** its Math.random's state: seeded as it starts, so a root started again draws the same numbers */
  rand: number;
}

/** Math.random as `s`'s own generator (mulberry32) while `fn` runs: drei's <Float> phase, <Sparkles>, three's uuids
 *  come out the same each time the root is stepped from frame 0, so a render after a seek equals its play */
function seeded<T>(s: Session, fn: () => T): T {
  const was = Math.random;
  Math.random = () => nextRandom(s);
  try {
    return fn();
  } finally {
    Math.random = was;
  }
}

const sessions = new Map<string, Session>();

/** The scene under an error boundary of raea's: what it throws (rendering, in an effect, in a ref) is handed to `on`,
 *  and it draws nothing after. A promise thrown (a load suspending) isn't an error and passes by it, as before. */
class Caught extends Component<{ on: (error: unknown) => void; children?: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch(error: unknown) {
    this.props.on(error);
  }
  render() {
    return this.state.failed ? null : this.props.children;
  }
}

export type Config = Omit<RenderProps<HTMLCanvasElement>, "gl" | "frameloop" | "size" | "events" | "dpr">;

function start(key: string, width: number, height: number, config: Config, json: string): Session {
  const root = createRoot(standInCanvas(width, height));
  let state: RootState | undefined;
  void root.configure({ ...config, gl: standInRenderer(), frameloop: "never", dpr: 1, size: { width, height, top: 0, left: 0 }, events: undefined, onCreated: (s) => { state = s; config.onCreated?.(s); } });
  const s: Session = { root, state: state as unknown as RootState, frame: -1, config: json, subs: 0, rand: seedOf(key) };
  sessions.set(key, s);
  return s;
}

/** The root `key` at local frame `frame` (fps a second), each step rendering `scene(i)` first. */
export function stepTo(key: string, frame: number, fps: number, width: number, height: number, config: Config, scene: (i: number) => ReactNode): Session {
  const json = JSON.stringify({ width, height, config }, (_k, v) => (typeof v === "function" ? String(v) : v instanceof THREE.Object3D ? v.uuid : v));
  let s = sessions.get(key);
  if (s && (frame < s.frame || s.config !== json)) {
    end(key);
    s = undefined;
  }
  s ??= start(key, width, height, config, json);
  const live = s;
  const render = (i: number) => {
    let store: ReturnType<typeof live.root.render> | undefined;
    let failed: { error: unknown } | undefined;
    seeded(live, () => flushSync(() => { store = live.root.render(createElement(Caught, { on: (error: unknown) => void (failed ??= { error }) }, scene(i))); }));
    if (store) live.state = store.getState();
    // thrown out of the <R3F> that's drawing it, so the composition throws as it would outside R3F: the render is
    // refused, naming the error. R3F's own root only reports it (reportError, else console.error: a stack without its
    // message) and draws nothing, and the render ended with 0 errors (#131). The root starts again next time
    if (failed) {
      end(key);
      throw failed.error;
    }
  };
  // the same frame drawn again (a live replay): its children may have changed, its clock hasn't. When parts of it
  // mounted only now (a model, a font: what suspended while the frames were stepped, loaded since), their useFrame
  // never ran: the root starts again, and with the loads cached it mounts at once and steps from frame 0 as if they
  // had been there all along (as three's own page waits for its loads and then draws the frame)
  if (frame === s.frame) {
    render(frame);
    if (subsOf(live) > live.subs && !late.has(key)) {
      late.add(key);
      end(key);
      return stepTo(key, frame, fps, width, height, config, scene);
    }
  }
  for (let i = s.frame + 1; i <= frame; i++) {
    // what it draws into targets is this step's (R3F23): the last step's passes go
    (live.state?.gl as unknown as { __raea?: { step?: () => void } } | undefined)?.__raea?.step?.();
    render(i);
    seeded(live, () => advance(i / fps, true, live.state));
    live.frame = i;
    live.subs = subsOf(live);
  }
  return live;
}

/** roots started again for parts that mounted late: once each, so one whose callbacks keep growing can't loop */
const late = new Set<string>();

const subsOf = (s: Session) => (s.state as unknown as { internal?: { subscribers?: unknown[] } }).internal?.subscribers?.length ?? 0;

/** the root `key` unmounted (the <R3F> left the tree, or starts again) */
export function end(key: string) {
  const s = sessions.get(key);
  if (!s) return;
  s.root.unmount();
  sessions.delete(key);
}
