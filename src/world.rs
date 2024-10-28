use core::f32;
use std::{collections::{HashMap, VecDeque}, sync::Arc};

use bevy_ecs::system::{Commands, Resource};
use chaos_vk::graphics::{buffer::VkIterBuffer, camera::Camera, mesh::mesh::Mesh, vertex::InstanceData, vk::{MemAllocators, Vk}};
use glam::{quat, vec3, Vec3};
use noise::{NoiseFn, Perlin, PerlinSurflet};
use tokio::{sync::{mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender}, Mutex}, task::JoinHandle};
use vulkano::{buffer::{Buffer, BufferCreateInfo, BufferUsage}, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}};

use crate::{chunk_builder::{get_lod_by_distance, ChunkBuilder, ChunkBuilderChannelData, ChunkBuilderCommands}, chunkmesh::ChunkMesh, culler::ChunkCuller, geometry::voxel_gen, math::{rand_betw, rand_vec3}};

pub const CHUNK_SIZE: usize = 64;
pub const DRAW_DISTANCE: f32 = 400.0;

pub type ChunkKey = (isize, isize, isize);

#[derive(Clone, Copy)]
pub struct Voxel {
    pub id: usize,
}

#[derive(Clone)]
pub struct Chunk {
    key: ChunkKey,
    voxels: Vec<Voxel>,
    pub lod: usize,
    pub outdated: bool,
}

impl Chunk {
    pub fn new(key: ChunkKey) -> Self {
        let pos = Chunk::get_worldpos(&key);
        let mut voxels = vec![Voxel { id: 0 }; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        
        let perlin = PerlinSurflet::new(0);
        let scale = 0.01;
    
        for i in 0..CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE {
            let (x, y, z) = voxel_gen::get_pos(i);
            let index = voxel_gen::get_index(x as usize, CHUNK_SIZE - y as usize, z as usize);

            // position of the voxel
            let pos = vec3(pos.x + x as f32, pos.y + y as f32, pos.z + z as f32) * scale;
            let res = perlin.get([pos.x as f64, pos.y as f64, pos.z as f64]);
            // let res = perlin.get([x as f64 * scale, y as f64 * scale, z as f64 * scale]) * CHUNK_SIZE as f64;
            /*
            if y > res as isize && key.1 == 0 {
                voxels[index].id = 1;
            } 
            */

            if res < 0.0 {
                voxels[i].id = 1;
            }
        }
    
        Self {
            key,
            voxels,
            lod: 0,
            outdated: false,
        }
    }
    
    

    pub fn get_mesh(&mut self, allocators: Arc<MemAllocators>) -> Option<ChunkMesh> {
        let voxels = self.voxels.clone();
        let key = self.key;

        let (vertices, indices) = voxel_gen::gen_mesh_data_culled(&voxels, self.lod + 1);

        if indices.len() > 0 {
            let mut mesh = ChunkMesh::new(allocators.clone(), &vertices, &indices);
    
            let pos = Self::get_worldpos(&key);
            
            mesh.ibo = Buffer::from_iter(
                allocators.memory.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                vec![InstanceData {
                    ofs: [pos.x, pos.y, pos.z],
                }],
            )
            .expect("failed to create buffer");
    
            Some(mesh)
        } else {
            None
        }
    }

    /* this might be broken? */
    pub fn get_ijk_chunkspace(pos: Vec3) -> ChunkKey {
        let i = (pos.x / CHUNK_SIZE as f32).floor() as isize;
        let j = (pos.y / CHUNK_SIZE as f32).floor() as isize;
        let k = (pos.z / CHUNK_SIZE as f32).floor() as isize;

        (i, j, k)
    }

    /* this too, might be broken */
    pub fn get_worldpos(key: &ChunkKey) -> Vec3 {
        let x = (key.0 * CHUNK_SIZE as isize) as f32;
        let y = (key.1 * CHUNK_SIZE as isize) as f32;
        let z = (key.2 * CHUNK_SIZE as isize) as f32;

        vec3(x, y, z)
    }

    pub fn update(&mut self, camera: &Camera) {
        if self.lod != get_lod_by_distance(camera, self.key) {
            self.outdated = true;
        }
    }
}

#[derive(Resource)]
pub struct ChunkWorld {
    chunks: HashMap<ChunkKey, Chunk>,

