# Clothes for SOMA's body from pictures of the dressed mannequin (raea docs/character.feature CH40 to CH45), the
# measuring and wearing behind `raea clothes make` and `raea clothes wear`. A garment is data on SOMA's own
# vertices, by their numbers in SOMA's template (so it rides any SOMA body): which are covered, how far the cloth
# stands off each, and its colour there.
#
#   measure: raea blender run tools/clothes/garment.py measure BODY.glb RIG.usda VIEWS_DIR OUT.json [UNDER.json UNDER_VIEWS_DIR]
#     VIEWS_DIR/{front,back,left,right}.jpg: the mannequin's views (tools/clothes/mannequin.py), each dressed by an
#     image model on its own, so each lies over its mannequin pixel for pixel. Each vertex of the skin is looked up
#     in the views that face it and see it; where the picture shows cloth it is covered and takes the colour. With
#     UNDER (the layer beneath and the pictures it was measured from), the garment is only where the picture changed
#     from the layer beneath, and it stands off at least that layer's distance plus its own thickness.
#   cover: raea blender run tools/clothes/garment.py cover BODY.glb RIG.usda OUT.json PART[:SHARE],... #RRGGBB STAND_M [UNDER.json]
#     a garment by procedure in the same format, with no picture: the skin of the parts named (each limb as far
#     along as its share), in one colour, so far off the skin (and a layer's thickness off UNDER).
#   wear: raea blender run tools/clothes/garment.py wear BODY.glb RIG.usda OUT_DIR GARMENT.json...   (innermost first)
#     writes OUT_DIR/<name>.glb for each garment (the covered skin pushed out, with the body's own weights) and
#     OUT_DIR/body.glb, the body with the skin that is wholly under cloth left out.
import sys, os, json
import bpy, bmesh, numpy as np
from mathutils import Vector, kdtree
from mathutils.bvhtree import BVHTree

args = sys.argv[sys.argv.index("--") + 1:]
mode, body_glb, rig_usd = args[:3]
OFF, THICK, MID = 0.004, 0.003, 0.9  # least stand-off, a layer's own thickness, the views' centre height (metres)


def canon():
    """SOMA's template vertices at rest, Blender's frame, metres: the numbers a garment is kept by"""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.wm.usd_import(filepath=rig_usd)
    o = next(o for o in bpy.data.objects if o.type == "MESH" and o.name.startswith("c_skin_mid"))
    return np.array([o.matrix_world @ v.co for v in o.data.vertices])


def load_body():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=body_glb)
    for o in [o for o in bpy.data.objects if o.type == "MESH" and not any(m.type == "ARMATURE" for m in o.modifiers)]:
        bpy.data.objects.remove(o)
    return max((o for o in bpy.data.objects if o.type == "MESH"), key=lambda o: len(o.data.vertices))


C = canon()
tree = kdtree.KDTree(len(C))
for i, p in enumerate(C):
    tree.insert(Vector(p), i)
tree.balance()
# each of the body's vertices: its number in the template, found by its rest place. A body of another shape has
# its vertices elsewhere, so NEUTRAL names the template-shaped body it was built beside (same export, same order).
neutral = os.environ.get("NEUTRAL")
if neutral:
    keep_path, body_glb = body_glb, neutral
    ref = load_body()
    cid = np.array([tree.find(ref.matrix_world @ v.co)[1] for v in ref.data.vertices])
    body_glb = keep_path
skin = load_body()
me, W = skin.data, skin.matrix_world
co = np.array([W @ v.co for v in me.vertices]); no = np.array([(W.to_3x3() @ v.normal).normalized() for v in me.vertices])
n = len(co)
if neutral:
    assert len(cid) == n, f"the shaped body has {n} vertices, its neutral twin {len(cid)}"
else:
    cid = np.array([tree.find(Vector(p))[1] for p in co])
bmw = bmesh.new(); bmw.from_mesh(me); bmw.transform(W); bvh = BVHTree.FromBMesh(bmw); bmw.verts.ensure_lookup_table()
ring = [[e.other_vert(v).index for e in v.link_edges] for v in bmw.verts]
bmw.free()

