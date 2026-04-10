#!/usr/bin/env python3
"""Measure local-space AABBs for every GLB in assets/models and
optionally emit a Rust prop-registry data table.

Reads the GLB JSON chunk, walks the scene graph from the default scene,
accumulates world-space AABBs by combining each mesh primitive's
POSITION accessor min/max with the parent chain of node transforms.

Usage:
    python3 inspect_glb_bounds.py          # human-readable table
    python3 inspect_glb_bounds.py --rust   # rust match arms for prop.rs
"""

from __future__ import annotations

import json
import math
import struct
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
MODELS = REPO / "assets" / "models"


def read_glb_json(path: Path) -> dict:
    with path.open("rb") as f:
        magic, version, _length = struct.unpack("<4sII", f.read(12))
        assert magic == b"glTF", f"not a GLB: {path}"
        assert version == 2, f"unsupported glTF version: {version}"
        chunk_len, chunk_type = struct.unpack("<I4s", f.read(8))
        assert chunk_type == b"JSON", f"first chunk not JSON in {path}"
        return json.loads(f.read(chunk_len))


def mat_identity() -> list[float]:
    return [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]


def mat_mul(a: list[float], b: list[float]) -> list[float]:
    # Column-major 4x4.
    r = [0.0] * 16
    for col in range(4):
        for row in range(4):
            s = 0.0
            for k in range(4):
                s += a[k * 4 + row] * b[col * 4 + k]
            r[col * 4 + row] = s
    return r


def trs_to_matrix(
    t: list[float] | None, r: list[float] | None, s: list[float] | None
) -> list[float]:
    tx, ty, tz = t or (0.0, 0.0, 0.0)
    qx, qy, qz, qw = r or (0.0, 0.0, 0.0, 1.0)
    sx, sy, sz = s or (1.0, 1.0, 1.0)
    # Rotation matrix from quaternion (column-major).
    xx, yy, zz = qx * qx, qy * qy, qz * qz
    xy, xz, yz = qx * qy, qx * qz, qy * qz
    wx, wy, wz = qw * qx, qw * qy, qw * qz
    m00 = (1 - 2 * (yy + zz)) * sx
    m01 = 2 * (xy + wz) * sx
    m02 = 2 * (xz - wy) * sx
    m10 = 2 * (xy - wz) * sy
    m11 = (1 - 2 * (xx + zz)) * sy
    m12 = 2 * (yz + wx) * sy
    m20 = 2 * (xz + wy) * sz
    m21 = 2 * (yz - wx) * sz
    m22 = (1 - 2 * (xx + yy)) * sz
    return [
        m00, m01, m02, 0.0,
        m10, m11, m12, 0.0,
        m20, m21, m22, 0.0,
        tx, ty, tz, 1.0,
    ]


def node_matrix(node: dict) -> list[float]:
    if "matrix" in node:
        return list(node["matrix"])  # already column-major
    return trs_to_matrix(
        node.get("translation"),
        node.get("rotation"),
        node.get("scale"),
    )


def transform_point(m: list[float], p: tuple[float, float, float]) -> tuple[float, float, float]:
    x, y, z = p
    rx = m[0] * x + m[4] * y + m[8] * z + m[12]
    ry = m[1] * x + m[5] * y + m[9] * z + m[13]
    rz = m[2] * x + m[6] * y + m[10] * z + m[14]
    return rx, ry, rz


def transform_aabb(
    m: list[float], lo: list[float], hi: list[float]
) -> tuple[list[float], list[float]]:
    # Transform 8 corners, take new min/max.
    corners = [
        (lo[0], lo[1], lo[2]),
        (hi[0], lo[1], lo[2]),
        (lo[0], hi[1], lo[2]),
        (hi[0], hi[1], lo[2]),
        (lo[0], lo[1], hi[2]),
        (hi[0], lo[1], hi[2]),
        (lo[0], hi[1], hi[2]),
        (hi[0], hi[1], hi[2]),
    ]
    xs = [transform_point(m, c) for c in corners]
    xlo = [min(c[i] for c in xs) for i in range(3)]
    xhi = [max(c[i] for c in xs) for i in range(3)]
    return xlo, xhi


