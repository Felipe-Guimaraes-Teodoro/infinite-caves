use std::{collections::HashMap, sync::{Arc, Mutex}, thread::sleep_ms};

use bevy_ecs::{component::Component, system::{Commands, Resource}, world::{Mut, World}};
use chaos_vk::{graphics::{buffer::{VkBuffer, VkIterBuffer}, camera::Camera, command::{CommandBufferType, VkBuilder}, mesh::mesh::Mesh, presenter::Presenter, utils::{descriptor_set, VkSecRenderpass}, vertex::PosVertex, vk::Vk}, imgui_renderer::ImGui};
use glam::{Mat4, Vec3};
use threadpool::ThreadPool;
use vulkano::{buffer::{Buffer, BufferCreateInfo, BufferUsage}, command_buffer::{CommandBufferInheritanceInfo, CommandBufferInheritanceRenderPassInfo, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents}, descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet}, memory::allocator::{AllocationCreateInfo, MemoryTypeFilter}, pipeline::{GraphicsPipeline, Pipeline}, render_pass::{Framebuffer, RenderPass, Subpass}};

use crate::{chunkmesh::ChunkMesh, math::SecondOrderDynamics, mesh_spawner::MeshComponent, shaders::vs, world::ChunkWorld};

#[derive(Resource)]
pub struct Renderer {
    pub camera: Camera,
    pub cam_sod: SecondOrderDynamics<Vec3>,

    pub meshes: Vec<Mesh>,
    pub pool: ThreadPool,
    pub keymap: [bool; 7],
}

impl Renderer {
    pub fn new() -> Self {
        let mut camera = Camera::new();
        camera.speed = 8.0;
        camera.proj = Mat4::perspective_rh(80.0f32.to_radians(), 12.0/9.0, 0.1, 1000.0);
        Self {
            camera,
            cam_sod: SecondOrderDynamics::new(2.75, 0.75, 0.0, camera.pos),
            meshes: vec![],
            pool: ThreadPool::new(num_cpus::get()),
            keymap: [false; 7],
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.camera.dt = dt;
        set_goal_according_to_input(&mut self.camera, self.keymap);
        let y = self.cam_sod.update(self.camera.dt, self.camera.goal);
        self.camera.right = Vec3::Y.cross(-self.camera.front).normalize();
        self.camera.up = self.camera.front.cross(self.camera.right).normalize();
        self.camera.update(-y);
    }
}

pub fn get_keymap(
    event: &winit::event::WindowEvent,
    keymap: &mut [bool; 7],
) -> [bool;7] {
    match event {
        winit::event::WindowEvent::KeyboardInput { input, .. } => {
            let action = match input.state {
                winit::event::ElementState::Pressed => true,
                winit::event::ElementState::Released => false,
            };

            match input.virtual_keycode {
                Some(winit::event::VirtualKeyCode::W) => {
                    keymap[0] = action;
                },
                Some(winit::event::VirtualKeyCode::A) => {
                    keymap[1] = action;
                },
                Some(winit::event::VirtualKeyCode::S) => {
                    keymap[2] = action;
                },
                Some(winit::event::VirtualKeyCode::D) => {
                    keymap[3] = action;
                },
                Some(winit::event::VirtualKeyCode::Space) => {
                    keymap[4] = action;
                },
                Some(winit::event::VirtualKeyCode::LControl) => {
                    keymap[5] = action;
                },
                Some(winit::event::VirtualKeyCode::LShift) => {
                    keymap[6] = action;
                },
                _ => ()
            }

            *keymap
        }

        _ => *keymap,
    }

}

pub fn set_goal_according_to_input(cam: &mut Camera, keymap: [bool; 7]) {
    let mut speed = cam.speed;
    if keymap[6] {
        speed = cam.speed * 20.0;
    }

    if keymap[0] {
        cam.goal -= speed * cam.dt * cam.front;
    }
    if keymap[1] {
        cam.goal -= speed * cam.dt * Vec3::cross(cam.front, cam.up);
    }
    if keymap[2] {
        cam.goal += speed * cam.dt * cam.front;
    }
    if keymap[3] {
        cam.goal += speed * cam.dt * Vec3::cross(cam.front, cam.up);
    }
    if keymap[4] {
        cam.goal -= speed * cam.dt * cam.up;
    }
    if keymap[5] {
        cam.goal += speed * cam.dt * cam.up;
    }
}

pub fn get_cmd_bufs(
    vk: Arc<Vk>, 
    renderer: &mut Renderer,
    imgui_renderer: &mut ImGui,
    presenter: &Presenter,
    pipeline: Arc<GraphicsPipeline>,
    world: &mut World,
    rp: Arc<RenderPass>,
) -> Vec<CommandBufferType> {
    let mut cmd_bufs = vec![];
    let mut meshes = world.query::<&MeshComponent>();

    let ubo = VkBuffer::uniform(vk.allocators.clone(), vs::Camera {
        view: renderer.camera.get_view(),
        proj: renderer.camera.get_proj(),
    });

    let camera_desc_set = descriptor_set(
        vk.clone(), 
        0, 
        pipeline.clone(), 
        [WriteDescriptorSet::buffer(0, ubo.content.clone())]
    ).0;

    let imgui_renderpasses = imgui_renderer.get_renderpasses(
        presenter.images.clone(),
        vk.clone()
    );

    let mut i = 0;

    for framebuffer in &presenter.framebuffers {
        let mut builder = VkBuilder::new_multiple(vk.clone());

        builder.0
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.1, 0.2, 0.3, 1.0].into()), Some(1.0.into())],
                    ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )
            .unwrap();
    
        builder.0
            .bind_pipeline_graphics(pipeline.clone())
            .unwrap()            
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics, 
                pipeline.layout().clone(), 
                0, 
                camera_desc_set.clone(),
            )
            .unwrap();

        for mesh in &renderer.meshes {
            mesh.build_commands(vk.clone(), &mut builder.0, pipeline.clone());
        }

        for mesh in meshes.iter(&world) {
            let mesh = Mesh::new(vk.clone(), &mesh.vertices, &mesh.indices);
            mesh.build_commands(vk.clone(), &mut builder.0, pipeline.clone());
        }
    
        let chunkmeshes = &mut world.resource_mut::<ChunkWorld>().meshes;
        for mesh in chunkmeshes.values_mut().filter_map(Option::as_mut) {
            if mesh.visible {
                mesh.bind_buffers(&mut builder.0);
                builder.0
                    .draw_indexed_indirect(mesh.get_indb(&vk))
                    .unwrap();
            }
        }
        
    
        builder.0.end_render_pass(Default::default()).unwrap();
    
        /* ----- RENDER IMGUI ------ */
    
        let render_pass = &imgui_renderpasses[i];
        builder.0.begin_render_pass(
            RenderPassBeginInfo {
                clear_values: vec![None],
                render_pass: render_pass.rp.clone(),
                ..RenderPassBeginInfo::framebuffer(render_pass.framebuffer.clone())
            },
            SubpassBeginInfo {
                contents: SubpassContents::SecondaryCommandBuffers,
                ..Default::default()
            },
        ).expect(&format!("failed to start imgui render pass on framebuffer {:?}", framebuffer));
    
        builder.0.execute_commands(render_pass.cmd_buf.clone()).unwrap();
        
        builder.0.end_render_pass(Default::default()).unwrap();
        
        
        cmd_bufs.push(
            builder.command_buffer()
        );
    
        i += 1;
    }

    // renderer.pool.join();

    cmd_bufs
}

