"""
Batch-convert every .fbx file under raw/models/ to .glb in assets/models/.

Usage:
    blender --background --python scripts/convert_fbx_to_glb.py

The script walks raw/models/ recursively, imports each .fbx into a
fresh Blender scene, and exports a .glb into the corresponding path
under assets/models/. Existing .glb files are overwritten.

Directory structure:
    raw/models/interior/Laptop.fbx   -->  assets/models/interior/Laptop.glb
    raw/models/cctv/camera.fbx       -->  assets/models/cctv/camera.glb

The raw/ directory is gitignored. Only the .glb output in assets/
is committed.

Requires Blender 3.6+ (for the glTF exporter settings used here).
"""

import bpy
import os
import sys
import glob

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW_DIR = os.path.join(ROOT, "raw", "models")
OUT_DIR = os.path.join(ROOT, "assets", "models")


def clear_scene():
    """Remove every object, mesh, material, image, and action from the current file."""
    bpy.ops.wm.read_factory_settings(use_empty=True)


def convert(fbx_path):
    rel = os.path.relpath(fbx_path, RAW_DIR)
    glb_rel = os.path.splitext(rel)[0] + ".glb"
    glb_path = os.path.join(OUT_DIR, glb_rel)

    os.makedirs(os.path.dirname(glb_path), exist_ok=True)

    clear_scene()

    print(f"  importing {rel} ...")
    bpy.ops.import_scene.fbx(filepath=fbx_path)

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
    if not os.path.isdir(RAW_DIR):
        print(f"Source directory not found: {RAW_DIR}")
        print("Place .fbx files under raw/models/ and re-run.")
        sys.exit(1)

    fbx_files = sorted(glob.glob(os.path.join(RAW_DIR, "**", "*.fbx"), recursive=True))
    if not fbx_files:
        print(f"No .fbx files found under {RAW_DIR}")
        sys.exit(1)

    print(f"Found {len(fbx_files)} .fbx files under {RAW_DIR}")
    print(f"Output directory: {OUT_DIR}\n")

    for path in fbx_files:
        convert(path)

    print(f"\nConverted {len(fbx_files)} files.")


if __name__ == "__main__":
    main()
