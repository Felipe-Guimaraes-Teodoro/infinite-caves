use std::{sync::{Arc, Mutex}, vec};

use bevy_ecs::{component::Component, entity::Entity, query::With, system::{Commands, Query, Res, ResMut, Resource}, world::World};
use chaos_vk::graphics::{mesh::mesh::Mesh, vertex::PosVertex};
use glam::Vec3;
use mlua::FromLua;
use rlua::{UserData, UserDataMethods};

use crate::geometry::sphere;

#[derive(Component)]
pub struct MeshComponent {
    pub vertices: Vec<PosVertex>,
    pub indices: Vec<u32>,
    pub position: Vec3,
}

#[derive(Clone, Copy, FromLua, Debug)]
pub enum SpawnCommand {
    Sphere(Vec3, f32), /* pos and radius */
    Clear,
}

impl SpawnCommand {
    pub fn encode(&self) -> u128 {
        match self {
            SpawnCommand::Sphere(pos, rad) => {
                let x = pos.x.to_bits() as u128;
                let y = pos.y.to_bits() as u128;
                let z = pos.z.to_bits() as u128;
                let r = rad.to_bits() as u128;

                (x << 96) | (y << 64) | (z << 32) | r
            }
            SpawnCommand::Clear => 69420,
        }
    }

    pub fn decode(packed: u128) -> Self {
        if packed == 69420 {
            return SpawnCommand::Clear;
        }

        let x = f32::from_bits((packed >> 96) as u32);
        let y = f32::from_bits((packed >> 64) as u32);
        let z = f32::from_bits((packed >> 32) as u32);
        let r = f32::from_bits(packed as u32);

        SpawnCommand::Sphere(Vec3::new(x, y, z), r)
    }
}

#[derive(Resource, FromLua, Clone, Debug)]
pub struct SpawnCommandBuffer {
    pub commands: Vec<SpawnCommand>,
}

pub fn startup(mut commands: Commands) {
    commands.insert_resource(SpawnCommandBuffer {
        commands: vec![],
    });
}

pub fn update(
    mut spawn_commands: ResMut<SpawnCommandBuffer>,
    mut commands: Commands,
    meshes: Query<Entity, With<MeshComponent>>,
) {
    let spawn_commands_read = &spawn_commands.commands;
    
    for command in spawn_commands_read {
        match command {
            SpawnCommand::Sphere(pos, radius) => {
                let sphere = sphere(16, *radius, *pos);

                commands.spawn(MeshComponent {
                    vertices: sphere.vertices,
                    indices: sphere.indices,
                    position: *pos,
                });
            },

            SpawnCommand::Clear => {
                for mesh in meshes.iter() {
                    commands.entity(mesh).remove::<MeshComponent>();
                }
            }
        }
    }

    if !spawn_commands.commands.is_empty() {
        spawn_commands.commands.pop();
    }

}