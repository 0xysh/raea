// three's PMREMGenerator filters an environment on the GPU: it renders a scene (RoomEnvironment, three's own recipe for
// soft studio light) or a panorama into a cube map with the renderer, which in raea is a stand-in that never draws
// (session.ts). raea filters an environment itself, from a panorama (lights.ts skyOf, raea's <Environment>), so the
// generator hands back what raea can read: fromEquirectangular of a panorama raea loads is that panorama, lighting the
// scene as scene.environment; fromScene and fromCubemap are left out, the scene drawn without them and a warning saying
// so (skyOf). fromScene threw inside the stand-in renderer and took the whole <R3F> scene with it (#131).
import * as THREE from "three";
import { srcOf } from "./texture";

/** what a stand-in environment was made by, when raea can't light by it: skyOf names it in its warning */
export const LEFT_OUT = "raeaLeftOut";

function standIn(src: string | undefined, by: string): THREE.WebGLRenderTarget {
  const t = new THREE.WebGLRenderTarget(1, 1);
  t.texture.mapping = THREE.CubeUVReflectionMapping;
  if (src) t.texture.userData.src = src;
  else t.texture.userData[LEFT_OUT] = by;
  return t;
}

let done = false;

/** PMREMGenerator's makers as stand-ins, once */
export function standInPMREM() {
  if (done) return;
  done = true;
  const pm = THREE.PMREMGenerator.prototype as unknown as Record<string, unknown>;
  pm.fromScene = () => standIn(undefined, "PMREMGenerator.fromScene");
  pm.fromSceneAsync = async () => standIn(undefined, "PMREMGenerator.fromScene");
  pm.fromEquirectangular = (texture: THREE.Texture) => standIn(srcOf(texture), "PMREMGenerator.fromEquirectangular");
  pm.fromEquirectangularAsync = async (texture: THREE.Texture) => standIn(srcOf(texture), "PMREMGenerator.fromEquirectangular");
  pm.fromCubemap = () => standIn(undefined, "PMREMGenerator.fromCubemap");
  pm.fromCubemapAsync = async () => standIn(undefined, "PMREMGenerator.fromCubemap");
  pm.compileCubemapShader = () => {};
  pm.compileEquirectangularShader = () => {};
}
