// The parity cases for render targets an app draws into itself, drawn on raea's GPU (docs/r3f.feature R3F23,
// src/input/passes.ts): each drawn by raea and by three.js in Chrome.
import { useFBO } from "@react-three/drei";
import { useFrame } from "@react-three/fiber";
import { useMemo } from "react";
import * as THREE from "three";
import type { InputCase } from "./fixtures";

/** the 11-15 audit's fbo probe: a lit, turning knot drawn into a useFBO from its own scene and camera, every frame,
 *  shown on a plane in the screen's scene (0.1.4 drew the plane white: the target was never filled) */
function FboPlane() {
  const fbo = useFBO(256, 256);
  const inner = useMemo(() => {
    const s = new THREE.Scene();
    s.background = new THREE.Color("#ff3366");
    const knot = new THREE.Mesh(new THREE.TorusKnotGeometry(0.6, 0.2, 128, 16), new THREE.MeshStandardMaterial({ color: "#f2c14e", roughness: 0.35 }));
    const sun = new THREE.DirectionalLight("#ffffff", 2.5);
    sun.position.set(2, 3, 2);
    s.add(knot, sun, new THREE.AmbientLight("#ffffff", 0.4));
    const cam = new THREE.PerspectiveCamera(50, 1, 0.1, 10);
    cam.position.z = 3;
    return { s, knot, cam };
  }, []);
  useFrame(({ gl, clock }) => {
    inner.knot.rotation.set(0.4, clock.elapsedTime * 1.5, 0);
    gl.setRenderTarget(fbo);
    gl.render(inner.s, inner.cam);
    gl.setRenderTarget(null);
  });
  return (
    <>
      <color attach="background" args={["#222222"]} />
      <mesh rotation={[0, -0.3, 0]}>
        <planeGeometry args={[3, 3]} />
        <meshBasicMaterial map={fbo.texture} />
      </mesh>
    </>
  );
}

export const TARGET_INPUTS: Record<string, InputCase> = {
  fbo: {
    canvas: { camera: { position: [0, 0, 4], fov: 45 } },
    frame: 12,
    scene: () => <FboPlane />,
  },
};
