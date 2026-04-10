# Asset Registry

Third-party assets used in Cordon. Each entry lists the source,
license, and where the converted files end up in `assets/`.

## Models

| Asset | Source | License | Raw | Runtime |
|---|---|---|---|---|
| Low-Poly Furniture Pack | (itch.io) | See itch.io page | `raw/models/interior/*.fbx` | `assets/models/interior/*.glb` |
| Low-Poly Storage Pack | [brokenvector (itch.io)](https://brokenvector.itch.io/low-poly-storage-pack) | See itch.io page | `raw/models/storage/*.dae` | `assets/models/storage/*.glb` |
| Surveillance Camera | [oxygen3d (itch.io)](https://oxygen3d.itch.io/game-ready-surveillance-camera-asset) | See itch.io page | `raw/models/cctv/` | `assets/models/cctv/camera.glb` |

## Conversion

Two Blender scripts convert source models to GLB:

```sh
# FBX → GLB (Blender 5.1+)
blender --background --python scripts/convert_fbx_to_glb.py

# DAE → GLB with palette texture (Blender 4.2 LTS)
"/Applications/Blender 4.2 LTS.app/Contents/MacOS/Blender" \
    --background --python scripts/convert_dae_to_glb.py
```

The DAE script assigns `Palette_Green.png` to every material's
base color before export. To use a different palette, edit the
`PALETTE` variable in `scripts/convert_dae_to_glb.py`.

## Directory layout

```
raw/                              ← source assets
  models/
    interior/*.fbx                ← low-poly furniture
    storage/*.dae                 ← storage/industrial props
  textures/storage/Palette_*.png   ← color palettes for storage
    cctv/                         ← surveillance camera + textures

assets/                           ← runtime assets (committed)
  models/
    interior/*.glb
    storage/*.glb
    cctv/camera.glb
```