def model_aabb(gltf: dict) -> tuple[list[float], list[float]] | None:
    meshes = gltf.get("meshes", [])
    accessors = gltf.get("accessors", [])
    nodes = gltf.get("nodes", [])
    scenes = gltf.get("scenes", [])
    scene = gltf.get("scene", 0)
    if not scenes:
        return None
    root_nodes = scenes[scene].get("nodes", [])

    lo = [math.inf, math.inf, math.inf]
    hi = [-math.inf, -math.inf, -math.inf]
    found = False

    def visit(node_idx: int, parent_mat: list[float]) -> None:
        nonlocal found
        node = nodes[node_idx]
        m = mat_mul(parent_mat, node_matrix(node))
        if "mesh" in node:
            mesh = meshes[node["mesh"]]
            for prim in mesh.get("primitives", []):
                pos_idx = prim.get("attributes", {}).get("POSITION")
                if pos_idx is None:
                    continue
                acc = accessors[pos_idx]
                if "min" not in acc or "max" not in acc:
                    continue
                plo, phi = transform_aabb(m, acc["min"], acc["max"])
                for i in range(3):
                    lo[i] = min(lo[i], plo[i])
                    hi[i] = max(hi[i], phi[i])
                found = True
        for child in node.get("children", []):
            visit(child, m)

    for n in root_nodes:
        visit(n, mat_identity())

    return (lo, hi) if found else None


def classify_origin(lo: list[float], hi: list[float]) -> str:
    # Is the model origin (0,0,0) at the base, the center, or neither?
    eps = 0.01
    height = hi[1] - lo[1]
    if abs(lo[1]) < eps:
        return "base"
    if abs(lo[1] + hi[1]) < eps * max(1.0, height):
        return "center"
    return f"offset(min_y={lo[1]:+.3f})"


# Names (stem only) that should spawn without a collider — thin / soft /
# tiny decoratives the player should walk through or over.
DECORATIVES = {
    "Bag_01", "Bag_02",
    "Rug",
    "Pillow",
    "Mug", "Bowl",
    "Cactus",
    "Kettle",
    "SoapBar",
    "TableMat",
    "GameBoard",
    "Vase1", "LargeVase", "VaseFlowers1",
    "Giftbox_01", "Giftbox_02", "Giftbox_03",
}


def variant_name(stem: str) -> str:
    """Convert a file stem like `StorageRack_01` or `EUR-Pallet` into a
    valid Rust CamelCase identifier."""
    # Split on non-alphanumeric.
    parts: list[str] = []
    buf = ""
    for ch in stem:
        if ch.isalnum():
            buf += ch
        else:
            if buf:
                parts.append(buf)
                buf = ""
    if buf:
        parts.append(buf)
    # Capitalize the first letter of each part, preserve the rest.
    out = ""
    for part in parts:
        out += part[0].upper() + part[1:] if part else ""
    # Numeric stems aren't valid; prefix with _ if needed (shouldn't happen).
    if out and out[0].isdigit():
        out = "_" + out
    return out


def scan() -> list[tuple[str, str, list[float], list[float]]]:
    """Return (path, stem, min, max) for every GLB with a measurable AABB."""
    rows: list[tuple[str, str, list[float], list[float]]] = []
    for p in sorted(MODELS.rglob("*.glb")):
        rel = p.relative_to(MODELS).as_posix()
        try:
            gltf = read_glb_json(p)
        except Exception as e:
            print(f"ERROR {rel}: {e}", file=sys.stderr)
            continue
        bounds = model_aabb(gltf)
        if bounds is None:
            continue
        lo, hi = bounds
        rows.append((rel, p.stem, lo, hi))
    return rows


