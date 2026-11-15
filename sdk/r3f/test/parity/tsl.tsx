// The TSL parity cases (docs/r3f.feature R3F21, R3F22): node materials three/webgpu draws in Chrome and raea's engine
// draws from three's own WGSL (src/input/tsl.ts, tsl-lit.ts). A graph's motion comes from a uniform R3F's clock sets in
// useFrame, never TSL's `time` (three's renderer counts that from performance.now(), so Chrome's frame would differ).
import { useFrame } from "@react-three/fiber";
import { MeshStandardNodeMaterial } from "three/webgpu";
import * as TSLTyped from "three/tsl";
import type { InputCase } from "./fixtures";

// untyped: @types/three's chained TSL types make tsc take minutes and gigabytes
const { color, mix, mx_noise_float, normalLocal, positionLocal, uniform } = TSLTyped as unknown as Record<string, (...a: unknown[]) => any> & { normalLocal: any; positionLocal: any };

/** three's WebGPURenderer for a TSL scene: its WebGL 2 backend (headless Chrome may have no WebGPU) */
async function nodeRenderer(props: object) {
  const { WebGPURenderer } = await import("three/webgpu");
  const r = new WebGPURenderer({ ...props, forceWebGL: true, antialias: true });
  await r.init();
  return r;
}

// the noise blob people post: an icosahedron its positionNode pushes out along the normal by 3D noise, coloured by the
// same noise, lit by the scene's lights (a MeshStandardNodeMaterial with nodes of its own: R3F21)
const blob = (() => {
  const t = uniform(0);
  const m = new MeshStandardNodeMaterial({ roughness: 0.4, metalness: 0.1 });
  const n = mx_noise_float(positionLocal.mul(1.6).add(t.mul(0.5)));
  m.positionNode = positionLocal.add(normalLocal.mul(n.mul(0.3)));
  m.colorNode = mix(color("#ff7744"), color("#3355ff"), n.mul(0.5).add(0.5));
  return { m, t };
})();

function Blob() {
  useFrame(({ clock }) => void (blob.t.value = clock.elapsedTime));
  return (
    <mesh material={blob.m} rotation={[0.2, 0.3, 0]}>
      <icosahedronGeometry args={[1, 32]} />
    </mesh>
  );
}

export const TSL_INPUTS: Record<string, InputCase> = {
  "tsl-lit": {
    canvas: { camera: { position: [0, 0.4, 4], fov: 45 }, gl: nodeRenderer },
    frame: 12,
    scene: () => (
      <>
        <color attach="background" args={["#14161c"]} />
        <ambientLight intensity={0.3} />
        <directionalLight position={[3, 5, 2]} intensity={2.5} />
        <Blob />
      </>
    ),
  },
};
