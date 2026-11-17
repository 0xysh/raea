# A sewn garment's trims (buttons, shoulder straps, pocket flaps, a belt) as parts of its glb: each one found on the
# draped cloth by casting toward it from outside, built there as its own small mesh (a domed button, a slab that
# follows the cloth, a band round the body), and skinned as the cloth under it is (each vertex the nearest cloth
# vertex's joints and weights), so it moves with the garment wherever <Character> takes it. A trim set made again
# replaces the last. Run after fabric.py's cover when a trim is in the garment's own cloth (it takes the cloth's maps).
#   python tools/garments/trims.py GARMENT.glb TRIMS.json        (add --landmarks: the body's joints, to place by)
# TRIMS.json: {"trims": [ ... ]}, every length in metres, in the garment's frame (glTF: y up, the body facing +z; all
# of raea's sewn garments are cut to SOMA's one body, so a place on one is the same place on another). Each trim:
#   {"kind": "button", "at": [x, y], "from": "front", "d": 0.022, "material": {...}}       from front|back|left|right
#      (or "at": [x, y, z] with "from": [dx, dy, dz], the direction it is cast along); "copies": [[dx, dy], ...]
#      places more of the same, offset; "mirror": true adds the -x twin of each
#   {"kind": "patch", "centre": [x, y, z], "from": [dx, dy, dz], "along": [ux, uy, uz], "size": [length, width],
#      "thick": 0.003, "end": "square"|"round"|"point" (the far end along `along`), "piping": {"w": 0.002, ...material},
#      "mirror": true, "material": {...}}                        a shoulder strap, a pocket flap, a collar patch
# "over": metres a button or patch stands further out (on a strap or a belt, not the cloth under it).
#   {"kind": "band", "y": 1.0, "h": 0.05, "thick": 0.004, "axis": [x, z], "within": 0.32, "material": {...}}
#      round the body at a height, on the outermost cloth nearer the axis than `within` (not the sleeves)
# material: {"colour": "#rrggbb", "metal": 0..1, "rough": 0..1, "cloth": true (the garment's cloth maps under the
#   colour)}. Any trim may carry "source": where it is attested (printed, kept in the glb's extras).
import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from glbio import Glb  # noqa: E402

LIFT = 0.0008     # off the cloth, so a trim never sinks into it where the cloth's triangles bow out between samples
DIRS = {"front": (0, 0, -1), "back": (0, 0, 1), "left": (-1, 0, 0), "right": (1, 0, 0), "top": (0, -1, 0)}


def unit(v):
    v = np.asarray(v, np.float64)
    return v / (np.linalg.norm(v, axis=-1, keepdims=True) + 1e-12)


class Cloth:
    def __init__(self, P, T, N):
        self.P, self.T, self.N = P, T, N
        self.A, self.B, self.C = P[T[:, 0]], P[T[:, 1]], P[T[:, 2]]
        self.e1, self.e2 = self.B - self.A, self.C - self.A

    def hits(self, o, d):
        """Every crossing of the ray o + t d (t > 0): [(t, point, normal)], nearest first (Moller-Trumbore)."""
        p = np.cross(d, self.e2)
        det = (self.e1 * p).sum(1)
        ok = np.abs(det) > 1e-12
        inv = np.where(ok, 1 / np.where(ok, det, 1), 0)
        s = o - self.A
        u = (s * p).sum(1) * inv
        q = np.cross(s, self.e1)
        v = (q * d).sum(1) * inv
        t = (q * self.e2).sum(1) * inv
        k = np.nonzero(ok & (u >= 0) & (v >= 0) & (u + v <= 1) & (t > 1e-6))[0]
        out = []
        for i in k[np.argsort(t[k])]:
            w = np.array([1 - u[i] - v[i], u[i], v[i]])
            n = unit((self.N[self.T[i]] * w[:, None]).sum(0))
            if (n * d).sum() > 0:                         # the side facing where the ray came from
                n = -n
            out.append((t[i], o + t[i] * d, n))
        return out

    def onto(self, at, d):
        """The cloth's first surface met from `at` along `d` (`at` outside it): (point, normal) or None."""
        h = self.hits(np.asarray(at, np.float64), unit(d))
        return (h[0][1], h[0][2]) if h else None


