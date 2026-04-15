//! PBR texture sets for the bunker palette.
//!
//! Each set bundles the handles a [`StandardMaterial`] needs for a
//! full physically-based look: sRGB base colour, linear normal map
//! (OpenGL convention), linear roughness (loaded into
//! `metallic_roughness_texture`'s G channel — see the shader
//! comment below), and linear ambient occlusion.
//!
//! **Metallic channel**: ambientCG ships metallic as a separate
//! image, but Bevy/glTF expects metallic + roughness packed into
//! one RGBA image (B = metallic, G = roughness). Rather than
//! combining images at load time (async dance), we load the
//! roughness map alone into `metallic_roughness_texture` and set
//! `StandardMaterial::metallic = 0.0` for non-metal materials.
//! Final metallic is then `metallic_scalar * texture.b = 0`, so
//! the B-channel signal is zeroed out and the G-channel roughness
//! still drives surface detail. Clean — no compositing needed.
//!
//! **Normal map convention**: ambientCG ships both `_NormalDX` and
//! `_NormalGL` variants. Bevy/glTF use the OpenGL (+Y up) encoding,
//! so we always load `_NormalGL`.

use bevy::asset::RenderAssetUsages;
use bevy::asset::io::file::FileAssetReader;
use bevy::image::{ImageLoaderSettings, ImageSampler};
use bevy::prelude::*;

/// One PBR texture set — the collection of per-material maps that
/// a single [`StandardMaterial`] samples from. Each field is
/// optional because not every ambientCG download ships every map
/// (Concrete024, for instance, has no AO or metalness image).
/// Pointing Bevy's asset loader at a non-existent file produces a
/// live `Handle<Image>` that silently fails to load — the shader
/// then samples a default texture and the material renders wrong
/// (in particular, missing AO on a cuboid mesh caused walls to
/// render transparent). Loading only the files that exist keeps
/// missing maps from leaking in.
#[derive(Clone)]
pub(crate) struct TextureSet {
    pub base_color: Handle<Image>,
    pub normal: Option<Handle<Image>>,

    /// Loaded from the roughness image alone; see module docs for
    /// the metallic-channel rationale.
    pub metallic_roughness: Option<Handle<Image>>,
    pub ambient_occlusion: Option<Handle<Image>>,
    /// Height map for parallax mapping. When present, the PBR
    /// shader offsets UV samples by perceived depth so flat
    /// surfaces read as having real relief (great for rough
    /// concrete's pour marks).
    pub depth: Option<Handle<Image>>,
}

impl TextureSet {
    /// Load an ambientCG-style texture set from a folder.
    ///
    /// `folder` is the path under `assets/textures/` (e.g.
    /// `"Concrete044C_1K-JPG"`). `basename` is the file prefix
    /// inside the folder — matches the folder name for ambientCG.
    ///
    /// Optional maps (normal, roughness, AO) are included only if
    /// the corresponding file exists on disk.
    pub(crate) fn load_ambient_cg(asset_server: &AssetServer, folder: &str, basename: &str) -> Self {
        let base_color = asset_server.load_with_settings(
            format!("textures/{folder}/{basename}_Color.jpg"),
            repeat_sampler_srgb,
        );
        let normal = asset_if_exists(folder, basename, "NormalGL")
            .map(|p| asset_server.load_with_settings(p, repeat_sampler_linear));
        let metallic_roughness = asset_if_exists(folder, basename, "Roughness")
            .map(|p| asset_server.load_with_settings(p, repeat_sampler_linear));
        let ambient_occlusion = asset_if_exists(folder, basename, "AmbientOcclusion")
            .map(|p| asset_server.load_with_settings(p, repeat_sampler_linear));
        let depth = asset_if_exists(folder, basename, "Displacement")
            .map(|p| asset_server.load_with_settings(p, repeat_sampler_linear));

        Self {
            base_color,
            normal,
            metallic_roughness,
            ambient_occlusion,
            depth,
        }
    }
}

/// Resolve an ambientCG map path (e.g. `Color`, `NormalGL`,
/// `AmbientOcclusion`) to an `assets/`-relative path iff the file
/// is actually on disk. Returns `None` otherwise so the caller
/// can skip binding a missing texture.
///
/// Uses [`FileAssetReader::get_base_path`] to locate the asset
/// root the same way Bevy's own loader does, then joins `assets/`
/// (the default sub-directory set by `AssetPlugin`). This stays
/// correct under `cargo run` (manifest dir), a distributed
/// binary (exe parent), and any `BEVY_ASSET_ROOT` override.
fn asset_if_exists(folder: &str, basename: &str, suffix: &str) -> Option<String> {
    let rel = format!("textures/{folder}/{basename}_{suffix}.jpg");
    let abs = FileAssetReader::get_base_path().join("assets").join(&rel);
    abs.exists().then_some(rel)
}

/// Image-load settings for textures that will be tiled across a
/// surface: addressing mode `Repeat` on both axes, sRGB colour
/// space (for base-colour maps).
fn repeat_sampler_srgb(settings: &mut ImageLoaderSettings) {
    settings.is_srgb = true;
    settings.sampler = ImageSampler::Descriptor(repeat_descriptor());
    settings.asset_usage = RenderAssetUsages::RENDER_WORLD;
}

/// Same as [`repeat_sampler_srgb`] but for linear-space textures
/// (normal / roughness / metallic / AO). Getting `is_srgb` right
/// is load-bearing — Bevy's loader defaults to sRGB, which would
/// mis-gamma-correct linear data.
fn repeat_sampler_linear(settings: &mut ImageLoaderSettings) {
    settings.is_srgb = false;
    settings.sampler = ImageSampler::Descriptor(repeat_descriptor());
    settings.asset_usage = RenderAssetUsages::RENDER_WORLD;
}

fn repeat_descriptor() -> bevy::image::ImageSamplerDescriptor {
    let mut d = bevy::image::ImageSamplerDescriptor::linear();
    d.address_mode_u = bevy::image::ImageAddressMode::Repeat;
    d.address_mode_v = bevy::image::ImageAddressMode::Repeat;
    d.address_mode_w = bevy::image::ImageAddressMode::Repeat;
    d
}
