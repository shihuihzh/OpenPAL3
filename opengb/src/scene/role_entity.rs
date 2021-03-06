use crate::{asset_manager::AssetManager, loaders::mv3_loader::*};
use radiance::math::{Vec2, Vec3};
use radiance::rendering::{Material, RenderObject, SimpleMaterial, VertexBuffer, VertexComponents};
use radiance::scene::{CoreEntity, EntityExtension};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

#[derive(PartialEq, Copy, Clone)]
pub enum RoleAnimationRepeatMode {
    NoRepeat,
    Repeat,
}

#[derive(PartialEq, Copy, Clone)]
pub enum RoleState {
    PlayingAnimation,
    Idle,
}

pub struct RoleEntity {
    name: String,
    asset_mgr: Rc<AssetManager>,
    animations: HashMap<String, RoleAnimation>,
    active_anim_name: String,
    idle_anim_name: String,
    anim_repeat_mode: RoleAnimationRepeatMode,
    is_active: bool,
    state: RoleState,
    auto_play_idle: bool,
}

impl RoleEntity {
    pub fn new(asset_mgr: &Rc<AssetManager>, role_name: &str, idle_anim: &str) -> RoleEntity {
        let mut animations = HashMap::new();
        if !idle_anim.trim().is_empty() {
            animations.insert(
                idle_anim.to_string(),
                asset_mgr.load_role_anim(role_name, idle_anim),
            );
        }

        RoleEntity {
            name: role_name.to_string(),
            asset_mgr: asset_mgr.clone(),
            animations,
            active_anim_name: idle_anim.to_string(),
            idle_anim_name: idle_anim.to_string(),
            anim_repeat_mode: RoleAnimationRepeatMode::Repeat,
            is_active: false,
            state: RoleState::Idle,
            auto_play_idle: true,
        }
    }

    pub fn set_active(self: &mut CoreEntity<Self>, active: bool) {
        self.is_active = active;
        if active {
            let anim_name = self.active_anim_name.clone();
            self.play_anim(&anim_name, self.anim_repeat_mode);
        } else {
            self.remove_component::<RenderObject>();
        }
    }

    pub fn play_anim(
        self: &mut CoreEntity<Self>,
        anim_name: &str,
        repeat_mode: RoleAnimationRepeatMode,
    ) {
        self.state = match anim_name.to_lowercase().as_ref() {
            "c01" => RoleState::Idle,
            _ => RoleState::PlayingAnimation,
        };

        if self.animations.get(anim_name).is_none() {
            let anim = self.asset_mgr.load_role_anim(&self.name, anim_name);
            self.animations.insert(anim_name.to_string(), anim);
        }

        self.active_anim_name = anim_name.to_string();
        self.anim_repeat_mode = repeat_mode;
        self.active_anim_mut().reset(repeat_mode);

        self.remove_component::<RenderObject>();
        self.add_component(self.active_anim().render_object());
    }

    pub fn set_auto_play_idle(&mut self, auto_play_idle: bool) {
        self.auto_play_idle = auto_play_idle;
    }

    pub fn state(&self) -> RoleState {
        self.state
    }

    fn active_anim(&self) -> &RoleAnimation {
        self.animations.get(&self.active_anim_name).unwrap()
    }

    fn active_anim_mut(&mut self) -> &mut RoleAnimation {
        self.animations.get_mut(&self.active_anim_name).unwrap()
    }
}

impl EntityExtension for RoleEntity {
    fn on_loading(self: &mut CoreEntity<Self>) {
        if !self.idle_anim_name.trim().is_empty() && self.is_active {
            let name = self.idle_anim_name.clone();
            self.play_anim(&name, RoleAnimationRepeatMode::Repeat);
        }
    }

    fn on_updating(self: &mut CoreEntity<Self>, delta_sec: f32) {
        if self.is_active {
            self.active_anim_mut().update(delta_sec);
            self.get_component_mut::<RenderObject>()
                .unwrap()
                .make_dirty();

            if self.active_anim().anim_finished() {
                self.state = RoleState::Idle;

                if self.state == RoleState::Idle && self.auto_play_idle {
                    self.play_anim("c01", RoleAnimationRepeatMode::Repeat);
                }
            }
        }
    }
}

pub struct RoleAnimation {
    frames: Vec<VertexBuffer>,
    anim_timestamps: Vec<u32>,
    last_anim_time: u32,
    repeat_mode: RoleAnimationRepeatMode,
    anim_finished: bool,
    vertices: Arc<RwLock<VertexBuffer>>,
    indices: Arc<RwLock<Vec<u32>>>,
    material: Arc<RwLock<Box<dyn Material>>>,
}

