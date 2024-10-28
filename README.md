# Infinite Caves
Based on the output of a perlin noise algorithm, a voxel can either assume a state of visible or not, resulting in what appears to be an unending cave system. The goal of this repository is to render and manage the unique structures within these cave systems.

This repository also serves as testing grounds for my vulkan crate `chaos-vk`, available on [crates.io](https://crates.io/crates/chaos-vk). It also utilizes `bevy-ecs`, for the managing of some parts of the program and `rlua`, for integrating Lua.

## Using Lua
1. Within the root folder, create a file named `script.lua`, which is where the program will read the script.
2. In order to execute, press `F5`.
3. The available commands are: `spawn_sphere(x, y, z, radius)` and `clear_world()`. Keep in mind that some of spawn_sphere's arguments (x, y and z) are still error-prone.

## Preview
![image](https://github.com/user-attachments/assets/329f84f0-d8ec-49d9-bff3-74713c6b1462)