VIEWS = {  # the way to the camera, a point's place in the picture (u right, v down, 0 to 1), the picture's right in the world
    "front": ((0, -1, 0), lambda p: (0.5 + p[0] / 2, 0.5 - (p[2] - MID) / 2), (1, 0, 0)),
    "back": ((0, 1, 0), lambda p: (0.5 - p[0] / 2, 0.5 - (p[2] - MID) / 2), (-1, 0, 0)),
    "left": ((1, 0, 0), lambda p: (0.5 + p[1] / 2, 0.5 - (p[2] - MID) / 2), (0, 1, 0)),
    "right": ((-1, 0, 0), lambda p: (0.5 - p[1] / 2, 0.5 - (p[2] - MID) / 2), (0, -1, 0)),
}


def pixels(d, name):
    img = bpy.data.images.load(os.path.join(d, f"{name}.jpg"))
    w, h = img.size
    return np.array(img.pixels[:], dtype=np.float32).reshape(h, w, 4)[::-1, :, :3]


def at(px, u, v):
    h, w = px.shape[:2]
    return px[min(h - 1, max(0, int(v * h))), min(w - 1, max(0, int(u * w)))]


def measure(views_dir, out, under=None, under_views=None):
    weight = np.zeros(n); cloth = np.zeros(n); colour = np.zeros((n, 3)); PIX = {}
    for name, (toward, place, _) in VIEWS.items():
        px = PIX[name] = pixels(views_dir, name)
        below = pixels(under_views, name) if under_views else None
        t = Vector(toward)
        for i in range(n):
            facing = float(no[i] @ np.array(toward))
            if facing < 0.35:
                continue
            p = Vector(co[i])
            if bvh.ray_cast(p + t * 0.002 + Vector(no[i]) * 0.001, t)[0] is not None:
                continue  # the body itself is between this point and the camera
            u, v = place(p)
            c = at(px, u, v); k = facing * facing
            weight[i] += k
            sat = float(c.max() - c.min()) / max(float(c.max()), 1e-3)
            bare = 0.17 < float(c.mean()) < 0.45 and sat < 0.12  # the mannequin's dark grey (black cloth is darker still)
            new = below is None or float(np.abs(c - at(below, u, v)).max()) > 0.08  # the picture changed here
            # (tried 11-15 and taken back: over a layer, counting any changed pixel as cloth whatever its shade. The
            # mannequin's own shade differs a little between two pictures, and the jacket then covered the hands)
            if not bare and new:
                cloth[i] += k; colour[i] += k * c
    seen = weight > 0
    covered = seen & (cloth > 0.5 * weight)
    colour[covered] /= cloth[covered][:, None]
    for _ in range(14):  # a vertex no view saw takes its neighbours' answer
        for i in np.nonzero(~seen)[0]:
            near = [j for j in ring[i] if seen[j]]
            if near:
                covered[i] = sum(covered[j] for j in near) * 2 > len(near)
                cs = [colour[j] for j in near if covered[j]]
                if covered[i] and cs:
                    colour[i] = np.mean(cs, axis=0)
                seen[i] = True
    for _ in range(3):  # a bare vertex ringed by cloth is cloth
        for i in [i for i in range(n) if not covered[i] and ring[i] and sum(covered[j] for j in ring[i]) * 3 >= len(ring[i]) * 2]:
            cs = [colour[j] for j in ring[i] if covered[j]]
            covered[i] = True; colour[i] = np.mean(cs, axis=0)
    left = set(np.nonzero(covered)[0].tolist())
    while left:  # and a small island of cloth is not
        stack, isle = [left.pop()], []
        while stack:
            i = stack.pop(); isle.append(i)
            for j in ring[i]:
                if j in left:
                    left.remove(j); stack.append(j)
        if len(isle) < 150:
            covered[isle] = False
    # a small bare patch walled in by cloth is cloth: the tops of the shoulders, which no level view faces
    bare = set(np.nonzero(~covered)[0].tolist())
    while bare:
        stack, patch = [bare.pop()], []
        while stack:
            i = stack.pop(); patch.append(i)
            for j in ring[i]:
                if j in bare:
                    bare.remove(j); stack.append(j)
        if len(patch) < 400:
            edge = [j for i in patch for j in ring[i] if covered[j]]
            if edge:
                c = np.mean(colour[edge], axis=0)
                covered[patch] = True; colour[patch] = c
    # and a bare vertex that faces up (which no level view sees squarely) among mostly covered skin is covered: the
    # tops of the shoulders under a jacket, whose gap runs into the collar and so is walled in by nothing
    for _ in range(4):
        add = []
        for i in np.nonzero(~covered & (no[:, 2] > 0.5))[0]:
            near, edge = {int(i)}, [int(i)]
            for _d in range(3):
                edge = [j for k in edge for j in ring[k] if j not in near]
                near.update(edge)
            near.discard(int(i))
            if near and sum(covered[j] for j in near) * 2 >= len(near):
                add.append(i)
        for i in add:
            cs = [colour[j] for j in ring[i] if covered[j]]
            covered[i] = True
            if cs:
                colour[i] = np.mean(cs, axis=0)
        if not add:
            break
    # and one that faces upward with cloth both in front of it and behind it, within 7 cm: the top of a shoulder
    # between a shirt's front and its back, which the bare neck beside it outvoted above, and the slope by the collar,
    # where the front view sees the layer beneath through an open collar (11-15, the overshirt's pale yokes); the neck
    # faces sideways and has no cloth in front of it
    for _ in range(3):
        held = np.nonzero(covered)[0]
        if not len(held):
            break
        kd = kdtree.KDTree(len(held))
        for k, i in enumerate(held):
            kd.insert(Vector(co[i]), k)
        kd.balance()
        add = []
        for i in np.nonzero(~covered & (no[:, 2] > 0.3))[0]:
            ys = [co[held[k]][1] - co[i][1] for _, k, _ in kd.find_range(Vector(co[i]), 0.07)]
            if any(y < -0.01 for y in ys) and any(y > 0.01 for y in ys):
                add.append(i)
        for i in add:
            cs = [colour[j] for j in ring[i] if covered[j]]
            covered[i] = True
            if cs:
                colour[i] = np.mean(cs, axis=0)
        if not add:
            break
    # the stand-off: read where a view sees a covered vertex on the body's outline, outward along its normal in the
    # picture until the backdrop, then spread over the garment
    floor = np.full(n, OFF)
    if under:
        u_stand = np.zeros(len(C)); u_stand[under["vertices"]] = under["stand"]
        floor = np.maximum(OFF, u_stand[cid] + THICK)
    stand = np.full(n, np.nan)
    for name, (toward, place, right) in VIEWS.items():
        px = PIX[name]; h, w = px.shape[:2]
        back = np.median(np.concatenate([px[:24, :24].reshape(-1, 3), px[:24, -24:].reshape(-1, 3)]), axis=0)
        ur, vd = np.array(right, float), np.array((0, 0, -1), float)
        for i in np.nonzero(covered)[0]:
            if abs(float(no[i] @ np.array(toward))) > 0.3:
                continue
            d = np.array([no[i] @ ur, no[i] @ vd]); L = np.hypot(*d)
            if L < 0.8:
                continue
            d /= L
            u0, v0 = place(Vector(co[i]))
            for k in range(1, 70):
                x, y = int(u0 * w + d[0] * k), int(v0 * h + d[1] * k)
                if not (0 <= x < w and 0 <= y < h):
                    break
                if float(np.abs(px[y, x] - back).max()) < 0.07:
                    r = k * 2.0 / w
                    stand[i] = r if np.isnan(stand[i]) else min(stand[i], r)
                    break
    known = ~np.isnan(stand)
    val = np.where(known, np.clip(stand, floor, 0.08), floor)
    for _ in range(60):
        nxt = val.copy()
        for i in np.nonzero(covered & ~known)[0]:
            nb = [j for j in ring[i] if covered[j]]
            if nb:
                nxt[i] = sum(val[j] for j in nb) / len(nb)
        val = nxt
    for _ in range(4):
        nxt = val.copy()
        for i in np.nonzero(covered)[0]:
            nb = [j for j in ring[i] if covered[j]]
            if nb:
                nxt[i] = 0.5 * val[i] + 0.5 * sum(val[j] for j in nb) / len(nb)
        val = np.maximum(nxt, floor)
    # and no further off than its part allows: a loose sleeve is drawn hanging below the arm in the mannequin's pose,
    # and that depth, read at the outline and laid all round the arm, made a balloon of it (11-15)
    gname = {g.index: g.name for g in skin.vertex_groups}
    cap = np.array([STAND_CAP.get(part_of(gname[max(v.groups, key=lambda g: g.weight).group])[0], 0.08) if v.groups else 0.08 for v in me.vertices])
    val = np.maximum(np.minimum(val, cap), floor)
    # by the template's numbers: a template vertex is covered if any of the body's copies of it is
    ids = sorted(set(int(c) for c in cid[covered]))
    where = {c: k for k, c in enumerate(ids)}
    s_sum = np.zeros(len(ids)); c_sum = np.zeros((len(ids), 3)); cnt = np.zeros(len(ids))
    for i in np.nonzero(covered)[0]:
        k = where[int(cid[i])]; s_sum[k] += val[i]; c_sum[k] += colour[i]; cnt[k] += 1
    g = {"name": os.path.splitext(os.path.basename(out))[0], "topology": "SOMA template, c_skin_mid, 18056 vertices", "units": "metres", "layer": (under["layer"] + 1) if under else 0,
         **({"over": under["name"]} if under else {}),
         # (the views by their path from the garment's file, so a project's garments move with it)
         "source": {"views": os.path.relpath(os.path.abspath(views_dir), os.path.dirname(os.path.abspath(out))), "how": "raea clothes make (tools/clothes/garment.py measure)"},
         "vertices": ids, "stand": [round(float(x), 5) for x in s_sum / cnt], "colour_srgb": [[round(float(x), 4) for x in c] for c in c_sum / cnt[:, None]],
         # how far the cloth may stray from its skinned place: nothing where it is snug, more the further it stands off
         "stray": [round(float(max(0.0, x - 0.012) * 0.6), 5) for x in s_sum / cnt]}
    json.dump(g, open(out, "w"), separators=(",", ":"))
    q = np.percentile(val[covered], [10, 50, 90]) * 1000
    print(f"clothes: {g['name']}: layer {g['layer']}, {len(ids)} of {len(C)} template vertices covered; stand-off mm at 10, 50, 90 per cent: {q[0]:.1f}, {q[1]:.1f}, {q[2]:.1f}; outline readings {int(known.sum())}")


