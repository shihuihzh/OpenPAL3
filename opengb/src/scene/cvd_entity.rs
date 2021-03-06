use crate::loaders::cvd_loader::*;
use radiance::math::{Vec2, Vec3};
use radiance::rendering::{RenderObject, SimpleMaterial, VertexBuffer, VertexComponents};
use radiance::scene::{CoreEntity, EntityExtension};
use std::path::PathBuf;

pub struct CvdModelEntity {
    texture_path: PathBuf,
    vertices: VertexBuffer,
    indices: Vec<u32>,
    id: u32,
}

impl CvdModelEntity {
    pub fn new(all_vertices: &Vec<CvdVertex>, material: &CvdMaterial, path: &str, id: u32) -> Self {
        let dds_name = material
            .texture_name
            .split_terminator('.')
            .next()
            .unwrap()
            .to_owned()
            + ".dds";
        let mut texture_path = PathBuf::from(path);
        texture_path.pop();
        texture_path.push(&dds_name);
        if !texture_path.exists() {
            texture_path.pop();
            texture_path.push(&material.texture_name);
        }

        let components =
            VertexComponents::POSITION /*| VertexComponents::NORMAL*/ | VertexComponents::TEXCOORD;

        let mut index_map = std::collections::HashMap::new();
        let mut reversed_index = vec![];
        let mut get_new_index = |index: u16| -> u32 {
            if index_map.contains_key(&index) {
                index_map[&index]
            } else {
                let new_index = reversed_index.len() as u32;
                reversed_index.push(index as usize);
                index_map.insert(index, new_index);
                new_index
            }
        };

        let mut indices: Vec<u32> = vec![];
        for t in material.triangles.as_ref().unwrap() {
            indices.push(get_new_index(t.indices[0]));
            indices.push(get_new_index(t.indices[1]));
            indices.push(get_new_index(t.indices[2]));
        }

        let mut vertices = VertexBuffer::new(components, reversed_index.len());
        for i in 0..reversed_index.len() {
            let vert = &all_vertices[reversed_index[i]];
            vertices.set_data(
                i,
                Some(&Vec3::new(
                    vert.position.x,
                    vert.position.y,
                    vert.position.z,
                )),
                None,
                Some(&Vec2::new(vert.tex_coord.x, vert.tex_coord.y)),
                None,
            );
        }

        CvdModelEntity {
            texture_path,
            vertices,
            indices,
            id,
        }
    }
}

impl EntityExtension for CvdModelEntity {
    fn on_loading(self: &mut CoreEntity<Self>) {
        self.add_component(RenderObject::new_with_data(
            self.vertices.clone(),
            self.indices.clone(),
            Box::new(SimpleMaterial::new(&self.texture_path)),
        ));
    }
}
