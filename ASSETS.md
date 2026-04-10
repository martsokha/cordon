# Asset Registry

Third-party assets used in Cordon. Each entry lists the source,
license, and where the converted files end up in `assets/`.

## Models

| Asset | Source | License | Raw | Runtime |
|---|---|---|---|---|
| Low-Poly Furniture Pack | (itch.io) | See itch.io page | `raw/models/interior/*.fbx` | `assets/models/interior/*.glb` |
| Low-Poly Storage Pack | [brokenvector (itch.io)](https://brokenvector.itch.io/low-poly-storage-pack) | See itch.io page | `raw/models/storage/*.dae` | `assets/models/storage/*.glb` |
| Surveillance Camera | [oxygen3d (itch.io)](https://oxygen3d.itch.io/game-ready-surveillance-camera-asset) | See itch.io page | `raw/models/cctv/` | `assets/models/cctv/camera.glb` |

## Workflow

Source models (.fbx, .dae) live in `raw/models/`. The Makefile
converts them to .glb in `assets/models/` using assimp.

```sh
# Install assimp (one-time):
brew install assimp

# Convert all models:
make models

# Clean generated GLBs:
make clean
```

Only changed files are reconverted (Make tracks dependencies).

To load a model in code:

```rust
let scene = asset_server.load("models/interior/Laptop.glb#Scene0");
commands.spawn(SceneRoot(scene));
```

Directory layout:

```
raw/                              ← source assets
  models/
    interior/*.fbx                ← low-poly furniture
    storage/*.dae                 ← storage/industrial props
    cctv/                         ← surveillance camera + textures

assets/                           ← runtime assets (committed)
  models/
    interior/*.glb
    storage/*.glb
    cctv/camera.glb
```