def export(obj, path, also=()):
    for o in bpy.data.objects:
        o.select_set(o is obj or o in also or o.type == "ARMATURE")
    kw = dict(filepath=path, export_format="GLB", use_selection=True, export_animations=False, export_yup=True, export_skins=True)
    try:
        bpy.ops.export_scene.gltf(**kw, export_vertex_color="ACTIVE", export_all_vertex_colors=False)
    except TypeError:
        bpy.ops.export_scene.gltf(**kw)


def seen_by():
    """per view: which of the body's vertices it sees (facing it, nothing of the body between)"""
    out = {}
    for name, (toward, _, _) in VIEWS.items():
        t = Vector(toward); f = no @ np.array(toward)
        out[name] = np.array([f[i] > 0.2 and bvh.ray_cast(Vector(co[i]) + t * 0.002 + Vector(no[i]) * 0.001, t)[0] is None for i in range(n)])
    return out


def bake(mesh, covered, views_dir, vcol, size=2048):
    """The garment's colour as a picture in the body's first UV set (spread to fill the picture): each texel of a
    covered triangle is a point of the body, looked up in the view that faces it most among those that see it. Where
    that view only grazes it, or shows backdrop or the bare mannequin there (a shoulder's top, a cuff past the drawn
    sleeve), the texel takes the garment's measured colour at its corners (vcol, per body vertex) instead."""
    uvl = mesh.uv_layers[0].data
    mesh.calc_loop_triangles()
    tris = [t for t in mesh.loop_triangles if all(covered[v] for v in t.vertices)]
    uv = np.array([[uvl[l].uv[:] for l in t.loops] for t in tris], float)
    lo, hi = uv.reshape(-1, 2).min(0), uv.reshape(-1, 2).max(0)
    uv = (uv - lo) / np.maximum(hi - lo, 1e-9)
    vid = np.array([t.vertices[:] for t in tris])
    PIX = {name: pixels(views_dir, name) for name in VIEWS}
    BACK = {name: np.median(np.concatenate([p[:24, :24].reshape(-1, 3), p[:24, -24:].reshape(-1, 3)]), axis=0) for name, p in PIX.items()}
    vis = seen_by()
    img = np.zeros((size, size, 3), np.float32); got = np.zeros((size, size), bool)
    names = list(VIEWS)
    toward = np.array([VIEWS[k][0] for k in names], float)
    for k in range(len(tris)):
        a, b, c = uv[k] * (size - 1)
        x0, x1 = int(np.floor(min(a[0], b[0], c[0]))), int(np.ceil(max(a[0], b[0], c[0])))
        y0, y1 = int(np.floor(min(a[1], b[1], c[1]))), int(np.ceil(max(a[1], b[1], c[1])))
        gx, gy = np.meshgrid(np.arange(x0, x1 + 1), np.arange(y0, y1 + 1))
        den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
        if abs(den) < 1e-9:
            continue
        w0 = ((b[1] - c[1]) * (gx - c[0]) + (c[0] - b[0]) * (gy - c[1])) / den
        w1 = ((c[1] - a[1]) * (gx - c[0]) + (a[0] - c[0]) * (gy - c[1])) / den
        w2 = 1 - w0 - w1
        inside = (w0 >= -0.02) & (w1 >= -0.02) & (w2 >= -0.02)
        if not inside.any():
            continue
        v = vid[k]; fn = no[v].mean(0); fn /= max(np.linalg.norm(fn), 1e-9)
        # the view: the one this triangle faces most, of those that see all its corners (else any it faces)
        score = toward @ fn
        ok = np.array([vis[nm][v].all() for nm in names])
        pick = int(np.argmax(np.where(ok, score, score - 2)))
        place = VIEWS[names[pick]][1]; px = PIX[names[pick]]; h, w = px.shape[:2]
        P = w0[inside][:, None] * co[v[0]] + w1[inside][:, None] * co[v[1]] + w2[inside][:, None] * co[v[2]]
        u, vv = place(P.T)
        col = px[np.clip((vv * h).astype(int), 0, h - 1), np.clip((u * w).astype(int), 0, w - 1)]
        mean, sat = col.mean(1), (col.max(1) - col.min(1)) / np.maximum(col.max(1), 1e-3)
        off = (np.abs(col - BACK[names[pick]]).max(1) < 0.07) | ((mean > 0.17) & (mean < 0.45) & (sat < 0.12)) | (score[pick] < 0.35)
        if off.any():
            mine = w0[inside][:, None] * vcol[v[0]] + w1[inside][:, None] * vcol[v[1]] + w2[inside][:, None] * vcol[v[2]]
            col = np.where(off[:, None], mine, col)
        img[gy[inside], gx[inside]] = col; got[gy[inside], gx[inside]] = True
    # (tried and taken out, 11-15: moving each other view's colours onto the front's by the front's main colours;
    # pale denim on the back was taken for the shirt's white and turned white. The views' colours still differ.)
    for _ in range(6):  # spread past each island's edge, so a filtered lookup doesn't pull in black
        nb = np.zeros_like(img); cnt = np.zeros(got.shape, np.float32)
        for dy, dx in ((0, 1), (0, -1), (1, 0), (-1, 0)):
            nb += np.roll(np.roll(img * got[..., None], dy, 0), dx, 1); cnt += np.roll(np.roll(got, dy, 0), dx, 1)
        fill = ~got & (cnt > 0)
        img[fill] = nb[fill] / cnt[fill][:, None]; got |= fill
    return img, lo, hi