def dome(p, n, r, h, seg=24):
    """A button: a domed disc of radius r standing h off the cloth at p, facing n."""
    t = unit(np.cross(n, [0, 1, 0] if abs(n[1]) < 0.9 else [1, 0, 0]))
    b = np.cross(n, t)
    rings = [(0.0, h), (0.45, 0.96 * h), (0.78, 0.86 * h), (0.93, 0.74 * h), (1.0, 0.5 * h), (1.0, -0.4 * h)]
    a = np.linspace(0, 2 * np.pi, seg, endpoint=False)
    V, UV = [], []
    for rr, hh in rings:
        for x in a:
            V.append(p + n * (hh + LIFT) + r * rr * (np.cos(x) * t + np.sin(x) * b))
            UV.append([0.5 + 0.5 * rr * np.cos(x), 0.5 + 0.5 * rr * np.sin(x)])
    F = []
    for i in range(len(rings) - 1):
        for k in range(seg):
            a0, a1 = i * seg + k, i * seg + (k + 1) % seg
            F += [[a0, a0 + seg, a1], [a1, a0 + seg, a1 + seg]]
    return np.array(V), np.array(F), np.array(UV)


def slab(grid_p, grid_n, inside, thick):
    """A slab conformed to the cloth: grid points (nu x nv x 3) and normals where it lies, `inside` (nu x nv) the
    cells' corners kept (a rounded or pointed end leaves some out), standing `thick` off it. Returns V, F, UV
    (UV in metres along the slab, so the garment's cloth maps lie on it at their own scale)."""
    nu, nv = inside.shape
    top = grid_p + grid_n * (LIFT + thick)
    bot = grid_p + grid_n * LIFT * 0.5
    V, UV, idx = [], [], -np.ones((2, nu, nv), int)
    du = np.linalg.norm(np.diff(grid_p, axis=0), axis=2).mean() if nu > 1 else 0
    dv = np.linalg.norm(np.diff(grid_p, axis=1), axis=2).mean() if nv > 1 else 0
    for layer, G in ((0, top), (1, bot)):
        for i in range(nu):
            for k in range(nv):
                if inside[i, k]:
                    idx[layer, i, k] = len(V)
                    V.append(G[i, k])
                    UV.append([k * dv, i * du])
    F = []
    for i in range(nu - 1):
        for k in range(nv - 1):
            q = [idx[0, i, k], idx[0, i + 1, k], idx[0, i + 1, k + 1], idx[0, i, k + 1]]
            if min(q) >= 0:
                F += [[q[0], q[2], q[1]], [q[0], q[3], q[2]]]
    # the edge: wherever a kept corner has an unkept (or no) neighbour, a wall from the top down to the cloth
    def kept(i, k):
        return 0 <= i < nu and 0 <= k < nv and inside[i, k]
    for i in range(nu):
        for k in range(nv):
            for (di, dk) in ((1, 0), (0, 1)):
                i2, k2 = i + di, k + dk
                if not (kept(i, k) and kept(i2, k2)):
                    continue
                # the two cells either side of this edge: a wall when only one of them is whole
                if di:
                    sides = [all(kept(i + a, k + b) for a in (0, 1) for b in (s, s + 1)) for s in (-1, 0)]
                else:
                    sides = [all(kept(i + a, k + b) for a in (s, s + 1) for b in (0, 1)) for s in (-1, 0)]
                if sides[0] == sides[1]:
                    continue
                a0, a1, b0, b1 = idx[0, i, k], idx[0, i2, k2], idx[1, i, k], idx[1, i2, k2]
                F += [[a0, b0, a1], [a1, b0, b1]] if sides[1] ^ bool(di) else [[a0, a1, b0], [a1, b1, b0]]
    return np.array(V), np.array(F), np.array(UV)