def emit_rust(rows: list[tuple[str, str, list[float], list[float]]]) -> str:
    lines: list[str] = []
    lines.append("// @generated by scripts/inspect_glb_bounds.py --rust")
    lines.append("// DO NOT EDIT BY HAND. Re-run the script after asset changes.")
    lines.append("")
    lines.append("use bevy::prelude::*;")
    lines.append("")
    lines.append("/// Static prop registry. Each variant maps to a specific GLB")
    lines.append("/// asset and the local-space AABB measured from that GLB.")
    lines.append("///")
    lines.append("/// The registry deliberately includes every prop in `assets/models/`")
    lines.append("/// so rooms can pull from the full catalog without re-running the")
    lines.append("/// codegen script. Variants not yet placed are expected to be unused.")
    lines.append("#[allow(dead_code)]")
    lines.append("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]")
    lines.append("pub enum Prop {")
    for _rel, stem, _lo, _hi in rows:
        lines.append(f"    {variant_name(stem)},")
    lines.append("}")
    lines.append("")
    lines.append("pub struct PropDef {")
    lines.append("    /// Asset path, relative to the `assets/` directory.")
    lines.append("    pub path: &'static str,")
    lines.append("    /// Minimum corner of the AABB in the model's local space.")
    lines.append("    pub aabb_min: Vec3,")
    lines.append("    /// Maximum corner of the AABB in the model's local space.")
    lines.append("    pub aabb_max: Vec3,")
    lines.append("    /// Whether [`prop`] should spawn a static collider for this prop.")
    lines.append("    pub collider: bool,")
    lines.append("}")
    lines.append("")
    lines.append("#[allow(dead_code)]")
    lines.append("impl Prop {")
    lines.append("    pub const fn def(self) -> PropDef {")
    lines.append("        match self {")
    for rel, stem, lo, hi in rows:
        variant = variant_name(stem)
        collider = "false" if stem in DECORATIVES else "true"
        lines.append(f"            Self::{variant} => PropDef {{")
        lines.append(f"                path: \"models/{rel}\",")
        lines.append(
            f"                aabb_min: Vec3::new({lo[0]:.4}f32, {lo[1]:.4}f32, {lo[2]:.4}f32),"
        )
        lines.append(
            f"                aabb_max: Vec3::new({hi[0]:.4}f32, {hi[1]:.4}f32, {hi[2]:.4}f32),"
        )
        lines.append(f"                collider: {collider},")
        lines.append("            },")
    lines.append("        }")
    lines.append("    }")
    lines.append("")
    lines.append("    /// Size of the AABB along each local axis.")
    lines.append("    pub fn size(self) -> Vec3 {")
    lines.append("        let d = self.def();")
    lines.append("        d.aabb_max - d.aabb_min")
    lines.append("    }")
    lines.append("")
    lines.append("    /// Center of the AABB in the model's local space.")
    lines.append("    pub fn local_center(self) -> Vec3 {")
    lines.append("        let d = self.def();")
    lines.append("        (d.aabb_min + d.aabb_max) * 0.5")
    lines.append("    }")
    lines.append("}")
    lines.append("")
    return "\n".join(lines) + "\n"


def main() -> int:
    if not MODELS.exists():
        print(f"no models dir at {MODELS}", file=sys.stderr)
        return 1

    rust_mode = "--rust" in sys.argv[1:]
    rows = scan()

    if rust_mode:
        out = REPO / "crates" / "cordon-bevy" / "src" / "bunker" / "room" / "prop_registry.rs"
        out.write_text(emit_rust(rows))
        print(f"wrote {out}  ({len(rows)} props)")
        return 0

    header = f"{'model':45s} {'origin':18s} {'size (x,y,z)':24s} {'min':28s} {'max':28s}"
    print(header)
    print("-" * len(header))
    for rel, _stem, lo, hi in rows:
        size = [hi[i] - lo[i] for i in range(3)]
        origin = classify_origin(lo, hi)
        print(
            f"{rel:45s} {origin:18s} "
            f"({size[0]:+.3f},{size[1]:+.3f},{size[2]:+.3f})  "
            f"({lo[0]:+.3f},{lo[1]:+.3f},{lo[2]:+.3f})  "
            f"({hi[0]:+.3f},{hi[1]:+.3f},{hi[2]:+.3f})"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