def wear(out_dir, garments):
    os.makedirs(out_dir, exist_ok=True)
    under = np.zeros(n, bool)
    for path in garments:
        g = json.load(open(path))
        stand = np.zeros(len(C)); col = np.zeros((len(C), 3)); has = np.zeros(len(C), bool)
        has[g["vertices"]] = True; stand[g["vertices"]] = g["stand"]; col[g["vertices"]] = g["colour_srgb"]
        covered = has[cid]
        piece = skin.copy(); piece.data = skin.data.copy(); bpy.context.scene.collection.objects.link(piece)
        bm = bmesh.new(); bm.from_mesh(piece.data); bm.verts.ensure_lookup_table()
        # the body's own open edges (its UV seams, where glTF splits a vertex in two): kept where they are below
        seam = bm.verts.layers.int.new("seam")
        for v in bm.verts:
            v[seam] = int(v.is_boundary)
        layer = bm.loops.layers.color.new("cloth")
        for f in bm.faces:
            for l in f.loops:
                l[layer] = (*[float(x) ** 2.2 for x in col[cid[l.vert.index]]], 1.0)
        for v in bm.verts:
            v.co += v.normal * float(stand[cid[v.index]])
        bmesh.ops.delete(bm, geom=[f for f in bm.faces if not all(covered[v.index] for v in f.verts)], context="FACES")
        bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
        # the hem: a garment's open edge runs along the body's triangles, a stair of them; each edge vertex is drawn
        # toward the middle of its two neighbours along the edge, a few times, which evens the stair into a line
        edge = [v for v in bm.verts if v.is_boundary and not v[seam]]
        for _ in range(4):
            to = {}
            for v in edge:
                nb = [e.other_vert(v) for e in v.link_edges if e.is_boundary and not e.other_vert(v)[seam]]
                if len(nb) == 2:
                    to[v] = v.co * 0.5 + (nb[0].co + nb[1].co) * 0.25
            for v, c in to.items():
                v.co = c
        bm.to_mesh(piece.data); bm.free()
        # its colour: a picture in the body's UVs, baked from the views it was measured from (the garment's own UVs
        # are the body's, spread to fill that picture); a garment made by procedure has no views and is its one colour
        if "views" in g["source"]:
            tex, lo, hi = bake(skin.data, covered, os.path.join(os.path.dirname(os.path.abspath(path)), g["source"]["views"]), col[cid])
        else:
            tex, lo, hi = np.tile(np.mean(g["colour_srgb"], axis=0).astype(np.float32), (4, 4, 1)), np.zeros(2), np.ones(2)
        uvl = piece.data.uv_layers[0].data
        for l in uvl:
            l.uv = ((l.uv[0] - lo[0]) / max(hi[0] - lo[0], 1e-9), (l.uv[1] - lo[1]) / max(hi[1] - lo[1], 1e-9))
        im = bpy.data.images.new(g["name"], tex.shape[1], tex.shape[0], alpha=False)
        im.pixels = np.concatenate([tex, np.ones((*tex.shape[:2], 1), np.float32)], axis=2).reshape(-1)  # (rows bottom up, as v runs)
        im.filepath_raw = os.path.join(out_dir, g["name"] + ".png"); im.file_format = "PNG"; im.save()
        mat = bpy.data.materials.new(g["name"]); mat.use_nodes = True
        bsdf = next(x for x in mat.node_tree.nodes if x.type == "BSDF_PRINCIPLED")
        a = mat.node_tree.nodes.new("ShaderNodeTexImage"); a.image = im
        mat.node_tree.links.new(a.outputs["Color"], bsdf.inputs["Base Color"]); bsdf.inputs["Roughness"].default_value = 0.85
        piece.data.materials.clear(); piece.data.materials.append(mat)
        export(piece, os.path.join(out_dir, g["name"] + ".glb"))
        bpy.data.objects.remove(piece)
        under |= covered
        print(f"clothes: wore {g['name']} over {int(covered.sum())} of the body's vertices: {os.path.join(out_dir, g['name'] + '.glb')}")
    # the skin wholly under cloth is left out: a vertex covered, with every neighbour covered too
    deep = under & np.array([all(under[j] for j in ring[i]) for i in range(n)])
    bm = bmesh.new(); bm.from_mesh(skin.data); bm.verts.ensure_lookup_table()
    gone = [f for f in bm.faces if all(deep[v.index] for v in f.verts)]
    total = len(bm.faces)
    bmesh.ops.delete(bm, geom=gone, context="FACES")
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
    bm.to_mesh(skin.data); bm.free()
    # with the body's other parts (a whole SOMA's eyes; a headless body has none)
    export(skin, os.path.join(out_dir, "body.glb"), [o for o in bpy.data.objects if o.type == "MESH" and o is not skin])
    print(f"clothes: the body without the skin wholly under cloth ({len(gone)} of {total} faces left out): {os.path.join(out_dir, 'body.glb')}")


