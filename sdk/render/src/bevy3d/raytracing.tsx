// Ray-traced lighting: Bevy's Solari (docs/raytracing.feature RT3), on the camera it's inside.
import { BevyComponent } from "../3d";

export interface SolariLightingProps {
  /** ReSTIR: reuse light samples between pixels and frames (finer shadows, steadier in hard light; slower). Off by
   * default, as Bevy has it */
  restir?: boolean;
  /** how many times light bounces (Bevy's default when left out) */
  bounces?: number;
  /** start the lighting's history again (a cut) */
  reset?: boolean;
  /** draw the view at this fraction of its size a side (a third to 1) and let MetalFX's temporal scaler bring it up to
   * the full size, so a game traces fewer pixels: 0.67 traces 45% of them (RT9). On the Mac; elsewhere it draws at the
   * full size and raea says so. 1 (or left out): the full size */
  scale?: number;
}

/** Ray-traced direct and indirect light for the <Camera3d> it's inside (Bevy's SolariLighting): shadows, bounce
 * light and reflections traced against the scene, in place of shadow maps. Needs a GPU with ray queries (Apple
 * silicon on Metal, RTX on Vulkan); elsewhere the scene draws raster-lit and raea says so. On the Mac MetalFX's
 * temporal denoiser cleans it up (RT4; RAEA_SOLARI_DENOISE=0 shows the raw noise). A <Character> (skinned, morphed) stays raster-lit: Bevy 0.20's
 * Solari can't trace it (RT5). */
export function SolariLighting(p: SolariLightingProps = {}) {
  if (p.bounces !== undefined && !(Number.isInteger(p.bounces) && p.bounces >= 0)) throw new RangeError(`SolariLighting: bounces is a whole number, got ${p.bounces}`);
  if (p.scale !== undefined && !(p.scale >= 1 / 3 && p.scale <= 1)) throw new RangeError(`SolariLighting: scale is a third to 1 (of the view's size a side), got ${p.scale}`);
  const value: Record<string, unknown> = {};
  if (p.restir !== undefined) value.restir = p.restir;
  if (p.bounces !== undefined) value.max_bounces = p.bounces;
  if (p.reset !== undefined) value.reset = p.reset;
  const lighting = <BevyComponent type="bevy_solari::realtime::SolariLighting" value={value} />;
  if (p.scale === undefined || p.scale === 1) return lighting;
  return (
    <>
      {lighting}
      <BevyComponent type="raea_render_raytrace::RenderScale" value={{ scale: p.scale }} />
    </>
  );
}

export interface PathtracerProps {
  /** start the accumulation again */
  reset?: boolean;
}

/** Bevy's reference path tracer for the <Camera3d> it's inside: slow and exact, accumulating over frames while the
 * view holds still. For stills to check a look against, not for motion. Same GPU needs as <SolariLighting>. */
export function Pathtracer(p: PathtracerProps = {}) {
  return <BevyComponent type="bevy_solari::pathtracer::Pathtracer" value={{ reset: p.reset ?? false }} />;
}