impl RoleAnimation {
    pub fn new(mv3file: &Mv3File, anim_repeat_mode: RoleAnimationRepeatMode) -> Self {
        let model: &Mv3Model = &mv3file.models[0];
        let mesh: &Mv3Mesh = &model.meshes[0];

        let mut texture_path = mv3file.path.clone();
        texture_path.pop();
        texture_path.push(std::str::from_utf8(&mv3file.textures[0].names[0]).unwrap());

        let hash =
            |index, texcoord_index| index as u32 * model.texcoord_count + texcoord_index as u32;

        let mut indices: Vec<u32> = Vec::<u32>::with_capacity(model.vertex_per_frame as usize);
        let mut vertices_data: Vec<Vec<(Vec3, Vec2)>> = vec![vec![]; model.frame_count as usize];
        let mut index_map = HashMap::new();

        for t in &mesh.triangles {
            for (&i, &j) in t.indices.iter().zip(&t.texcoord_indices) {
                let h = hash(i, j);
                let index = match index_map.get(&h) {
                    None => {
                        let index = index_map.len();
                        for k in 0..model.frame_count as usize {
                            let frame = &model.frames[k];
                            vertices_data[k].push((
                                Vec3::new(
                                    frame.vertices[i as usize].x as f32 * 0.01562,
                                    frame.vertices[i as usize].y as f32 * 0.01562,
                                    frame.vertices[i as usize].z as f32 * 0.01562,
                                ),
                                Vec2::new(
                                    model.texcoords[j as usize].u,
                                    -model.texcoords[j as usize].v,
                                ),
                            ));
                        }
                        index_map.insert(h, index as u32);
                        index as u32
                    }
                    Some(index) => *index,
                };

                indices.push(index);
            }
        }

        let mut frames: Vec<VertexBuffer> =
            Vec::<VertexBuffer>::with_capacity(model.frame_count as usize);
        for i in 0..model.frame_count as usize {
            frames.push(VertexBuffer::new(
                VertexComponents::POSITION | VertexComponents::TEXCOORD,
                index_map.len(),
            ));

            let vertex_data = &vertices_data[i];
            let vert = frames.get_mut(i).unwrap();
            for j in 0..vertex_data.len() {
                vert.set_component(j, VertexComponents::POSITION, |p: &mut Vec3| {
                    *p = vertex_data[j].0;
                });
                vert.set_component(j, VertexComponents::TEXCOORD, |t: &mut Vec2| {
                    *t = vertex_data[j].1;
                });
            }
        }

        let anim_timestamps = model.frames.iter().map(|f| f.timestamp).collect();
        let vertices = Arc::new(RwLock::new(frames[0].clone()));

        RoleAnimation {
            frames,
            anim_timestamps,
            last_anim_time: 0,
            repeat_mode: anim_repeat_mode,
            anim_finished: false,
            vertices,
            indices: Arc::new(RwLock::new(indices)),
            material: Arc::new(RwLock::new(Box::new(SimpleMaterial::new(&texture_path)))),
        }
    }

    pub fn reset(&mut self, repeat_mode: RoleAnimationRepeatMode) {
        self.anim_finished = false;
        self.last_anim_time = 0;
        self.repeat_mode = repeat_mode;
        let mut vertices = self.vertices.write().unwrap();
        *vertices = self.frames[0].clone()
    }

    pub fn update(&mut self, delta_sec: f32) {
        let mut anim_time = (delta_sec * 4580.) as u32 + self.last_anim_time;
        let total_anim_length = *self.anim_timestamps.last().unwrap();

        if anim_time >= total_anim_length && self.repeat_mode == RoleAnimationRepeatMode::NoRepeat {
            self.anim_finished = true;
            return;
        }

        anim_time %= total_anim_length;
        let frame_index = self
            .anim_timestamps
            .iter()
            .position(|&t| t > anim_time)
            .unwrap_or(0)
            - 1;
        let next_frame_index = (frame_index + 1) % self.anim_timestamps.len();
        let percentile = (anim_time - self.anim_timestamps[frame_index]) as f32
            / (self.anim_timestamps[next_frame_index] - self.anim_timestamps[frame_index]) as f32;

        let vertex_buffer = self.frames.get(frame_index).unwrap();
        let next_vertex_buffer = self.frames.get(next_frame_index).unwrap();

        let mut vertices = self.vertices.write().unwrap();
        for i in 0..self.frames.get(frame_index).unwrap().count() {
            let position = vertex_buffer.position(i).unwrap();
            let next_position = next_vertex_buffer.position(i).unwrap();
            let tex_coord = vertex_buffer.tex_coord(i).unwrap();

            vertices.set_component(i, VertexComponents::POSITION, |p: &mut Vec3| {
                p.x = position.x * (1. - percentile) + next_position.x * percentile;
                p.y = position.y * (1. - percentile) + next_position.y * percentile;
                p.z = position.z * (1. - percentile) + next_position.z * percentile;
            });
            vertices.set_component(i, VertexComponents::TEXCOORD, |t: &mut Vec2| {
                t.x = tex_coord.x;
                t.y = tex_coord.y;
            });
        }

        self.last_anim_time = anim_time;
    }

    pub fn anim_finished(&self) -> bool {
        self.anim_finished
    }

    pub fn render_object(&self) -> RenderObject {
        RenderObject::new_host_dynamic_with_data(&self.vertices, &self.indices, &self.material)
    }
}
