//! A colour a copy over an <Instances>' shared surface (three's instanceColor, docs/r3f.feature R3F20a): the standard
//! material extended by a storage buffer of the copies' linear colours, which multiply its base colour before the light.
//! Every copy holds the one TintMaterial, so Bevy's automatic batching still draws the set as one instanced call; a
//! copy finds its colour by its MeshTag above the two low bits (compose.rs keeps a mesh's composition pass in those).
//!
//! scene3d's instance.rs makes it for a mesh row with `inst_tint` 1 and rewrites the buffer when the frame's list
//! changes; the standard material it extends is the row's own, copied in again when that changes.

use super::*;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin};
use bevy::render::render_resource::AsBindGroup;
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;

pub type TintMaterial = ExtendedMaterial<StandardMaterial, Tint>;

pub fn install(app: &mut App) {
    bevy::asset::embedded_asset!(app, "tint.wesl");
    app.add_plugins(MaterialPlugin::<TintMaterial>::default());
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct Tint {
    /// each copy's linear rgba, by its MeshTag >> 2
    #[storage(100, read_only)]
    pub colours: Handle<ShaderBuffer>,
}

impl MaterialExtension for Tint {
    fn fragment_shader() -> ShaderRef {
        "embedded://raea_sys_material/tint.wesl".into()
    }
}

/// The copies' colours as the buffer holds them: linear rgba, alpha 1. A buffer of the same size is written in place
/// on the GPU (GpuShaderBuffer reuses it), so the material's bind group stands.
pub fn colour_buffer(rgb: &[[f32; 3]]) -> ShaderBuffer {
    let mut words: Vec<[f32; 4]> = Vec::with_capacity(rgb.len());
    for &[r, g, b] in rgb {
        words.push([r, g, b, 1.0]);
    }
    ShaderBuffer::new(words, bevy::asset::RenderAssetUsages::default())
}

/// A copy's MeshTag: its number above the two bits a composition pass keeps
pub fn copy_tag(copy: usize, pass: u32) -> u32 {
    ((copy as u32) << 2) | (pass & 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_tag_keeps_the_pass_bits() {
        assert_eq!(copy_tag(5, 2), 22);
        assert_eq!(copy_tag(5, 2) >> 2, 5);
        assert_eq!(copy_tag(5, 2) & 3, 2);
    }
}
