use std::{sync::{Arc, LazyLock}, time::Duration};

use chaos_vk::graphics::{camera::Camera, vk::MemAllocators};
use tokio::sync::{mpsc::{channel, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender}, Mutex};

use crate::{chunkmesh::ChunkMesh, world::{Chunk, ChunkKey, CHUNK_SIZE, DRAW_DISTANCE}};

#[derive(Clone)]
pub enum ChunkBuilderCommands {
    NewChunk(ChunkKey, Vec<ChunkKey>),
    Info(Camera, Vec<ChunkKey>),
}

#[derive(Clone)]
pub struct ChunkBuilderChannelData {
    pub chunk: (ChunkKey, Chunk, Option<ChunkMesh>),
    
}

pub struct  ChunkBuilder {
    pub command_sender: Sender<ChunkBuilderCommands>,
    command_recv: Arc<Mutex<Receiver<ChunkBuilderCommands>>>,

    data_sender: Sender<ChunkBuilderChannelData>,
    pub data_recv: Receiver<ChunkBuilderChannelData>,
}

impl ChunkBuilder {
    pub fn new() -> Self {
        let (tx, rx) = channel(2);
        let (tx2, rx2) = channel(2);
        
        let rx = Arc::new(Mutex::new(rx));

        Self {
            command_sender: tx,
            command_recv: rx,

            data_sender: tx2,
            data_recv: rx2,
        }
    }

    pub fn begin_loop(&mut self, allocators: Arc<chaos_vk::graphics::vk::MemAllocators>) {
        (-3..3).for_each(|id| {
            let rx = self.command_recv.clone();
            let tx = self.data_sender.clone();
            let allocators = allocators.clone();

            tokio::task::spawn(async move {
                dbg!(id);
                loop {
                    let mut recv = rx.lock().await;
                    if let Some(command) = recv.recv().await {
                        match command {
                            ChunkBuilderCommands::NewChunk(_, vec) => todo!(),
                            ChunkBuilderCommands::Info(camera, existing_chunks) => {
                                on_info(camera, existing_chunks, allocators.clone(), tx.clone(), id)
                                    .await;
                            },
                        }
                    }

                    // tokio::time::sleep(Duration::from_millis(16)).await;
                }
            });
        });
    }
    

}

pub async fn on_info(
    camera: Camera, 
    existing_chunks: Vec<ChunkKey>, 
    allocators: Arc<MemAllocators>,

    tx: Sender<ChunkBuilderChannelData>,
    id: isize,
) {
    let (i,j,k) = Chunk::get_ijk_chunkspace(camera.pos);

    let dx = id;
    for dz in -3..3 {
        'y: for dy in -2..2 {
            let k = (i + dx, j + dy, k + dz);
            
            for key in &existing_chunks {
                if k == *key {
                    continue 'y;
                }
            }
            let mut chunk = Chunk::new(k);
            
            chunk.lod = get_lod_by_distance(&camera, k);
    
            let mesh = chunk.get_mesh(allocators.clone());
    
            tx.send(ChunkBuilderChannelData {
                chunk: (k, chunk, mesh),
            }).await.unwrap();
        }
    }
}

pub fn get_lod_by_distance(camera: &Camera, k: ChunkKey) -> usize {
    match camera.pos.distance(Chunk::get_worldpos(&k)) {
        d if d < DRAW_DISTANCE * 0.25 => {
            0
        },
        d if d < DRAW_DISTANCE * 0.5 => {
            2
        }
        d if d < DRAW_DISTANCE * 0.75 => {
            4
        }
        _ => {
            8
        }
    }
}