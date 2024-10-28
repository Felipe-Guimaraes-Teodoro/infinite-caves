// #![allow(unused)]

use chaos_vk::graphics::vertex::PosVertex;
use glam::Vec3;

pub struct GeometryData {
    pub vertices: Vec<PosVertex>,
    pub indices: Vec<u32>,
}


pub fn sphere(iterations: usize, radius: f32, pos: Vec3) -> GeometryData {
    let mut vertices = vec![];
    let pi = std::f32::consts::PI;

    for lat in 0..=iterations {
        let theta = pi * lat as f32 / iterations as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=iterations {
            let phi = 2.0 * pi * lon as f32 / iterations as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = cos_phi * sin_theta * radius;
            let y = cos_theta * radius;
            let z = sin_phi * sin_theta * radius;

            // let s = lon as f32 / iterations as f32;
            // let t = 1.0 - (lat as f32 / iterations as f32);

            // let normal = vec3(x, y, z).normalize();

            vertices.push(PosVertex {
                pos: [x, y, z],
            });
        }
    }

    let mut indices: Vec<u32> = vec![];
    for lat in 0..iterations {
        for lon in 0..iterations {
            let first = lat * (iterations + 1) + lon;
            let second = first + iterations + 1;

            indices.push(first as u32);
            indices.push(second as u32);
            indices.push((first + 1) as u32);

            indices.push(second as u32);
            indices.push((second + 1) as u32);
            indices.push((first + 1) as u32);
        }
    }

    for vertex in &mut vertices {
        vertex.pos[0] += pos.x;
        vertex.pos[1] += pos.y;
        vertex.pos[2] += pos.z
    }

    GeometryData {
        vertices,
        indices,
    }
}

pub mod voxel_gen {
    use chaos_vk::graphics::vertex::PosVertex;
    use glam::Vec3;
    use crate::world::{Voxel, CHUNK_SIZE};

    pub fn gen_mesh_data_culled(voxels: &Vec<Voxel>, lod: usize) -> (Vec<PosVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for x in (0..CHUNK_SIZE).step_by(lod) {
            for y in (0..CHUNK_SIZE).step_by(lod) {
                for z in (0..CHUNK_SIZE).step_by(lod) {
                    let idx = x * CHUNK_SIZE * CHUNK_SIZE + y * CHUNK_SIZE + z;
                    let voxel = &voxels[idx];

                    if voxel.id == 0 {
                        continue;
                    }
                    
                    let mut visible_faces = [false; 6];

                    if is_visible(&voxels, idx, (-(lod as isize), 0, 0)) { visible_faces[0] = true; }
                    if is_visible(&voxels, idx, (lod as isize, 0, 0)) { visible_faces[1] = true; }
                    if is_visible(&voxels, idx, (0, -(lod as isize), 0)) { visible_faces[2] = true; }
                    if is_visible(&voxels, idx, (0, lod as isize, 0)) { visible_faces[3] = true; }
                    if is_visible(&voxels, idx, (0, 0, -(lod as isize))) { visible_faces[4] = true; }
                    if is_visible(&voxels, idx, (0, 0, lod as isize)) { visible_faces[5] = true; }

                    for (face_idx, &visible) in visible_faces.iter().enumerate() {
                        if !visible {
                            continue;
                        }

                        let start_vertex_idx = vertices.len() as u32;

                        match face_idx {
                            0 => { // left face
                                vertices.push(PosVertex { pos: [x as f32, y as f32, z as f32] });
                                vertices.push(PosVertex { pos: [x as f32, y as f32, (z + lod) as f32] });
                                vertices.push(PosVertex { pos: [x as f32, (y + lod) as f32, (z + lod) as f32] });
                                vertices.push(PosVertex { pos: [x as f32, (y + lod) as f32, z as f32] });
                            },
                            1 => { // right face
                                vertices.push(PosVertex { pos: [(x + lod) as f32, y as f32, z as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, (y + lod) as f32, z as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, (y + lod) as f32, (z + lod) as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, y as f32, (z + lod) as f32] });
                            },
                            2 => { // bottom face
                                vertices.push(PosVertex { pos: [x as f32, y as f32, z as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, y as f32, z as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, y as f32, (z + lod) as f32] });
                                vertices.push(PosVertex { pos: [x as f32, y as f32, (z + lod) as f32] });
                            },
                            3 => { // top face
                                vertices.push(PosVertex { pos: [x as f32, (y + lod) as f32, z as f32] });
                                vertices.push(PosVertex { pos: [x as f32, (y + lod) as f32, (z + lod) as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, (y + lod) as f32, (z + lod) as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, (y + lod) as f32, z as f32] });
                            },
                            4 => { // back face
                                vertices.push(PosVertex { pos: [x as f32, y as f32, z as f32] });
                                vertices.push(PosVertex { pos: [x as f32, (y + lod) as f32, z as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, (y + lod) as f32, z as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, y as f32, z as f32] });
                            },
                            5 => { // front face
                                vertices.push(PosVertex { pos: [x as f32, y as f32, (z + lod) as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, y as f32, (z + lod) as f32] });
                                vertices.push(PosVertex { pos: [(x + lod) as f32, (y + lod) as f32, (z + lod) as f32] });
                                vertices.push(PosVertex { pos: [x as f32, (y + lod) as f32, (z + lod) as f32] });
                            },
                            _ => {}
                        }

                        indices.push(start_vertex_idx);
                        indices.push(start_vertex_idx + 1);
                        indices.push(start_vertex_idx + 2);
                        indices.push(start_vertex_idx + 2);
                        indices.push(start_vertex_idx + 3);
                        indices.push(start_vertex_idx);
                    }
                }
            }
        }

        (vertices, indices)
    }
    
    pub fn get_voxel(pos: Vec3) -> usize {
        let x = ((pos.x as i32 % CHUNK_SIZE as i32 + CHUNK_SIZE as i32) % CHUNK_SIZE as i32) as usize;
        let y = ((pos.y as i32 % CHUNK_SIZE as i32 + CHUNK_SIZE as i32) % CHUNK_SIZE as i32) as usize;
        let z = ((pos.z as i32 % CHUNK_SIZE as i32 + CHUNK_SIZE as i32) % CHUNK_SIZE as i32) as usize;
    
        x * (CHUNK_SIZE * CHUNK_SIZE) + y * CHUNK_SIZE + z
    }

    pub fn get_pos(idx: usize) -> (isize, isize, isize) {
        let x = (idx / (CHUNK_SIZE * CHUNK_SIZE)) as isize;
        let y = ((idx % (CHUNK_SIZE * CHUNK_SIZE)) / CHUNK_SIZE) as isize;
        let z = (idx % CHUNK_SIZE) as isize;

        (x, y, z)
    }

    pub fn get_index(x: usize, y: usize, z: usize) -> usize {
        x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
    }
    
    pub fn is_visible(voxels: &Vec<Voxel>, idx: usize, direction: (isize, isize, isize)) -> bool {
        let (x, y, z) = get_pos(idx);
    
        let (dx, dy, dz) = direction;
    
        let nx = x + dx;
        let ny = y + dy;
        let nz = z + dz;
    
        if nx < 0 || nx >= CHUNK_SIZE as isize
        || ny < 0 || ny >= CHUNK_SIZE as isize
        || nz < 0 || nz >= CHUNK_SIZE as isize {
            return true;
        }
    
        let neighbor_pos = nx as usize * (CHUNK_SIZE * CHUNK_SIZE)
                        + ny as usize * CHUNK_SIZE
                        + nz as usize;
    
        voxels[neighbor_pos].id == 0
    }
}