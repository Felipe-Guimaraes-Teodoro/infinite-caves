use std::{cell::RefCell, default, fs::File, io::Read, rc::Rc, sync::{Arc, Mutex}};

use bevy_app::{App, Startup, Update};
use bevy_ecs::{bundle::Bundle, schedule::SystemSchedule, world::World};
use chaos_vk::{graphics::{mesh::mesh::Mesh, presenter::Presenter, utils::{instancing_pipeline, render_pass_with_depth}, vertex::{InstanceData, PosVertex}, vk::Vk}, imgui_renderer::ImGui};
use geometry::sphere;
use glam::{vec3, Mat4, Vec3};
use lua::LuaIntegration;
use math::rand_betw;
use mesh_spawner::{SpawnCommand, SpawnCommandBuffer};
use renderer::{get_cmd_bufs, get_keymap, Renderer};
use rlua::{chunk, Lua, RluaCompat};
use shaders::{fs, vs};
use vk_mod::CustomNew;
use vulkano::{device::{Device, Features}, pipeline::{graphics::{color_blend::{ColorBlendAttachmentState, ColorBlendState}, depth_stencil::DepthStencilState, input_assembly::{InputAssemblyState, PrimitiveTopology}, multisample::MultisampleState, rasterization::RasterizationState, vertex_input::{Vertex, VertexDefinition}, viewport::{Viewport, ViewportState}, GraphicsPipelineCreateInfo}, layout::PipelineDescriptorSetLayoutCreateInfo, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo}, render_pass::{RenderPass, Subpass}, shader::ShaderModule};
use winit::{dpi::PhysicalSize, event::{DeviceEvent, ElementState, Event, MouseScrollDelta, VirtualKeyCode, WindowEvent}, event_loop::{ControlFlow, EventLoop}};
use world::{insert_chunkworld_resource, ChunkWorld};

mod shaders;
mod renderer;
mod geometry;
mod math;
mod mesh_spawner;
pub mod lua;
pub mod world;
pub mod chunkmesh;
pub mod vk_mod;
pub mod chunk_builder;
pub mod culler;

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new();
    let vk = Vk::custom_new(&event_loop);
    vk.window.set_inner_size(PhysicalSize::new(1200, 900));
    let mut app = App::new();

    app
        .add_systems(Startup, mesh_spawner::startup)
        .add_systems(Update, mesh_spawner::update);

    insert_chunkworld_resource(app.world_mut().commands(), vk.allocators.clone());

    let mut renderer = Renderer::new();
    let sphere = sphere(5, 0.5, Vec3::ZERO);
    renderer.meshes.push(Mesh::new(vk.clone(), &sphere.vertices, &sphere.indices));
    let vs = vs::load(vk.device.clone()).unwrap();
    let fs = fs::load(vk.device.clone()).unwrap();
    let mut presenter = Presenter::new(vk.clone());
    let rp = render_pass_with_depth(vk.clone(), Some(presenter.swapchain.clone()));
    let mut pipeline = get_pipeline(vk.clone(), vs.clone(), fs.clone(), rp.clone(), Viewport {
        offset: [0.0, 0.0],
        extent: [1200.0, 900.0],
        depth_range: 0.0..=1.0,
    });

    let mut imgui = ImGui::new(vk.clone(), &presenter);
    presenter.window_resized = true;

    let mut dt = 0.0;
    let (mut cursor_x, mut cursor_y) = (0.0, 0.0);

    let mut buf = String::new();
    let mut lua_integration = LuaIntegration::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        renderer.keymap = get_keymap(&event, &mut renderer.keymap);
                        lua_integration.handle_input(input);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        imgui.on_mouse_move(position.x as f32, position.y as f32);
                    }
                    WindowEvent::MouseInput { button, state, .. } => {
                        imgui.on_mouse_click(button, state);
                    }
                    WindowEvent::MouseWheel { delta: MouseScrollDelta::PixelDelta(pos), .. } => {
                        imgui.on_mouse_scroll(pos.x as f32, pos.y as f32);
                    }

                    WindowEvent::Resized(size) => {
                        renderer.camera.proj = Mat4::perspective_rh(80.0f32.to_radians(), size.width as f32/size.height as f32, 0.1, 1000.0);
                        pipeline = get_pipeline(vk.clone(), vs.clone(), fs.clone(), rp.clone(), Viewport {
                            offset: [0.0, 0.0],
                            extent: size.into(),
                            depth_range: 0.0..=1.0,
                        });
                    }
                    _ => ()
                }
            }

            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                cursor_x += delta.0 as f32;
                cursor_y += delta.1 as f32;
                
                renderer.camera.mouse_callback(cursor_x, cursor_y);
            }
            
            Event::MainEventsCleared => {
                let now = std::time::Instant::now();

                app.update();

                lua_integration.update(&mut app);

                let mut world = app.world_mut();
                let mut chunkworld = world.resource_mut::<ChunkWorld>();
                chunkworld.update(vk.allocators.clone(), &renderer.camera);

                let frame = imgui.frame(&vk.window);
                frame.text(format!("hello, world! dt: {:?}", dt*1000.0));
                frame.input_text("code", &mut buf)
                    .build();

                presenter.recreate(vk.clone(), rp.clone());

                presenter.cmd_bufs = get_cmd_bufs(
                    vk.clone(), 
                    &mut renderer, 
                    &mut imgui, 
                    &presenter, 
                    pipeline.clone(),
                    &mut world,
                    rp.clone()
                );
                renderer.update(dt);
                presenter.present(vk.clone());


                dt = now.elapsed().as_secs_f32();
                // dbg!(now.elapsed());
            }

            _ => ()
        }
    });
}

pub fn get_pipeline(
    vk: Arc<Vk>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,

    render_pass: Arc<RenderPass>,
    viewport: Viewport,
) -> Arc<GraphicsPipeline> {
    let vs = vs.entry_point("main").unwrap();
    let fs = fs.entry_point("main").unwrap();

    let vertex_input_state = [PosVertex::per_vertex(), InstanceData::per_instance()]
        .definition(&vs.info().input_interface)
        .unwrap();

    let stages = [
        PipelineShaderStageCreateInfo::new(vs),
        PipelineShaderStageCreateInfo::new(fs),
    ];

    let layout = PipelineLayout::new(
        vk.device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(vk.device.clone())
            .unwrap(),
    )
    .unwrap();

    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

    GraphicsPipeline::new(
        vk.device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState {
                primitive_restart_enable: false,
                ..Default::default()
            }),
            viewport_state: Some(ViewportState {
                viewports: [viewport].into_iter().collect(),
                ..Default::default()
            }),
            rasterization_state: Some(RasterizationState {
                cull_mode: vulkano::pipeline::graphics::rasterization::CullMode::Back,
                front_face: vulkano::pipeline::graphics::rasterization::FrontFace::Clockwise,
                ..Default::default()
            }),
            multisample_state: Some(MultisampleState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState::default(),
            )),
            subpass: Some(subpass.into()),
            depth_stencil_state: Some(DepthStencilState::simple_depth_test()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    )
    .unwrap()
}