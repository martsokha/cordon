"""
Batch-convert DAE files to GLB with a palette texture assigned.

Usage (requires Blender 4.2 LTS):
    "/Applications/Blender 4.2 LTS.app/Contents/MacOS/Blender" \
        --background --python scripts/convert_dae_to_glb.py

Walks raw/models/ for .dae files, imports each into a fresh scene,
assigns the palette texture to every material's base color, and
exports as GLB to the matching path under assets/models/.

The palette texture is raw/models/storage/Palette_Green.png by
default. Change PALETTE below to use a different one.
"""

import bpy
import os
import sys
import glob

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW_DIR = os.path.join(ROOT, "raw", "models")
OUT_DIR = os.path.join(ROOT, "assets", "models")
PALETTE = os.path.join(ROOT, "raw", "textures", "storage", "Palette_Green.png")


def clear_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def convert(dae_path):
    rel = os.path.relpath(dae_path, RAW_DIR)
    glb_rel = os.path.splitext(rel)[0] + ".glb"
    glb_path = os.path.join(OUT_DIR, glb_rel)

    os.makedirs(os.path.dirname(glb_path), exist_ok=True)
    clear_scene()

    print(f"  importing {rel} ...")
    bpy.ops.wm.collada_import(filepath=dae_path)

    # Load the palette texture image (reuse if already loaded).
    palette_name = os.path.basename(PALETTE)
    if palette_name not in bpy.data.images:
        palette_img = bpy.data.images.load(PALETTE)
    else:
        palette_img = bpy.data.images[palette_name]

    # Assign the palette to every material in the scene.
    for mat in bpy.data.materials:
        if not mat.use_nodes:
            mat.use_nodes = True
        tree = mat.node_tree
        nodes = tree.nodes
        links = tree.links

        # Find or create the Principled BSDF node.
        bsdf = None
        for node in nodes:
            if node.type == "BSDF_PRINCIPLED":
                bsdf = node
                break
        if bsdf is None:
            bsdf = nodes.new("ShaderNodeBsdfPrincipled")

        # Create an image texture node and connect it.
        tex_node = nodes.new("ShaderNodeTexImage")
        tex_node.image = palette_img

        # Connect texture color → base color.
        links.new(tex_node.outputs["Color"], bsdf.inputs["Base Color"])

    print(f"  exporting {glb_rel} ...")
    bpy.ops.export_scene.gltf(
        filepath=glb_path,
        export_format="GLB",
        export_texcoords=True,
        export_normals=True,
        export_materials="EXPORT",
        export_image_format="AUTO",
        export_yup=True,
    )
    print(f"  done.")


def main():
    if not os.path.exists(PALETTE):
        print(f"Palette texture not found: {PALETTE}")
        sys.exit(1)

    dae_files = sorted(glob.glob(os.path.join(RAW_DIR, "**", "*.dae"), recursive=True))
    if not dae_files:
        print(f"No .dae files found under {RAW_DIR}")
        sys.exit(1)

    print(f"Found {len(dae_files)} .dae files")
    print(f"Palette: {PALETTE}")
    print(f"Output: {OUT_DIR}\n")

    for path in dae_files:
        convert(path)

    print(f"\nConverted {len(dae_files)} files.")


if __name__ == "__main__":
    main()
