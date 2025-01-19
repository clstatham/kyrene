use encase::ShaderType;
use kyrene_asset::{Load, LoadSource};
use kyrene_core::prelude::{tokio, WorldHandle};

use crate::geom::{Point3, Vec2, Vec3};

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct Vertex {
    pub position: Point3,
    pub normal: Vec3,
    pub tex_coords: Vec2,
}

#[derive(Default)]
pub struct ObjMeshLoader;

impl Load for ObjMeshLoader {
    type Asset = Vec<Mesh>;
    type Error = tobj::LoadError;

    async fn load(
        &self,
        source: LoadSource,
        _world: WorldHandle,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = match source {
            LoadSource::Path(path) => tokio::fs::read(path)
                .await
                .map_err(|_| tobj::LoadError::ReadError)?,
            LoadSource::Bytes(bytes) => bytes,
            LoadSource::Existing(asset) => return Ok(asset.downcast().unwrap()),
        };

        let mut reader = std::io::Cursor::new(bytes);

        let (models, _) = tokio::task::spawn_blocking(move || {
            tobj::load_obj_buf(
                &mut reader,
                &tobj::LoadOptions {
                    single_index: true,
                    triangulate: true,
                    ..Default::default()
                },
                move |mat_path| tobj::load_mtl(mat_path),
            )
        })
        .await
        .unwrap()?;

        let mut meshes = Vec::new();

        for model in models {
            let mesh = model.mesh;

            let mut vertices = Vec::with_capacity(mesh.positions.len() / 3);
            let mut indices = Vec::with_capacity(mesh.indices.len());

            for i in 0..mesh.positions.len() / 3 {
                let position = Point3::new(
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                );

                let normal = if !mesh.normals.is_empty() {
                    Vec3::new(
                        mesh.normals[i * 3],
                        mesh.normals[i * 3 + 1],
                        mesh.normals[i * 3 + 2],
                    )
                } else {
                    Vec3::ZERO
                };

                let tex_coords = if !mesh.texcoords.is_empty() {
                    Vec2::new(mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1])
                } else {
                    Vec2::ZERO
                };

                vertices.push(Vertex {
                    position,
                    normal,
                    tex_coords,
                });
            }

            for i in 0..mesh.indices.len() {
                indices.push(mesh.indices[i]);
            }

            meshes.push(Mesh { vertices, indices });
        }

        Ok(meshes)
    }
}
