# The body with the skin wholly under cloth left out, so it can't poke through as it moves.
#   raea blender run hide_under.py BODY.glb OUT.glb GARMENT.glb...
# A body vertex is under cloth when a ray out along its normal meets a garment within REACH (6 cm); a face goes when
# its vertices and all their neighbours are under cloth (the edge of the cut stays, so no gap shows at a hem).
import bpy, bmesh, sys, os
from mathutils import Vector
from mathutils.bvhtree import BVHTree

a = sys.argv[sys.argv.index("--") + 1:]
body_glb, out, garments = a[0], a[1], a[2:]
REACH = float(os.environ.get("REACH", "0.06"))
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=body_glb)
skin = max((o for o in bpy.data.objects if o.type == "MESH"), key=lambda o: len(o.data.vertices))
keep = set(bpy.data.objects)
dg = bpy.context.evaluated_depsgraph_get()
cb = bmesh.new()
for gpath in garments:
    before = set(bpy.data.objects); bpy.ops.import_scene.gltf(filepath=gpath)
    for o in set(bpy.data.objects) - before:
        if o.type == "MESH":
            t = bmesh.new(); t.from_object(o, bpy.context.evaluated_depsgraph_get()); t.transform(o.matrix_world)
            m = bpy.data.meshes.new("t"); t.to_mesh(m); t.free(); cb.from_mesh(m)
bvh = BVHTree.FromBMesh(cb)
W = skin.matrix_world; N = W.to_3x3()
bm = bmesh.new(); bm.from_mesh(skin.data); bm.verts.ensure_lookup_table()
under = [False] * len(bm.verts)
for v in bm.verts:
    p = W @ v.co; nrm = (N @ v.normal).normalized()
    hit = bvh.ray_cast(p + nrm * 0.0005, nrm, REACH)
    under[v.index] = hit[0] is not None
deep = [under[v.index] and all(under[e.other_vert(v).index] for e in v.link_edges) for v in bm.verts]
gone = [f for f in bm.faces if all(deep[v.index] for v in f.verts)]
total = len(bm.faces)
bmesh.ops.delete(bm, geom=gone, context="FACES")
bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
bm.to_mesh(skin.data); bm.free()
for o in bpy.data.objects:
    o.select_set(o in keep and o.type in ("MESH", "ARMATURE"))
bpy.ops.export_scene.gltf(filepath=out, export_format="GLB", use_selection=True, export_animations=False, export_yup=True, export_skins=True)
print(f"hide_under: {len(gone)} of {total} faces under cloth left out: {out}")