def patch_grid(cl, centre, d, along, size, end, res=0.006):
    """Cast a grid over a length x width rectangle at `centre` (length along `along`) down `d` onto the cloth."""
    d = unit(d)
    U = unit(np.asarray(along) - (np.asarray(along) @ d) * d)
    W = np.cross(d, U)
    L, w = size
    nu, nv = max(2, int(np.ceil(L / res)) + 1), max(2, int(np.ceil(w / res)) + 1)
    P = np.zeros((nu, nv, 3)); N = np.zeros((nu, nv, 3)); ok = np.zeros((nu, nv), bool)
    for i in range(nu):
        for k in range(nv):
            u, v = -L / 2 + L * i / (nu - 1), -w / 2 + w * k / (nv - 1)
            # the far end: round (a semicircle of the width) or point (a chevron), cut from the rectangle
            far = u - (L / 2 - w / 2)
            if end == "round" and far > 0 and far ** 2 + v ** 2 > (w / 2) ** 2 + 1e-9:
                continue
            if end == "point" and far > 0 and abs(v) > (w / 2) * (1 - far / (w / 2)) + 1e-9:
                continue
            o = np.asarray(centre) + u * U + v * W - d * 0.4
            h = cl.onto(o, d)
            if h is None:
                continue
            P[i, k], N[i, k], ok[i, k] = h[0], h[1], True
    return P, N, ok


def band(cl, y, h, thick, axis, within, seg=96):
    """A band round the body: at each angle, the outermost cloth nearer the axis than `within`."""
    rows = 4
    P = np.zeros((rows, seg + 1, 3)); N = np.zeros_like(P); ok = np.zeros((rows, seg + 1), bool)
    for r in range(rows):
        yy = y - h / 2 + h * r / (rows - 1)
        for k in range(seg + 1):
            a = 2 * np.pi * k / seg
            o = np.array([axis[0] + 0.8 * np.cos(a), yy, axis[1] + 0.8 * np.sin(a)])
            hs = cl.hits(o, unit([axis[0] - o[0], 0, axis[1] - o[2]]))
            hs = [x for x in hs if np.hypot(x[1][0] - axis[0], x[1][2] - axis[1]) < within]
            if hs:
                P[r, k], N[r, k], ok[r, k] = hs[0][1], unit(hs[0][2] * [1, 0.3, 1]), True
    # a gap (a ray that met nothing) filled from its neighbours round the ring
    for r in range(rows):
        good = np.nonzero(ok[r])[0]
        if len(good) == 0:
            raise SystemExit(f"trims: no cloth round y={y:.3f} within {within} m of the axis")
        for k in np.nonzero(~ok[r])[0]:
            j = good[np.argmin(np.minimum(np.abs(good - k), seg - np.abs(good - k)))]
            P[r, k], N[r, k] = P[r, j], N[r, j]
        ok[r] = True
    # smooth round the ring (the cloth's folds shouldn't kink a belt pulled tight over them)
    for _ in range(3):
        P[:, 1:-1] = (P[:, :-2] + 2 * P[:, 1:-1] + P[:, 2:]) / 4
    P[:, -1] = P[:, 0]
    return P, N, ok


