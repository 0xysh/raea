# The mannequin clothes are drawn on (raea docs/character.feature CH40): SOMA's body at rest, flat dark grey on
# mid grey, without perspective, from the front, the back, its left and its right, the four views in one picture so
# an image model dresses all of them as one garment. The cameras are written beside it, so what is drawn can be laid
# back on the body. `raea clothes` makes the machine's views once, in raea's models folder (soma/mannequin).
#   raea blender run tools/clothes/mannequin.py BODY.glb OUT_DIR [SIZE] [PIECE.glb...]
# With PIECEs (a dressed body's garments, `raea clothes wear`), the mannequin is drawn wearing them in their own
# colours, and the views are OUT_DIR/{front,back,left,right}.jpg: what a garment made by procedure looks like, for
# one drawn over it.
import sys, os, json, math
import bpy
from mathutils import Vector, Matrix

a = sys.argv[sys.argv.index("--") + 1:]
src, out, size, pieces = a[0], a[1], int(a[2]) if len(a) > 2 else 1024, a[3:]
os.makedirs(out, exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
for o in [o for o in bpy.data.objects if o.type == "MESH" and not any(m.type == "ARMATURE" for m in o.modifiers)]:
    bpy.data.objects.remove(o)
for o in bpy.data.objects:  # the body's own parts in the mannequin's grey (workbench draws a material's viewport colour)
    if o.type == "MESH":
        for m in o.data.materials:
            m.diffuse_color = (0.22, 0.22, 0.22, 1)
for p in pieces:
    bpy.ops.import_scene.gltf(filepath=p)
sc = bpy.context.scene
sc.render.engine = "BLENDER_WORKBENCH"
sh = sc.display.shading
sh.light = "STUDIO"; sh.color_type = "TEXTURE" if pieces else "SINGLE"; sh.single_color = (0.22, 0.22, 0.22); sh.show_cavity = True; sh.cavity_type = "BOTH"; sh.show_shadows = False
sc.world = bpy.data.worlds.new("w"); sc.world.color = (0.578, 0.578, 0.578)
sc.render.film_transparent = False
sc.view_settings.view_transform = "Standard"
sc.render.resolution_x = sc.render.resolution_y = size
cam_d = bpy.data.cameras.new("c"); cam_d.type = "ORTHO"; cam_d.ortho_scale = 2.0
cam = bpy.data.objects.new("c", cam_d); sc.collection.objects.link(cam); sc.camera = cam
MID = 0.9  # metres up: the frame is 2 m tall, the floor 10 cm above its bottom
views = {"front": (0, -4), "back": (0, 4), "left": (4, 0), "right": (-4, 0)}  # the body faces -Y in Blender; +X is its left
cams = {}
for name, (x, y) in views.items():
    cam.location = (x, y, MID)
    cam.rotation_euler = (Vector((0, 0, MID)) - cam.location).to_track_quat("-Z", "Y").to_euler()
    if pieces:
        sc.render.image_settings.file_format = "JPEG"; sc.render.image_settings.quality = 95
    sc.render.filepath = os.path.join(out, f"{name}.jpg" if pieces else f"mannequin_{name}.png")
    bpy.ops.render.render(write_still=True)
    # raea's frame: y up, +z ahead, x its left
    cams[name] = {"from": [x, MID, -y], "at": [0, MID, 0], "height_m": 2.0}
if not pieces:
    json.dump({"units": "metres, Y up, +Z ahead", "orthographic": True, "size": size, "views": cams}, open(os.path.join(out, "mannequin.cam.json"), "w"), indent=1)
print("clothes: the mannequin's four views", out)
