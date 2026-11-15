# SOMA's body as a signed distance grid for the GPU drape, signed by fast winding number (libigl) rather than by the
# nearest face's normal (sdf_build.py), which put "inside" patches in the air above the shoulders.
#   python sdf_igl.py BODY.glb OUT.npz [CELL_M]
# Metres, glTF's frame (y up, +z ahead), negative inside; OUT holds origin, cell, sdf (nx, ny, nz), grad (nx, ny, nz, 3).
import sys, time
import numpy as np, trimesh, igl

glb, out = sys.argv[1:3]
cell = float(sys.argv[3]) if len(sys.argv) > 3 else 0.007
scene = trimesh.load(glb, process=False)
skin = max(scene.dump() if isinstance(scene, trimesh.Scene) else [scene], key=lambda m: len(m.vertices))
V = np.asarray(skin.vertices, float); F = np.asarray(skin.faces, np.int64)
print(f"sdf: skin {len(V)} vertices, y {V[:, 1].min():.3f}..{V[:, 1].max():.3f}, z {V[:, 2].min():.3f}..{V[:, 2].max():.3f}", flush=True)
lo = np.array([-0.75, -0.05, -0.32]); hi = np.array([0.75, 1.88, 0.34])
n = np.ceil((hi - lo) / cell).astype(int) + 1
g = np.stack(np.meshgrid(*[lo[i] + cell * np.arange(n[i]) for i in range(3)], indexing="ij"), -1).reshape(-1, 3)
t0 = time.time()
S, _, _, _ = igl.signed_distance(g, V, F, sign_type=igl.SIGNED_DISTANCE_TYPE_FAST_WINDING_NUMBER)
sdf = S.reshape(n).astype(np.float32)
grad = np.stack(np.gradient(sdf, cell), axis=-1).astype(np.float32)
np.savez_compressed(out, origin=lo.astype(np.float32), cell=np.float32(cell), sdf=sdf, grad=grad)
print(f"sdf: {tuple(int(x) for x in n)} points at {cell * 1000:.0f} mm, inside {int((sdf < 0).sum())}, {time.time() - t0:.0f} s -> {out}")