    pub meshes: HashMap<ChunkKey, Option<ChunkMesh>>,
    chunks_to_remove: Vec<ChunkKey>,
    meshes_to_remove: Vec<ChunkKey>,
    queue: VecDeque<ChunkKey>,

    chunk_builder_tx: Sender<ChunkBuilderCommands>,
    chunk_builder_rx: Receiver<ChunkBuilderChannelData>,
}

impl ChunkWorld {
    pub fn new(allocators: Arc<MemAllocators>) -> Self {
        let chunks = HashMap::new();

        let mut chunk_builder = ChunkBuilder::new();
        chunk_builder.begin_loop(allocators);

        Self {
            chunks: chunks.clone(),
            meshes: HashMap::new(),
            chunks_to_remove: vec![],
            meshes_to_remove: vec![],
            queue: VecDeque::new(),
            chunk_builder_tx: chunk_builder.command_sender,
            chunk_builder_rx: chunk_builder.data_recv,
        }
    }
    /* 
    pub fn update(&mut self, allocators: Arc<MemAllocators>, camera: &Camera) {
        let (i, j, k) = Chunk::get_ijk_chunkspace(camera.pos);
        
        let mut existing_chunks = vec![];
        for key in self.chunks.keys() {
            existing_chunks.push(*key);
        }

        self.chunk_builder_tx.try_send(
            ChunkBuilderCommands::Info(*camera, existing_chunks),
        ).ok();
    
        self.chunks_to_remove.retain(|k| {
            if let Some(chunk) = self.chunks.get_mut(k) {
                if chunk.lod < 4 {
                    chunk.lod = get_lod_by_distance(camera, *k);
                    true
                } else {
                    self.chunks.remove(k).is_some() && self.meshes.remove(k).is_some()
                }
            } else {
                false
            }
        });
    
        while let Ok(rx) = self.chunk_builder_rx.try_recv() {
            let (k, chunk, mesh) = rx.chunk;

            self.chunks.insert(k, chunk);
            self.meshes.insert(k, mesh);
        }

        /* clear out chunks that are too far away */
        for k in self.chunks.keys() {
            let dist = Chunk::get_worldpos(k).distance(camera.pos) / 100.0;
            if dist > DRAW_DISTANCE {
                self.chunks_to_remove.push(*k);
            }
        }

        if let Some(chunk) = self.chunks.get(&(i,j,k)) {
            if chunk.lod != 0 {
                self.chunks_to_remove.push((i,j,k));
            }
        }
    }
    */

    pub fn update(&mut self, allocators: Arc<MemAllocators>, camera: &Camera) {
        let mut existing_chunks = vec![];
        for key in self.chunks.keys() {
            existing_chunks.push(*key);
        }
        self.chunk_builder_tx.try_send(
            ChunkBuilderCommands::Info(*camera, existing_chunks),
        ).ok();

        while let Ok(rx) = self.chunk_builder_rx.try_recv() {
            let (k, chunk, mesh) = rx.chunk;

            self.chunks.insert(k, chunk);
            self.meshes.insert(k, mesh);
        }

        self.chunks_to_remove.retain(|k| {
            self.chunks.remove(k).is_some()
        });
        self.meshes_to_remove.retain(|k| {
            self.meshes.remove(k).is_some()
        });

        for (k, chunk) in &mut self.chunks {
            chunk.update(camera);
            if chunk.outdated {
                self.chunks_to_remove.push(*k);
            }
        }

        for k in self.meshes.keys() {
            // let lod = chunk.lod;

            if  /* lod >= 4 
            || */ camera.pos.distance(Chunk::get_worldpos(k)) > DRAW_DISTANCE 
            {
                self.meshes_to_remove.push(*k);
            }
        }

        for (k, mesh) in &mut self.meshes {
            if let Some(ref mut mesh) = mesh {
                mesh.visible = ChunkCuller::is_visible(*k, camera);
            }
        }
    }
}

pub fn insert_chunkworld_resource(mut commands: Commands, allocators: Arc<MemAllocators>) {
    let chunk_world = ChunkWorld::new(allocators);
    commands.insert_resource(chunk_world);
}