# the most a garment measured from pictures stands off each part (metres; the trunk 8 cm)
STAND_CAP = {"upperArm": 0.03, "foreArm": 0.025, "hand": 0.01, "thigh": 0.04, "shin": 0.035, "foot": 0.015, "toes": 0.01, "neck": 0.03, "head": 0.03}
LIMB = {"Arm": ("upperArm", "ForeArm"), "ForeArm": ("foreArm", "Hand"), "Leg": ("thigh", "Shin"), "Shin": ("shin", "Foot"), "Foot": ("foot", "ToeBase")}
TRUNK = {"Hips": "hips", "Spine1": "waist", "Spine2": "torso", "Chest": "chest", "Shoulder": "chest", "Neck1": "neck", "Neck2": "neck", "ToeBase": "toes", "ToeEnd": "toes"}
PARTS = sorted(set(TRUNK.values()) | {v[0] for v in LIMB.values()} | {"hand", "head"})


def part_of(name):
    """a joint's part (hips, waist, torso, chest, neck, head, upperArm, foreArm, hand, thigh, shin, foot, toes) and,
    for a limb, the joint it starts at and the one it ends at"""
    side = "Left" if name.startswith("Left") else "Right" if name.startswith("Right") else ""
    j = name[len(side):]
    if "Twist" in j:  # a twist joint is its limb's
        j = j[:j.index("Twist")]
    if j.startswith("Hand"):
        return "hand", None
    if j in LIMB:
        return LIMB[j][0], (side + j, side + LIMB[j][1])
    return TRUNK.get(j, "head"), None


