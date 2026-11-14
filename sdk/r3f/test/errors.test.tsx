// What an R3F scene throws is the composition's error, and three's PMREMGenerator doesn't take the scene with it
// (#131: any error inside <R3F> was swallowed, the scene drew nothing and the render ended with 0 errors).
import { describe, expect, test } from "bun:test";
import { useThree } from "@react-three/fiber";
import { useEffect, useMemo, type ReactNode } from "react";
import * as THREE from "three";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { convert } from "../src/input/scene";
import { end, stepTo } from "../src/input/session";
import { standInPMREM } from "../src/input/pmrem";
import type { Part } from "../src/parts";

standInPMREM();

const flat = (parts: Part[]): Part[] => parts.flatMap((p) => [p, ...flat(p.children ?? [])]);
let n = 0;
function partsOf(scene: (i: number) => ReactNode, key = `e${n++}`) {
  const s = stepTo(key, 0, 30, 400, 400, {}, scene);
  const st = s.state as unknown as { scene: THREE.Scene; camera: THREE.Camera; gl: THREE.WebGLRenderer };
  return flat(convert(st.scene, st.camera, st.gl, 400).parts);
}
const Box = () => (
  <mesh>
    <boxGeometry />
    <meshBasicMaterial color="red" />
  </mesh>
);

describe("an error inside the scene", () => {
  test("thrown while rendering, it's thrown out of the frame with its message, and the next frame starts again", () => {
    const Boom = (): null => {
      throw new Error("mesh threw");
    };
    expect(() => partsOf(() => (<><Box /><Boom /></>), "boom")).toThrow("mesh threw");
    // the same key without the throw draws: the failed root was ended, not kept
    expect(partsOf(() => <Box />, "boom").some((p) => p.type === "Mesh")).toBe(true);
    end("boom");
  });

  test("thrown in an effect, it's thrown out too", () => {
    const Late = () => {
      useEffect(() => {
        throw new Error("effect threw");
      }, []);
      return null;
    };
    expect(() => partsOf(() => (<><Box /><Late /></>))).toThrow("effect threw");
  });
});

describe("three's PMREMGenerator", () => {
  test("fromScene (RoomEnvironment) leaves the environment out with a warning and the scene draws", () => {
    const warned: string[] = [];
    const warn = console.warn;
    console.warn = (m: string) => void warned.push(String(m));
    try {
      const Studio = () => {
        const { gl, scene } = useThree();
        useMemo(() => {
          const pm = new THREE.PMREMGenerator(gl);
          scene.environment = pm.fromScene(new RoomEnvironment(), 0.04).texture;
        }, [gl, scene]);
        return null;
      };
      const parts = partsOf(() => (<><Studio /><Box /></>));
      expect(parts.some((p) => p.type === "Mesh")).toBe(true);
      expect(parts.some((p) => p.type === "Environment")).toBe(false);
    } finally {
      console.warn = warn;
    }
    expect(warned.some((w) => w.includes("PMREMGenerator.fromScene"))).toBe(true);
  });

  test("fromEquirectangular of a panorama lights the scene by that panorama", () => {
    const Sky = () => {
      const { gl, scene } = useThree();
      useMemo(() => {
        const hdr = new THREE.Texture();
        hdr.userData.src = "media/studio.hdr";
        scene.environment = new THREE.PMREMGenerator(gl).fromEquirectangular(hdr).texture;
      }, [gl, scene]);
      return null;
    };
    const env = partsOf(() => (<><Sky /><Box /></>)).find((p) => p.type === "Environment");
    expect(env?.props.src).toBe("media/studio.hdr");
  });
});
