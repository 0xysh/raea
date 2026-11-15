# The garment data engine's last step: each GPU-draped garment (drape_mlx.py) becomes a worn garment and its pictures.
# The body loads once; per garment: seams welded, the cut as UVs, weights from the skin, skinned to SOMA's armature,
# exported as <name>.glb, and rendered on the mannequin from four sides (<name>_<view>.png).
#   raea blender run bake_batch.py BODY.glb DIR...      (each DIR holds one <name>.draped.npz)
# PICTURES=1: the pictures alone, as JPEG (<name>_<view>.jpg), no weights or glb: what a lean design needs for training
# (garment_embed.py reads them); a DIR that has them already is skipped.
import bpy, bmesh, sys, os, glob, json, zlib, time
import numpy as np
from mathutils import Vector

a = sys.argv[sys.argv.index("--") + 1:]
body_glb, dirs = a[0], a[1:]
PALETTE = ["#d9d4c7", "#2f3b52", "#8a3b2f", "#3f5a3c", "#c9a227", "#6b4f7a", "#1f1f1f", "#b8c4cc", "#a85d3d", "#e8e2d6"]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=body_glb)
for o in [o for o in bpy.data.objects if o.type == "MESH" and not any(m.type == "ARMATURE" for m in o.modifiers)]:
    bpy.data.objects.remove(o)
skin = max((o for o in bpy.data.objects if o.type == "MESH"), key=lambda o: len(o.data.vertices))
arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
sm = bpy.data.materials.new("skin"); sm.use_nodes = True
sb = next(x for x in sm.node_tree.nodes if x.type == "BSDF_PRINCIPLED"); sb.inputs["Base Color"].default_value = (0.55, 0.55, 0.57, 1)
sm.diffuse_color = (0.55, 0.55, 0.57, 1)  # Workbench draws the viewport colour, not the node's
skin.data.materials.clear(); skin.data.materials.append(sm)
sc = bpy.context.scene
sc.world = bpy.data.worlds.new("w"); sc.world.color = (0.92, 0.92, 0.9)
sc.render.engine = "BLENDER_WORKBENCH"; sc.display.shading.light = "STUDIO"; sc.display.shading.color_type = "MATERIAL"
sc.display.shading.show_cavity = True; sc.render.resolution_x = 512; sc.render.resolution_y = 768
cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam")); sc.collection.objects.link(cam); sc.camera = cam
cam.data.type = "ORTHO"; cam.data.ortho_scale = 2.0
VIEWS = {"front": (0, -4, 0.95), "threequarter": (2.8, -2.8, 0.95), "side": (4, 0, 0.95), "back": (0, 4, 0.95)}
PICS = os.environ.get("PICTURES") == "1"
if PICS: sc.render.image_settings.file_format = "JPEG"; sc.render.image_settings.quality = 88

for d in dirs:
    t0 = time.time()
    src = glob.glob(os.path.join(d, "*.draped.npz"))
    if not src: print(f"bake: {d}: nothing draped", flush=True); continue
    Z = np.load(src[0]); name = os.path.basename(src[0])[:-len(".draped.npz")]
    if PICS and all(os.path.exists(os.path.join(d, f"{name}_{v}.jpg")) for v in VIEWS): continue
    X = Z["X"].astype(float); X = np.c_[X[:, 0], -X[:, 2], X[:, 1]]  # glTF (x, y, z) is Blender (x, -z, y)
    me = bpy.data.meshes.new(name); me.from_pydata(X.tolist(), [], Z["F"].tolist()); me.update()
    g = bpy.data.objects.new(name, me); sc.collection.objects.link(g)
    bm = bmesh.new(); bm.from_mesh(me); uv = bm.loops.layers.uv.new("cut"); FLAT = Z["FLAT"]
    for fc in bm.faces:
        for l in fc.loops: l[uv].uv = FLAT[l.vert.index]
    bm.verts.ensure_lookup_table(); targets = {}
    for i, j in Z["seams"]:
        i, j = int(i), int(j)
        while i in targets: i = targets[i]
        while j in targets: j = targets[j]
        if j != i: targets[j] = i
    bmesh.ops.weld_verts(bm, targetmap={bm.verts[j]: bm.verts[i] for j, i in targets.items()})
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces); bm.to_mesh(me); bm.free()
    # smooth: a flat-shaded mesh exports every triangle with its own three vertices (198,881 for 66,305), shading
    # as facets ("pebbles") wherever it is drawn
    me.polygons.foreach_set("use_smooth", [True] * len(me.polygons)); me.update()
    if not PICS:
        for vg in skin.vertex_groups: g.vertex_groups.new(name=vg.name)
        for o in bpy.data.objects: o.select_set(o is g)
        bpy.context.view_layer.objects.active = g
        dt = g.modifiers.new("weights", "DATA_TRANSFER"); dt.object = skin
        dt.use_vert_data = True; dt.data_types_verts = {"VGROUP_WEIGHTS"}; dt.vert_mapping = "POLYINTERP_NEAREST"
        dt.layers_vgroup_select_src = "ALL"; dt.layers_vgroup_select_dst = "NAME"
        bpy.ops.object.modifier_apply(modifier=dt.name)
        g.parent = arm; g.modifiers.new("skin", "ARMATURE").object = arm
    colour = PALETTE[zlib.crc32(name.encode()) % len(PALETTE)]
    rgb = [int(colour[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    mat = bpy.data.materials.new(name); mat.use_nodes = True
    bsdf = next(x for x in mat.node_tree.nodes if x.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (*[x ** 2.2 for x in rgb], 1); bsdf.inputs["Roughness"].default_value = 0.9
    mat.diffuse_color = (*[x ** 2.2 for x in rgb], 1)
    me.materials.append(mat)
    if not PICS:
        for o in bpy.data.objects: o.select_set(o is g or o is arm)
        bpy.ops.export_scene.gltf(filepath=os.path.join(d, name + ".glb"), export_format="GLB", use_selection=True,
                                  export_animations=False, export_yup=True, export_skins=True)
    for view, loc in VIEWS.items():
        cam.location = loc; cam.rotation_euler = (Vector((0, 0, 0.95)) - Vector(loc)).to_track_quat("-Z", "Y").to_euler()
        sc.render.filepath = os.path.join(d, f"{name}_{view}.{'jpg' if PICS else 'png'}"); bpy.ops.render.render(write_still=True)
    welded = len(me.vertices)
    if not PICS:
        json.dump({"name": name, "colour": colour, "views": list(VIEWS), "vertices": welded},
                  open(os.path.join(d, name + ".bake.json"), "w"))
    bpy.data.objects.remove(g); bpy.data.meshes.remove(me); bpy.data.materials.remove(mat)
    print(f"bake: {name}: {len(X)} -> {welded} vertices, {colour}, {time.time() - t0:.1f} s", flush=True)
