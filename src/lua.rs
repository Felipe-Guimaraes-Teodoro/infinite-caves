use std::{fs::File, io::Read};

use bevy_app::App;
use glam::vec3;
use mlua::Lua;
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

use crate::mesh_spawner::{SpawnCommand, SpawnCommandBuffer};

pub struct LuaIntegration {
    lua: Lua,
    buf: String,
}

impl LuaIntegration {
    pub fn new() -> Self {
        let lua = Lua::new();

        let spawn_command_buffer: Vec<u128> = vec![];

        lua.globals().set("spawn_commands", spawn_command_buffer).unwrap();
        let spawn_sphere = lua.create_function(|lua, (x, y, z, r): (f32, f32, f32, f32) | {
            let command = SpawnCommand::Sphere(vec3(x, y, z), r);
            lua.globals().set("spawn_commands", vec![command.encode()]).unwrap();
            lua.globals().get::<&str, Vec<u128>>("spawn_commands").unwrap().push(command.encode());
            
            Ok(())
        }).unwrap();
        let clear_world = lua.create_function(|lua, ()| {
            let command = SpawnCommand::Clear;
            lua.globals().set("spawn_commands", vec![command.encode()]).unwrap();
            lua.globals().get::<&str, Vec<u128>>("spawn_commands").unwrap().push(command.encode());
            
            Ok(())
        }).unwrap();

        lua.globals().set("spawn_sphere", spawn_sphere).unwrap();
        lua.globals().set("clear_world", clear_world).unwrap();

        Self {
            lua,
            buf: String::new()
        }
    }

    pub fn handle_input(&mut self, input: KeyboardInput) {
        if input.state == ElementState::Pressed {
            if let Some(keycode) = input.virtual_keycode {
                match keycode {
                    VirtualKeyCode::F5 => {
                        let mut file = File::open("script.lua").unwrap();
                        file.read_to_string(&mut self.buf).unwrap();

                        let chunk = self.lua.load(self.buf.as_str().trim());
                        chunk.exec().ok();

                        self.buf.clear();
                    }
                    _ => ()

                }
            }
        }
    }

    pub fn update(&mut self, app: &mut App) {
        let mut spawn_commands = app.world_mut().get_resource_mut::<SpawnCommandBuffer>().unwrap();
        let lua_spawn_commands = self.lua.globals().get::<&str, Vec<u128>>("spawn_commands").unwrap();
        spawn_commands.commands = lua_spawn_commands
            .iter()
            .map(|v| SpawnCommand::decode(*v))
            .collect();
        
        let clear: Vec<u128> = vec![];
        self.lua.globals().set("spawn_commands", clear).unwrap();
    }
}