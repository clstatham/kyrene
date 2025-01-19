use kyrene_asset::{Handle, Load, LoadSource, WorldAssets};
use kyrene_core::prelude::{tokio, WorldHandle};
use kyrene_graphics::{
    color::Color,
    texture::{Texture, TextureLoader},
};

pub struct Material {
    pub albedo: Color,
    pub diffuse: Handle<Texture>,
    pub normal: Handle<Texture>,
    pub specular: Handle<Texture>,
    pub ambient_occlusion: Handle<Texture>,
    pub emissive: Handle<Texture>,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub ambient_occlusion_factor: f32,
    pub emissive_factor: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            albedo: Color::WHITE,
            diffuse: Handle::INVALID,
            normal: Handle::INVALID,
            specular: Handle::INVALID,
            ambient_occlusion: Handle::INVALID,
            emissive: Handle::INVALID,
            roughness_factor: 0.0,
            metallic_factor: 0.0,
            ambient_occlusion_factor: 0.0,
            emissive_factor: 0.0,
        }
    }
}

#[derive(Default)]
pub struct ObjMaterialLoader;

impl Load for ObjMaterialLoader {
    type Asset = Vec<Material>;
    type Error = tobj::LoadError;

    async fn load(
        &self,
        source: LoadSource,
        world: WorldHandle,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = match source {
            LoadSource::Path(path) => tokio::fs::read(path)
                .await
                .map_err(|_| tobj::LoadError::ReadError)?,
            LoadSource::Bytes(bytes) => bytes,
            LoadSource::Existing(asset) => return Ok(asset.downcast().unwrap()),
        };

        let mut reader = std::io::Cursor::new(bytes);

        let (_, materials) = tobj::load_obj_buf(
            &mut reader,
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
            move |mat_path| tobj::load_mtl(mat_path),
        )?;

        let mut obj_materials = Vec::new();

        for material in materials? {
            let diffuse = if let Some(diffuse) = material.diffuse_texture {
                world
                    .load_asset::<TextureLoader>(LoadSource::Path(diffuse.into()))
                    .await
            } else {
                Handle::INVALID
            };

            let normal = if let Some(normal) = material.normal_texture {
                world
                    .load_asset::<TextureLoader>(LoadSource::Path(normal.into()))
                    .await
            } else {
                Handle::INVALID
            };

            let specular = if let Some(specular) = material.specular_texture {
                world
                    .load_asset::<TextureLoader>(LoadSource::Path(specular.into()))
                    .await
            } else {
                Handle::INVALID
            };

            let ambient_occlusion = if let Some(ambient_occlusion) = material.ambient_texture {
                world
                    .load_asset::<TextureLoader>(LoadSource::Path(ambient_occlusion.into()))
                    .await
            } else {
                Handle::INVALID
            };

            let albedo_base = material.diffuse.unwrap_or([1.0, 1.0, 1.0]);
            let albedo = Color::new(albedo_base[0], albedo_base[1], albedo_base[2], 1.0);

            let roughness_factor = 0.0;
            let metallic_factor = material.shininess.unwrap_or(0.0);

            let ambient_occlusion_factor = 1.0;

            obj_materials.push(Material {
                albedo,
                diffuse,
                normal,
                specular,
                ambient_occlusion,
                emissive: Handle::INVALID,
                roughness_factor,
                metallic_factor,
                ambient_occlusion_factor,
                emissive_factor: 0.0,
            });
        }

        Ok(obj_materials)
    }
}
