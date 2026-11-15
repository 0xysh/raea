// The parity cases for postprocessing's effects raea draws as <FramePass>es (docs/r3f.feature R3F25, src/input/effects.ts):
// each a lit scene under a composer of them, drawn by raea and by three.js in Chrome.
import * as PP from "@react-three/postprocessing";
import { BlendFunction } from "postprocessing";
import type { InputCase } from "./fixtures";

const scene = (
  <>
    <color attach="background" args={["#1d2330"]} />
    <ambientLight intensity={0.5} />
    <directionalLight position={[2, 5, 1]} intensity={2.5} />
    <mesh position={[0, 0.6, 0]}>
      <sphereGeometry args={[0.6, 32, 16]} />
      <meshStandardMaterial color="#e05a2b" roughness={0.4} />
    </mesh>
    <mesh position={[1.4, 0.4, -0.6]}>
      <boxGeometry args={[0.8, 0.8, 0.8]} />
      <meshStandardMaterial color="#3a8fd8" roughness={0.5} />
    </mesh>
    <mesh rotation={[-Math.PI / 2, 0, 0]}>
      <planeGeometry args={[8, 8]} />
      <meshStandardMaterial color="#cfd6e0" roughness={0.9} />
    </mesh>
  </>
);

export const POST2_INPUTS: Record<string, InputCase> = {
  // sepia, scanlines overlaid, a grid at a third, the colours cut to 5 bits, then tone-mapped
  post2: {
    canvas: { camera: { position: [3, 2.5, 4], fov: 45 } },
    frame: 6,
    scene: () => (
      <>
        {scene}
        <PP.EffectComposer>
          <PP.Sepia intensity={0.8} />
          <PP.Scanline density={1.2} opacity={0.25} />
          <PP.Grid scale={1.5} lineWidth={0.05} blendFunction={BlendFunction.MULTIPLY} />
          <PP.ColorDepth bits={5} />
          <PP.ToneMapping mode={6} />
        </PP.EffectComposer>
      </>
    ),
  },
  // after the tone mapping (a dot screen's rgb*10-5 goes far below 0, which three's ACES curve turns bright and Bevy's
  // clamps): a dot screen at half, the colours averaged, screen-blended noise
  post3: {
    canvas: { camera: { position: [3, 2.5, 4], fov: 45 } },
    frame: 6,
    scene: () => (
      <>
        {scene}
        <PP.EffectComposer>
          <PP.ToneMapping mode={6} />
          <PP.DotScreen angle={Math.PI / 4} scale={0.9} opacity={0.5} />
          <PP.ColorAverage opacity={0.6} />
          <PP.Noise opacity={0.08} />
        </PP.EffectComposer>
      </>
    ),
  },
};
