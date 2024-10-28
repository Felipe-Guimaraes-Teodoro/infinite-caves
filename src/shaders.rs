pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 460

            layout(location = 0) in vec3 pos; // per vertex (could be expressed as an index of the chunk)

            layout (location = 1) in vec3 ofs; // per instance

            layout(set = 0, binding = 0) uniform Camera {
                mat4 view;
                mat4 proj;
            };

            layout(set = 1, binding = 0) uniform Model {
                mat4 model;
            };

            layout(location = 0) out vec4 o_pos;

            void main() {
                gl_Position = proj * view * model * vec4(pos + ofs, 1.0);

                o_pos = vec4(pos, 1.0);
            }
        ",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 460

            layout(location = 0) out vec4 f_color;

            layout(location = 0) in vec4 i_pos;

            void main() {
                float r = 0.5 + 0.5 * sin(i_pos.x * 0.2 + i_pos.y * 0.2);
                float g = 0.5 + 0.5 * cos(i_pos.y * 0.2 + i_pos.z * 0.2);
                float b = 0.5 + 0.5 * sin(i_pos.z * 0.2 + i_pos.x * 0.2);

                f_color = vec4(r, g, b, 1.0);
            }
        ",
    }
}