def material(spec, base, name):
    c = spec.get("colour", "#808080")
    lin = [(int(c[i:i + 2], 16) / 255) ** 2.2 for i in (1, 3, 5)]
    pbr = base.get("pbrMetallicRoughness", {})
    m = {"name": name, "extras": {"trim": True, "colour": c}}
    if spec.get("cloth") and "baseColorTexture" in pbr:
        m["pbrMetallicRoughness"] = {"baseColorFactor": [min(1, x / 0.8) for x in lin] + [1], "baseColorTexture": pbr["baseColorTexture"],
                                     "metallicRoughnessTexture": pbr["metallicRoughnessTexture"], "metallicFactor": 0.0, "roughnessFactor": 1.0}
        m["normalTexture"] = base["normalTexture"]
        m["extras"]["cloth"] = base.get("extras", {}).get("cloth", True)
    else:
        m["pbrMetallicRoughness"] = {"baseColorFactor": lin + [1], "metallicFactor": float(spec.get("metal", 0)), "roughnessFactor": float(spec.get("rough", 0.6))}
    return m


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__ or "trims.py GARMENT.glb TRIMS.json")
    path = sys.argv[1]
    g = Glb(path)
    j = g.j
    if "--landmarks" in sys.argv:
        # the joints' bind-pose places: each joint's world matrix is the inverse of its inverse bind matrix
        sk = j["skins"][0]
        ibm = g.read(sk["inverseBindMatrices"]).reshape(-1, 4, 4).transpose(0, 2, 1)
        for n, m in zip(sk["joints"], ibm):
            w = np.linalg.inv(m)
            name = j["nodes"][n].get("name", n)
            if any(s in name for s in ("Neck", "Spine", "Chest", "Hips", "Shoulder", "Clavicle", "Arm", "Head", "Leg")) and "Twist" not in name:
                print(f"{name:24s} {w[0, 3]:+.3f} {w[1, 3]:+.3f} {w[2, 3]:+.3f}")
        return
    spec = json.load(open(sys.argv[2]))
    mesh = j["meshes"][0]
    # the last trims out: their primitives, materials and accessors
    keep = [p for p in mesh["primitives"] if not p.get("extras", {}).get("trim")]
    mesh["primitives"] = keep
    mats = j.get("materials", [])
    live = [i for i, m in enumerate(mats) if not m.get("extras", {}).get("trim")]
    remap = {o: n for n, o in enumerate(live)}
    j["materials"] = [mats[i] for i in live]
    for p in keep:
        if "material" in p:
            p["material"] = remap[p["material"]]
    g.drop_accessors(g.used_accessors())
    # the cloth is every primitive the garment had (a collar in its own colour is a primitive of its own)
    base = keep[0]
    Ps, Ns, Ts, Js, Ws, at = [], [], [], [], [], 0
    for p in keep:
        Ps.append(g.read(p["attributes"]["POSITION"]).astype(np.float64)); Ns.append(g.read(p["attributes"]["NORMAL"]).astype(np.float64))
        Ts.append(g.read(p["indices"]).reshape(-1, 3).astype(np.int64) + at); at += len(Ps[-1])
        Js.append(g.read(p["attributes"]["JOINTS_0"]).astype(np.uint16)); Ws.append(g.read(p["attributes"]["WEIGHTS_0"]).astype(np.float32))
    P, Nr, T, Jn, Wt = (np.concatenate(x) for x in (Ps, Ns, Ts, Js, Ws))
    cl = Cloth(P, T, Nr)
    from scipy.spatial import cKDTree
    tree = cKDTree(P)
    cloth_mat = j["materials"][base.get("material", 0)] if j.get("materials") else {}
    made = []

    def emit(V, F, UV, mspec, label, src):
        if len(F) == 0:
            print(f"trims: {label}: nothing of it lies on the cloth (missed)")
            return
        V, F = np.asarray(V, np.float64), np.asarray(F, np.int64)
        # normals: area-weighted from the faces
        Nv = np.zeros_like(V)
        fn = np.cross(V[F[:, 1]] - V[F[:, 0]], V[F[:, 2]] - V[F[:, 0]])
        for c in range(3):
            np.add.at(Nv, F[:, c], fn)
        Nv = unit(Nv).astype(np.float32)
        _, nn = tree.query(V)
        j.setdefault("materials", []).append(material(mspec, cloth_mat, label))
        prim = {"attributes": {"POSITION": g.accessor(V.astype(np.float32), 34962, bounds=True), "NORMAL": g.accessor(Nv, 34962),
                               "TEXCOORD_0": g.accessor(np.asarray(UV, np.float32), 34962), "JOINTS_0": g.accessor(Jn[nn], 34962),
                               "WEIGHTS_0": g.accessor(Wt[nn], 34962)},
                "indices": g.accessor(F.astype(np.uint32).reshape(-1), 34963), "material": len(j["materials"]) - 1,
                "extras": {"trim": label, **({"source": src} if src else {})}}
        mesh["primitives"].append(prim)
        made.append(label)

    for n, t in enumerate(spec["trims"]):
        kind, src = t["kind"], t.get("source", "")
        label = t.get("name", f"{kind}-{n}")
        mirrors = [1, -1] if t.get("mirror") else [1]
        if kind == "button":
            d = DIRS[t["from"]] if isinstance(t["from"], str) else t["from"]
            d = unit(d)
            spots = [t["at"]] + [[t["at"][0] + o[0], t["at"][1] + o[1]] + list(t["at"][2:]) for o in t.get("copies", [])]
            Vs, Fs, UVs = [], [], []
            for sx in mirrors:
                for at in spots:
                    at = np.array([sx * at[0], at[1], at[2] if len(at) > 2 else 0.0])
                    dd = d * [sx, 1, 1]
                    if len(t["at"]) == 2:          # from outside along d: start well clear
                        at = at - dd * 0.6
                        if t["from"] in ("front", "back"):
                            at[2] = -dd[2] * 0.6
                    h = cl.onto(at, dd)
                    if h is None:
                        print(f"trims: {label}: no cloth at {np.round(at, 3).tolist()}")
                        continue
                    V, F, UV = dome(h[0] + h[1] * t.get("over", 0), h[1], t.get("d", 0.022) / 2, t.get("h", t.get("d", 0.022) * 0.18))
                    Fs.append(F + sum(len(x) for x in Vs)); Vs.append(V); UVs.append(UV)
            if Vs:
                emit(np.concatenate(Vs), np.concatenate(Fs), np.concatenate(UVs), t.get("material", {}), label, src)
        elif kind == "patch":
            for sx in mirrors:
                c = np.array(t["centre"]) * [sx, 1, 1]
                d = np.array(DIRS[t["from"]] if isinstance(t["from"], str) else t["from"], np.float64) * [sx, 1, 1]
                u = np.array(t["along"], np.float64) * [sx, 1, 1]
                L, w = t["size"]
                if "piping" in t:
                    pw = t["piping"].get("w", 0.002)
                    Pg, Ng, ok = patch_grid(cl, c, d, u, [L + 2 * pw, w + 2 * pw], t.get("end", "square"))
                    V, F, UV = slab(Pg, Ng, ok, t.get("thick", 0.003) * 0.85)
                    emit(V, F, UV, t["piping"], f"{label}-piping{'-r' if sx < 0 else ''}", src)
                Pg, Ng, ok = patch_grid(cl, c, d, u, [L, w], t.get("end", "square"))
                V, F, UV = slab(Pg + Ng * t.get("over", 0), Ng, ok, t.get("thick", 0.003))
                emit(V, F, UV, t.get("material", {}), f"{label}{'-r' if sx < 0 else ''}", src)
        elif kind == "band":
            Pg, Ng, ok = band(cl, t["y"], t.get("h", 0.05), t.get("thick", 0.004), t.get("axis", [0, -0.02]), t.get("within", 0.32))
            V, F, UV = slab(Pg.transpose(1, 0, 2), Ng.transpose(1, 0, 2), ok.T, t.get("thick", 0.004))
            emit(V, F, UV, t.get("material", {}), label, src)
        else:
            raise SystemExit(f"trims: {label}: kind {kind} is button, patch or band")
        if src:
            print(f"trims: {label} ({src})")
    size = g.save(path)
    print(f"trims: {len(made)} parts on {os.path.basename(path)} ({size / 1e6:.1f} MB)")


if __name__ == "__main__":
    main()
