# Asset Registry

Third-party assets used in Cordon. Each entry lists the source,
license, and where the converted files end up in `assets/`.

## Models

| Asset | Source | License | Raw | Runtime |
|---|---|---|---|---|
| Low-Poly Furniture Pack | (itch.io) | See itch.io page | `raw/models/interior/*.fbx` | `assets/models/interior/*.glb` |
| Surveillance Camera | [oxygen3d (itch.io)](https://oxygen3d.itch.io/game-ready-surveillance-camera-asset) | See itch.io page | `raw/models/cctv/` | `assets/models/cctv/camera.glb` |

## Workflow

Source FBX files and textures live in `raw/models/` (gitignored).
The Blender conversion script exports `.glb` files into the
matching path under `assets/models/`, which is what Bevy loads at
runtime.

```sh
blender --background --python scripts/convert_fbx_to_glb.py
```

This walks `raw/models/` recursively and writes a `.glb` next to
every `.fbx` under `assets/models/`. Textures referenced by the
FBX are embedded into the GLB by Blender automatically.

To load a model in code:

```rust
let scene = asset_server.load("models/interior/Laptop.glb#Scene0");
commands.spawn(SceneRoot(scene));
```

Directory layout:

```
raw/                              ← gitignored source assets
  models/
    interior/
      Laptop.fbx
      WoodenChair.fbx
      ...
    cctv/
      camera.fbx
      *.jpg

assets/                           ← committed runtime assets
  models/
    interior/
      Laptop.glb                  ← loaded by Bevy
      WoodenChair.glb
      ...
    cctv/
      camera.glb
```
