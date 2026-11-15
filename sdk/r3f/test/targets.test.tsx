// Render targets drawn on raea's GPU (docs/r3f.feature R3F23): a pass an app draws itself (gl.setRenderTarget(t);
// gl.render(scene, camera)) is its scene's parts, as they were when it was drawn, on a render layer of its own, and a
// camera part drawing that layer into the target's image; a material reading the target's texture names that image.
import { describe, expect, test } from "bun:test";
import { createPortal, useFrame } from "@react-three/fiber";
import { useMemo, type ReactNode } from "react";
import * as THREE from "three";
import { convert } from "../src/input/scene";
import { end, stepTo } from "../src/input/session";
import type { Part } from "../src/parts";

const flat = (parts: Part[]): Part[] => parts.flatMap((p) => [p, ...flat(p.children ?? [])]);
let n = 0;
function partsOf(scene: () => ReactNode, frame = 0): Part[] {
  const key = `targets${n++}`;
  const s = stepTo(key, frame, 30, 400, 400, {}, scene);
  const st = s.state as unknown as { scene: THREE.Scene; camera: THREE.Camera; gl: THREE.WebGLRenderer };
  const out = flat(convert(st.scene, st.camera, st.gl, 400).parts);
  end(key);
  return out;
}

/** the audit's fbo probe: a knot drawn into a target (its own scene, through a portal) shown on a plane */
function Fbo() {
  const target = useMemo(() => new THREE.WebGLRenderTarget(128, 64), []);
  const scene = useMemo(() => Object.assign(new THREE.Scene(), { background: new THREE.Color("#ff8800") }), []);
  const cam = useMemo(() => {
    const c = new THREE.PerspectiveCamera(50, 2, 0.1, 100);
    c.position.set(0, 0, 4);
    return c;
  }, []);
  useFrame(({ gl }) => {
    gl.setRenderTarget(target);
    gl.render(scene, cam);
    gl.setRenderTarget(null);
  });
  return (
    <>
      {createPortal(<mesh name="knot"><torusKnotGeometry args={[0.6, 0.2, 32, 8]} /><meshBasicMaterial color="#2266ff" /></mesh>, scene)}
      <mesh name="screen"><planeGeometry args={[2, 1]} /><meshBasicMaterial map={target.texture} /></mesh>
    </>
  );
}

describe("a render into a target, drawn by raea's engine", () => {
  test("is a camera into the target's image, before the screen's, drawing its scene's parts on its own layer", () => {
    const out = partsOf(() => <Fbo />);
    const cam = out.find((p) => typeof p.props.target === "string")!;
    expect(cam.type).toBe("PerspectiveCamera");
    expect(cam.props.target).toMatch(/^render:[^:]+:128x64$/);
    expect(cam.props.order as number).toBeLessThan(0);
    // cleared to the pass scene's background, opaque
    expect(cam.props.bg).toBe("#ff8800ff");
    const layer = (cam.props.layers as number[])[0];
    expect(layer).toBeGreaterThan(31);
    const knot = out.find((p) => p.type === "Mesh" && String(p.props.name).includes("knot"))!;
    expect(knot.props.layers).toEqual([layer]);
    expect(String(knot.props.name).startsWith("pass/")).toBe(true);
    // the screen's plane is on no pass layer and reads the image the camera draws, by the same name
    const screen = out.find((p) => p.type === "Mesh" && String(p.props.name).includes("screen"))!;
    expect(screen.props.layers).toBeUndefined();
    const map = flat(screen.children ?? []).map((c) => c.props.map).find((m) => typeof m === "string");
    expect(map).toBe(cam.props.target as string);
  });

  test("is the frame's own: a render of frame 3 has the one pass frame 3 drew, not one for each frame stepped", () => {
    const out = partsOf(() => <Fbo />, 3);
    expect(out.filter((p) => typeof p.props.target === "string").length).toBe(1);
    expect(out.filter((p) => p.type === "Mesh" && String(p.props.name).includes("knot")).length).toBe(1);
  });

  test("takes its scene as it was when drawn: a quad drawn twice with another uniform is two passes, each its own", () => {
    function Twice() {
      const a = useMemo(() => new THREE.WebGLRenderTarget(32, 32, { type: THREE.HalfFloatType }), []);
      const b = useMemo(() => new THREE.WebGLRenderTarget(32, 32, { type: THREE.HalfFloatType }), []);
      const mat = useMemo(() => new THREE.ShaderMaterial({ uniforms: { k: { value: 0 } }, vertexShader: "void main() { gl_Position = vec4(position, 1.0); }", fragmentShader: "uniform float k; void main() { gl_FragColor = vec4(k); }" }), []);
      const quad = useMemo(() => new THREE.Mesh(new THREE.PlaneGeometry(2, 2), mat), [mat]);
      const cam = useMemo(() => new THREE.Camera(), []);
      useFrame(({ gl }) => {
        mat.uniforms.k.value = 0.25;
        gl.setRenderTarget(a);
        gl.render(quad, cam);
        mat.uniforms.k.value = 0.75;
        gl.setRenderTarget(b);
        gl.render(quad, cam);
        gl.setRenderTarget(null);
      });
      return null;
    }
    const out = partsOf(() => <Twice />);
    const cams = out.filter((p) => typeof p.props.target === "string");
    expect(cams.length).toBe(2);
    // a float target's image is a float one, and a bare THREE.Camera's pass is drawn by an orthographic one over the quad
    for (const c of cams) {
      expect(c.props.target).toMatch(/:32x32:float$/);
      expect(c.type).toBe("OrthographicCamera");
    }
    expect(cams[0].props.order as number).toBeLessThan(cams[1].props.order as number);
    const k = (layer: number) => {
      const quad = out.find((p) => p.type === "Mesh" && (p.props.layers as number[] | undefined)?.[0] === layer)!;
      return flat(quad.children ?? []).map((c) => (c.props.uniforms as { k?: number } | undefined)?.k).find((v) => v !== undefined);
    };
    expect(k((cams[0].props.layers as number[])[0])).toBe(0.25);
    expect(k((cams[1].props.layers as number[])[0])).toBe(0.75);
  });
});
