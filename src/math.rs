use std::{f32::consts::PI, ops::{Add, Mul, Sub}};
use glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};
use rand::prelude::*;

/* i really have no idea how to deal with indentation when generics come into place */


pub fn distance(a: f32, b: f32) -> f32 {
    return f32::sqrt(a*a + b*b);
}

pub fn lerp
    <T: Sub<Output = T> + Add<Output = T> + Mul<Output = T> + Copy + Mul<f32, Output = T>>
    (min: T, max: T, t: f32) -> T 
{
    return min + (max - min) * t;
}

// generates a random value T between n1: T and n2: T
pub fn rand_betw
<
    T: std::cmp::PartialOrd +
    rand::distributions::uniform::SampleUniform,
>
(
    n1: T, 
    n2: T
) -> T {
    let mut r = thread_rng();
    r.gen_range(n1..n2)
}

pub fn rand_vec2() -> Vec2 {
    vec2(rand_betw(0.0, 1.0), rand_betw(0.0, 1.0))
}

pub fn rand_vec3() -> Vec3 {
    vec3(rand_betw(0.0, 1.0), rand_betw(0.0, 1.0), rand_betw(0.0, 1.0))
}

pub fn rand_vec4() -> Vec4 {
    vec4(rand_betw(0.0, 1.0), rand_betw(0.0, 1.0), rand_betw(0.0, 1.0), rand_betw(0.0, 1.0))
}

/* make it so that input x yields in a smooth output y */
pub struct SecondOrderDynamics<T> { 
    // previous inputs
    xp: T, 
    y: T, 
    yd: T,

    //constants
    k1: f32,
    k2: f32, 
    k3: f32,

    t_critical: f32,
}

impl SecondOrderDynamics<Vec3> {
    pub fn new(f: f32, z: f32, r: f32, x0: Vec3) -> Self {
        let k1 = z / (PI * f);
        let k2 = 1.0 / ((2.0 * PI * f) * (2.0 * PI * f));
        let k3 = r * z / (2.0 * PI * f);
        
        let xp = x0;
        let y = x0;
        let yd = vec3(0.0, 0.0, 0.0);

        // critical timestep threshold where the simulation would
        // become unstable past it

        let t_critical = (f32::sqrt(4.0*k2 + k1 * k1) - k1) * 0.8; // multiply by an arbitrary value to be safe

        Self {
            k1,
            k2,
            k3,

            xp,
            y,
            yd,

            t_critical,
        }
    }

    pub fn update(&mut self, mut timestep: f32, x: Vec3) -> Vec3 {
        let xd = (x - self.xp) / timestep;
        self.xp = x;

        let iterations = f32::ceil(timestep / self.t_critical); // take extra iterations if t > tcrit
        timestep = timestep / iterations; // lower timesteps

        for _ in 0..iterations as usize {
            self.y = self.y + timestep * self.yd;
            self.yd = self.yd + timestep * (x + self.k3*xd - self.y - self.k1*self.yd) / self.k2;
        }

        self.y
    }
}