def cover(out, spec, colour, stand_m, under=None):
    """A garment by procedure, with no picture: the skin of the parts named, each limb as far along as its share
    says ("foreArm:0.5", the half nearest the elbow), in one colour, standing off the skin by stand_m (and off the
    layer beneath by a layer's thickness)."""
    want = {}
    for s in spec.split(","):
        p, _, share = s.strip().partition(":")
        if p not in PARTS:
            raise SystemExit(f"clothes: no part {p!r}; the parts are {', '.join(PARTS)}")
        want[p] = float(share) if share else 1.0
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    head = {b.name: np.array(arm.matrix_world @ b.head_local) for b in arm.data.bones}
    gname = {g.index: g.name for g in skin.vertex_groups}
    covered = np.zeros(n, bool)
    for i, v in enumerate(me.vertices):
        if not v.groups:
            continue
        top = gname[max(v.groups, key=lambda g: g.weight).group]
        p, limb = part_of(top)
        if p not in want:
            continue
        if limb and want[p] < 1.0 and limb[0] in head and limb[1] in head:
            a, b = head[limb[0]], head[limb[1]]
            t = float((co[i] - a) @ (b - a)) / max(float((b - a) @ (b - a)), 1e-9)
            if t > want[p]:
                continue
        covered[i] = True
    for _ in range(2):  # the edge evened without wearing it away: a vertex changes only when two thirds of its neighbours differ
        k = np.array([sum(covered[j] for j in ring[i]) / max(len(ring[i]), 1) for i in range(n)])
        covered = np.where(k > 2 / 3, True, np.where(k < 1 / 3, False, covered))
    # a hem allowance: one ring past the parts' edge, so garments on neighbouring parts overlap instead of leaving a gap
    covered = covered | np.array([any(covered[j] for j in ring[i]) for i in range(n)])
    rgb = [int(colour.lstrip("#")[k:k + 2], 16) / 255 for k in (0, 2, 4)]
    ids = sorted(set(int(c) for c in cid[covered]))
    stand = np.full(len(ids), max(stand_m, OFF))
    if under:
        u = np.zeros(len(C)); u[under["vertices"]] = under["stand"]
        stand = np.maximum(stand, u[ids] + THICK)
    g = {"name": os.path.splitext(os.path.basename(out))[0], "topology": "SOMA template, c_skin_mid, 18056 vertices", "units": "metres", "layer": (under["layer"] + 1) if under else 0,
         **({"over": under["name"]} if under else {}),
         "source": {"how": "raea clothes cover (tools/clothes/garment.py cover)", "parts": spec, "colour": colour},
         "vertices": ids, "stand": [round(float(x), 5) for x in stand], "colour_srgb": [[round(x, 4) for x in rgb]] * len(ids),
         "stray": [round(float(max(0.0, x - 0.012) * 0.6), 5) for x in stand]}
    json.dump(g, open(out, "w"), separators=(",", ":"))
    print(f"clothes: {g['name']}: layer {g['layer']}, {len(ids)} of {len(C)} template vertices covered ({spec}), {colour}, {stand.min() * 1000:.1f} to {stand.max() * 1000:.1f} mm off what is under it")


if mode == "measure":
    views_dir, out = args[3], args[4]
    measure(views_dir, out, json.load(open(args[5])) if len(args) > 5 else None, args[6] if len(args) > 6 else None)
elif mode == "cover":
    cover(args[3], args[4], args[5], float(args[6]), json.load(open(args[7])) if len(args) > 7 else None)
else:
    wear(args[3], args[4:])
