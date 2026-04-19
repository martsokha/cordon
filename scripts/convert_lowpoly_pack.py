"""
Batch-convert the LowPolyAssetPack_Free FBX files to GLB.

Usage (requires Blender 4.2+):
    blender --background --python scripts/convert_lowpoly_pack.py

The pack ships every mesh with a shared tiny atlas at
`TextureMap/MasterMaterial1px.png` — each mesh samples a single
pixel per face. The FBX files themselves have no baked materials
(see NOTES.txt in the pack). This script:

  1. Loads the atlas once as a Blender Image with pixel-art
     ("Closest") filtering.
  2. Walks the pack, imports each .fbx into a fresh scene,
     assigns an atlas-backed material to every mesh in the
     import, and exports a single .glb per source file.
  3. Flattens the directory tree — output filenames are the
     source basename with the `Mesh_` prefix stripped.

Output lands in `assets/models/lowpoly/`. Existing .glb files
are overwritten.
"""

import bpy
import glob
import os
import sys

# Edit these two if the pack lives elsewhere.
SRC_PACK = "/Users/martsokha/Downloads/LowPolyAssetPack_Free"
FBX_ROOT = os.path.join(SRC_PACK, "FBX and Blend Files")
ATLAS_SRC = os.path.join(SRC_PACK, "TextureMap", "MasterMaterial1px.png")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, "assets", "models", "lowpoly")


def clear_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def build_atlas_material(atlas_path):
    """Create a material with the atlas as base-colour, sampled
    'Closest' so pixel colours don't bleed into each other."""
    image = bpy.data.images.load(atlas_path, check_existing=True)
    # Pixel-art atlas: one pixel per mesh region, so any bilinear
    # filtering smears neighbour colours across UV islands.
    image.colorspace_settings.name = "sRGB"

    mat = bpy.data.materials.new(name="LowPolyAtlas")
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    links = mat.node_tree.links

    for n in list(nodes):
        nodes.remove(n)

    bsdf = nodes.new("ShaderNodeBsdfPrincipled")
    output = nodes.new("ShaderNodeOutputMaterial")
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = image
    tex.interpolation = "Closest"

    links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    links.new(bsdf.outputs["BSDF"], output.inputs["Surface"])

    bsdf.inputs["Roughness"].default_value = 0.75
    bsdf.inputs["Metallic"].default_value = 0.0

    return mat


def assign_material_to_meshes(mat):
    """Replace every imported mesh object's material slot with
    the shared atlas material, inserting one if the mesh has
    none."""
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        me = obj.data
        if not me.materials:
            me.materials.append(mat)
        else:
            for i in range(len(me.materials)):
                me.materials[i] = mat


def out_name_for(fbx_path):
    """Flatten the source tree and strip the `Mesh_` prefix.
    Example: .../Shelves/Shelf_03/Mesh_Shelf_03_Tall.fbx ->
    Shelf_03_Tall.glb"""
    base = os.path.splitext(os.path.basename(fbx_path))[0]
    if base.startswith("Mesh_"):
        base = base[len("Mesh_"):]
    return base + ".glb"


def convert(fbx_path, atlas_path):
    out_path = os.path.join(OUT_DIR, out_name_for(fbx_path))

    clear_scene()

    rel = os.path.relpath(fbx_path, FBX_ROOT)
    print(f"  importing {rel} ...")
    bpy.ops.import_scene.fbx(filepath=fbx_path)

    mat = build_atlas_material(atlas_path)
    assign_material_to_meshes(mat)

    print(f"  exporting {os.path.basename(out_path)} ...")
    bpy.ops.export_scene.gltf(
        filepath=out_path,
        export_format="GLB",
        export_texcoords=True,
        export_normals=True,
        export_materials="EXPORT",
        export_image_format="AUTO",
        export_yup=True,
    )


def main():
    if not os.path.isdir(FBX_ROOT):
        print(f"Pack FBX root not found: {FBX_ROOT}")
        sys.exit(1)
    if not os.path.isfile(ATLAS_SRC):
        print(f"Atlas not found: {ATLAS_SRC}")
        sys.exit(1)

    os.makedirs(OUT_DIR, exist_ok=True)

    fbx_files = sorted(
        glob.glob(os.path.join(FBX_ROOT, "**", "*.fbx"), recursive=True)
    )
    if not fbx_files:
        print(f"No .fbx files found under {FBX_ROOT}")
        sys.exit(1)

    print(f"Found {len(fbx_files)} .fbx files.")
    print(f"Output directory: {OUT_DIR}\n")

    for path in fbx_files:
        convert(path, ATLAS_SRC)

    print(f"\nConverted {len(fbx_files)} files.")


if __name__ == "__main__":
    main()
