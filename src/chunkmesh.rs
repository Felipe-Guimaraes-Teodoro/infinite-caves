use std::{collections::HashMap, sync::Arc};

use chaos_vk::graphics::{buffer::{VkBuffer, VkIterBuffer}, command::{BuilderType, SecBuilderType, SecondaryCmdBufType, VkBuilder}, utils::descriptor_set, vertex::{InstanceData, PosVertex}, vk::{MemAllocators, Vk}};
use glam::{Mat4, Quat, Vec3};
use vulkano::{buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer}, command_buffer::{CommandBufferInheritanceInfo, CommandBufferInheritanceRenderPassInfo, DispatchIndirectCommand, DrawIndexedIndirectCommand, DrawIndirectCommand, SecondaryAutoCommandBuffer, SecondaryCommandBufferAbstract}, descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet}, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}, pipeline::{GraphicsPipeline, Pipeline}, render_pass::{Framebuffer, Subpass}};

#[derive(BufferContents, Clone, Copy)]
#[repr(C)]
pub struct Model {
    model: [[f32;4];4]
}

#[derive(Clone)]
pub struct ChunkMesh {
    pub vertices: Vec<PosVertex>,
    pub indices: Vec<u32>,
    pub instances: Vec<InstanceData>,

    pub vbo: Subbuffer<[PosVertex]>,
    pub ibo: Subbuffer<[InstanceData]>,
    pub ebo: Subbuffer<[u32]>,
    pub ubo: Option<VkBuffer<Model>>,
    pub dc: Option<Arc<PersistentDescriptorSet>>,

    pub cbo: Vec<Option<SecondaryCmdBufType>>,

    pub indb: Option<Subbuffer<[DrawIndexedIndirectCommand]>>,
    
    pub visible: bool,
}

/* TODO: on CHAOS_VK add static index and vertex buffers */
impl ChunkMesh {
    pub fn new(allocators: Arc<MemAllocators>, vertices: &Vec<PosVertex>, indices: &Vec<u32>) -> Self {
        let instances = vec![InstanceData {ofs: [0.0, 0.0, 0.0]}];

        let vertex_buf = Buffer::from_iter(
            allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vertices.to_vec(),
        )
        .expect("failed to create buffer");

        let index_buf = Buffer::from_iter(
            allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            indices.to_vec(),
        )
        .expect("failed to create buffer");

        let instance_buf = Buffer::from_iter(
            allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            instances.to_vec(),
        )
        .expect("failed to create buffer");

        Self {
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
            instances: instances.to_vec(),

            vbo: vertex_buf,
            ebo: index_buf,
            ibo: instance_buf,
            ubo: None,
            dc: None,

            cbo: vec![None; 3],

            indb: None,
            visible: true,
        }
    }

    pub fn get_indb(&mut self, vk: &Arc<Vk>) -> Subbuffer<[DrawIndexedIndirectCommand]> {
        if self.indb.is_some() {
            self.indb.clone().unwrap()
        } else {
            let buffer  = Buffer::from_iter(
                vk.allocators.memory.clone(), 
                BufferCreateInfo {
                    usage: BufferUsage::INDIRECT_BUFFER,
                    ..Default::default()
                }, 
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                vec![self.get_indirect_command()] 
            ).unwrap();
            self.indb = Some(
                buffer.clone()
            );

            buffer
        }
    }

    pub fn get_dc(&mut self, vk: Arc<Vk>, pipeline: Arc<GraphicsPipeline>, ubo: VkBuffer<Model>) -> Arc<PersistentDescriptorSet> {
        if self.dc.is_none() {
            self.dc = Some(descriptor_set(
                vk.clone(), 
                1,
                pipeline.clone(), 
                [WriteDescriptorSet::buffer(0, ubo.content.clone())]
            ).0);

            return self.dc.clone().unwrap();
        } else {
            return self.dc.clone().unwrap();
        }
    }

    /// Warning: this function assumes a graphics pipeline has already been bounded
    /// 
    /// On the shaders, it assumes:
    /// ```glsl
    /// layout(set = 1, binding = 0) uniform Model { 
    ///     mat4 model;
    /// };
    /// ```

    pub fn get_indirect_command(&self) -> DrawIndexedIndirectCommand {
        DrawIndexedIndirectCommand {
            index_count: self.indices.len() as u32,
            instance_count: 1,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }

    pub fn bind_buffers(&self, builder: &mut BuilderType) {
        builder
            .bind_vertex_buffers(0, 
                (self.vbo.clone(), self.ibo.clone())
            )
            .unwrap()
            .bind_index_buffer(self.ebo.clone())
            .unwrap();
    }
}