pub fn commands(
    vk: &Arc<Vk>, 
    framebuffer: &Arc<Framebuffer>,
    pipeline: &Arc<GraphicsPipeline>,
    camera_desc_set: &Arc<PersistentDescriptorSet>,
    // chunkmeshes: &mut Arc<Mutex<HashMap<(isize, isize, isize), Option<ChunkMesh>>>>,
    imgui_renderpass: &VkSecRenderpass,

    cmd_bufs: &mut Vec<CommandBufferType>,
    i: &mut usize,
) {
    let mut builder = VkBuilder::new_multiple(vk.clone());

    builder.0
        .begin_render_pass(
            RenderPassBeginInfo {
                clear_values: vec![Some([0.1, 0.2, 0.3, 1.0].into()), Some(1.0.into())],
                ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
            },
            SubpassBeginInfo {
                contents: SubpassContents::Inline,
                ..Default::default()
            },
        )
        .unwrap();

    //builder.0
    //    .bind_pipeline_graphics(pipeline.clone())
    //    .unwrap()            
    //    .bind_descriptor_sets(
    //        vulkano::pipeline::PipelineBindPoint::Graphics, 
    //        pipeline.layout().clone(), 
    //        0, 
    //        camera_desc_set.clone(),
    //    )
    //    .unwrap();
        // .bind_descriptor_sets(
        //     vulkano::pipeline::PipelineBindPoint::Graphics, 
        //     pipeline.layout().clone(), 
        //     1, 
        //     Mesh::get_ubo(&Mesh::, vk),
        // )
        // .unwrap();

    // let mut chunkmeshes_lock = chunkmeshes.lock().unwrap();
    // for mesh in chunkmeshes_lock.values_mut() {
    //     if let Some(mesh) = mesh {
    //         mesh.bind_buffers(&mut builder.0);
    //         builder.0.draw_indexed_indirect(
    //             mesh.get_indb(vk.clone())
    //         ).unwrap();
    //     }
    // }
    // drop(chunkmeshes_lock);

    builder.0.end_render_pass(Default::default()).unwrap();

    /* ----- RENDER IMGUI ------ */

    let render_pass = imgui_renderpass;
    builder.0.begin_render_pass(
        RenderPassBeginInfo {
            clear_values: vec![None],
            render_pass: render_pass.rp.clone(),
            ..RenderPassBeginInfo::framebuffer(render_pass.framebuffer.clone())
        },
        SubpassBeginInfo {
            contents: SubpassContents::SecondaryCommandBuffers,
            ..Default::default()
        },
    ).expect(&format!("failed to start imgui render pass on framebuffer {:?}", framebuffer));

    builder.0.execute_commands(render_pass.cmd_buf.clone()).unwrap();
    
    builder.0.end_render_pass(Default::default()).unwrap();
    
    
    cmd_bufs.push(
        builder.command_buffer()
    );

    *i += 1;
}

/* someone forgot to implement clone for VkSecRenderpass. i wonder who could it possibly be */
trait CustomClone {
    fn custom_clone(&self) -> Self;
}

impl CustomClone for VkSecRenderpass {
    fn custom_clone(&self) -> Self {
        Self { cmd_buf: self.cmd_buf.clone(), framebuffer: self.framebuffer.clone(), rp: self.rp.clone(), clear_values: self.clear_values.clone() }
    }
}