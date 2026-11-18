// Where a part is drawn (docs/realism.feature LT21): its levels by distance or by size on screen, cross-faded, its
// draw distance and its shadow, as raea-sys-lod's Detail component on the part (systems/lod).
import { BevyComponent } from "./bevy";

/** Where a part is drawn. Distances in metres from the camera; sizes on screen are the part's size across in pixels,
 * at the camera's inside resolution, so a level looks the same at any output size. Drawn from the farther of `near`
 * and where it shrinks to `maxPx`, to the nearer of `far` and where it shrinks to `minPx`; past that it isn't drawn.
 * Levels of one thing are parts side by side, the end of one the start of the next:
 * `<Model src="a.glb" detail={{ far: 120 }} />` and `<Model src="a-far.glb" detail={{ near: 120 }} />`. */
export interface Detail {
  /** drawn from this distance, metres (0 by default) */
  near?: number;
  /** not drawn past this distance, metres (none by default) */
  far?: number;
  /** not drawn once smaller than this on screen, pixels across: a draw distance that scales with the part's size */
  minPx?: number;
  /** drawn only once smaller than this on screen, pixels across: a coarse level, after the fine one */
  maxPx?: number;
  /** each edge cross-fades over this share of its distance, half either side (0.1 by default; 0 switches at once) */
  fade?: number;
  /** false: this level casts no shadow, so a coarser level drawn for the shadows alone can (true by default) */
  shadow?: boolean;
  /** the part's size across, metres, for the sizes on screen (by default its meshes' bounds) */
  size?: number;
}

const NUMS = ["near", "far", "minPx", "maxPx", "fade", "size"] as const;

/** a part's `detail` checked and made raea-sys-lod's component, every field given (raea reads a struct whole) */
export function detailNode(who: string, d: Detail | undefined) {
  if (d === undefined) return null;
  if (typeof d !== "object" || d === null) throw new TypeError(`${who}: detail is { near, far, minPx, maxPx, fade, shadow, size }, got ${JSON.stringify(d)}`);
  for (const k of NUMS) if (d[k] !== undefined && !(Number.isFinite(d[k]) && (d[k] as number) >= 0)) throw new RangeError(`${who}: detail.${k} is 0 or more, got ${d[k]}`);
  if (d.fade !== undefined && d.fade > 1) throw new RangeError(`${who}: detail.fade is a share of the distance, 0 to 1, got ${d.fade}`);
  if (d.far !== undefined && d.near !== undefined && d.far > 0 && d.far <= d.near) throw new RangeError(`${who}: detail.far (${d.far}) must be past detail.near (${d.near})`);
  if (d.shadow !== undefined && typeof d.shadow !== "boolean") throw new TypeError(`${who}: detail.shadow is true or false, got ${JSON.stringify(d.shadow)}`);
  const value = { near: d.near ?? 0, far: d.far ?? 0, min_px: d.minPx ?? 0, max_px: d.maxPx ?? 0, fade: d.fade ?? 0.1, shadow: d.shadow ?? true, size: d.size ?? 0 };
  return <BevyComponent key="raea-detail" type="raea_sys_lod::Detail" value={value} />;
}
