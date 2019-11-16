use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

type GL = web_sys::WebGl2RenderingContext;

pub struct View {
    pub camera: nalgebra::Matrix4<f32>,
    pub viewport_start: [i32; 2],
    pub viewport_size: [i32; 2],
}

pub struct Renderer {
    canvas: web_sys::HtmlCanvasElement,
    gl: GL,
    world_tex: web_sys::WebGlTexture,

    program: web_sys::WebGlProgram,
    vao: web_sys::WebGlVertexArrayObject,
    vertex_buffer: web_sys::WebGlBuffer,

    framebuffer_tex: web_sys::WebGlTexture,
    framebuffer: web_sys::WebGlFramebuffer,

    postprocess_program: web_sys::WebGlProgram,
    postprocess_vao: web_sys::WebGlVertexArrayObject,
    postprocess_vertex_buffer: web_sys::WebGlBuffer,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.gl.delete_texture(Some(&self.world_tex));

        self.gl.delete_program(Some(&self.program));
        self.gl.delete_vertex_array(Some(&self.vao));
        self.gl.delete_buffer(Some(&self.vertex_buffer));

        self.gl.delete_framebuffer(Some(&self.framebuffer));
        self.gl.delete_texture(Some(&self.framebuffer_tex));

        self.gl.delete_program(Some(&self.postprocess_program));
        self.gl.delete_vertex_array(Some(&self.postprocess_vao));
        self.gl.delete_buffer(Some(&self.postprocess_vertex_buffer));
    }
}

impl Renderer {
    /// LEAVES WORLD TEXTURE UNINITIALIZED.
    pub fn new(canvas: &web_sys::HtmlCanvasElement) -> Self {
        let gl = canvas
            .get_context("webgl2")
            .unwrap_throw()
            .unwrap_throw()
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .unwrap_throw();

        gl.get_extension("EXT_color_buffer_float")
            .expect_throw("OpenGL extension \"EXT_color_buffer_float\" not found.")
            .expect_throw("OpenGL extension \"EXT_color_buffer_float\" not found.");
        gl.get_extension("EXT_float_blend")
            .expect_throw("OpenGL extension \"EXT_float_blend\" not found.")
            .expect_throw("OpenGL extension \"EXT_float_blend\" not found.");

        // Additive Blending
        gl.enable(GL::BLEND);
        gl.blend_func(GL::ONE, GL::ONE);

        let program = compile_program(&gl, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE);

        let vao = gl.create_vertex_array().unwrap_throw();
        gl.bind_vertex_array(Some(&vao));

        let vertex_buffer = gl.create_buffer().unwrap_throw();

        let attribute_pos = gl.get_attrib_location(&program, "pos") as u32;
        let attribute_normal = gl.get_attrib_location(&program, "normal") as u32;

        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vertex_buffer));
        gl.enable_vertex_attrib_array(attribute_pos);
        gl.vertex_attrib_pointer_with_i32(attribute_pos, 3, GL::FLOAT, false, 6 * 4, 0);
        gl.enable_vertex_attrib_array(attribute_normal);
        gl.vertex_attrib_pointer_with_i32(attribute_normal, 3, GL::FLOAT, false, 6 * 4, 3 * 4);

        {
            let mut data: Vec<f32> = Vec::new();

            #[rustfmt::skip]
            for i in 0..=WORLD_SIZE {
                data.extend_from_slice(&[
                    i as f32, 0.               , 0.               , 1., 0., 0.,
                    i as f32, WORLD_SIZE as f32, 0.               , 1., 0., 0.,
                    i as f32, WORLD_SIZE as f32, WORLD_SIZE as f32, 1., 0., 0.,
                    i as f32, WORLD_SIZE as f32, WORLD_SIZE as f32, 1., 0., 0.,
                    i as f32, 0.               , WORLD_SIZE as f32, 1., 0., 0.,
                    i as f32, 0.               , 0.               , 1., 0., 0.,

                    0.               , i as f32, 0.               , 0., 1., 0.,
                    WORLD_SIZE as f32, i as f32, 0.               , 0., 1., 0.,
                    WORLD_SIZE as f32, i as f32, WORLD_SIZE as f32, 0., 1., 0.,
                    WORLD_SIZE as f32, i as f32, WORLD_SIZE as f32, 0., 1., 0.,
                    0.               , i as f32, WORLD_SIZE as f32, 0., 1., 0.,
                    0.               , i as f32, 0.               , 0., 1., 0.,

                    0.               , 0.               , i as f32, 0., 0., 1.,
                    WORLD_SIZE as f32, 0.               , i as f32, 0., 0., 1.,
                    WORLD_SIZE as f32, WORLD_SIZE as f32, i as f32, 0., 0., 1.,
                    WORLD_SIZE as f32, WORLD_SIZE as f32, i as f32, 0., 0., 1.,
                    0.               , WORLD_SIZE as f32, i as f32, 0., 0., 1.,
                    0.               , 0.               , i as f32, 0., 0., 1.,

                ]);
            };

            gl.buffer_data_with_array_buffer_view(
                GL::ARRAY_BUFFER,
                &as_f32_array(&data).into(),
                GL::STATIC_DRAW,
            );
        }

        let world_tex = gl.create_texture().unwrap_throw();
        gl.bind_texture(GL::TEXTURE_3D, Some(&world_tex));
        gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_MIN_FILTER, GL::NEAREST as i32);
        gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_MAG_FILTER, GL::NEAREST as i32);
        gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_3D, GL::TEXTURE_WRAP_R, GL::CLAMP_TO_EDGE as i32);

        /*


        */

        let framebuffer_tex = gl.create_texture().unwrap_throw();
        gl.bind_texture(GL::TEXTURE_2D, Some(&framebuffer_tex));
        gl.pixel_storei(GL::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            GL::TEXTURE_2D,
            0,                      // level
            GL::RGBA32F as i32,     // internal_format
            canvas.width() as i32,  // width
            canvas.height() as i32, // height
            0,                      // border
            GL::RGBA,               // format
            GL::FLOAT,              // type
            None,
        )
        .unwrap_throw();
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::NEAREST as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::NEAREST as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);

        let framebuffer = gl.create_framebuffer().unwrap_throw();
        gl.bind_framebuffer(GL::FRAMEBUFFER, Some(&framebuffer));
        gl.framebuffer_texture_2d(
            GL::FRAMEBUFFER,
            GL::COLOR_ATTACHMENT0,
            GL::TEXTURE_2D,
            Some(&framebuffer_tex),
            0,
        );

        let postprocess_program = compile_program(
            &gl,
            POSTPROCESS_VERTEX_SHADER_SOURCE,
            POSTPROCESS_FRAGMENT_SHADER_SOURCE,
        );

        let postprocess_vao = gl.create_vertex_array().unwrap_throw();
        gl.bind_vertex_array(Some(&postprocess_vao));

        let postprocess_vertex_buffer = gl.create_buffer().unwrap_throw();

        let postprocess_attribute_coord =
            gl.get_attrib_location(&postprocess_program, "coord") as u32;

        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&postprocess_vertex_buffer));
        gl.enable_vertex_attrib_array(postprocess_attribute_coord);
        gl.vertex_attrib_pointer_with_i32(postprocess_attribute_coord, 2, GL::FLOAT, false, 0, 0);
        gl.buffer_data_with_array_buffer_view(
            GL::ARRAY_BUFFER,
            &as_f32_array(&[0., 0., 0., 1., 1., 1., 1., 1., 1., 0., 0., 0.]),
            GL::STATIC_DRAW,
        );

        Self {
            canvas: canvas.clone(),
            gl,
            world_tex,

            program,
            vao,
            vertex_buffer,

            framebuffer_tex,
            framebuffer,

            postprocess_program,
            postprocess_vao,
            postprocess_vertex_buffer,
        }
    }

    /// BORDER MUST BE BLACK
    pub fn set_world_tex(&mut self, data: &[u8]) {
        self.gl.bind_texture(GL::TEXTURE_3D, Some(&self.world_tex));
        self.gl.pixel_storei(GL::UNPACK_ALIGNMENT, 1);
        self.gl
            .tex_image_3d_with_opt_u8_array(
                GL::TEXTURE_3D,
                0,                 // level
                GL::R8UI as i32,   // internal_format
                WORLD_SIZE as i32, // width
                WORLD_SIZE as i32, // height
                WORLD_SIZE as i32, // depth
                0,                 // border
                GL::RED_INTEGER,   // format
                GL::UNSIGNED_BYTE, // type
                Some(data),
            )
            .unwrap_throw();
    }

    pub fn render(
        &self,
        views: Vec<View>,
        brightness: f32,
        opacity: f32,
        colors: impl IntoIterator<Item = f32>,
    ) {
        self.gl
            .bind_framebuffer(GL::FRAMEBUFFER, Some(&self.framebuffer));
        self.gl.use_program(Some(&self.program));
        self.gl.bind_vertex_array(Some(&self.vao));

        self.gl.bind_texture(GL::TEXTURE_3D, Some(&self.world_tex));
        self.gl.uniform1i(
            self.gl
                .get_uniform_location(&self.program, "world")
                .as_ref(),
            0,
        );

        #[rustfmt::skip]
        self.gl.uniform3fv_with_f32_array(
            self.gl
                .get_uniform_location(&self.program, "colors")
                .as_ref(),
            &[0.0, 0.0, 0.0].iter().copied().chain(colors).collect::<Vec<_>>()
        );

        self.gl.clear_color(0., 0., 0., 1.);
        self.gl.clear(GL::COLOR_BUFFER_BIT);

        for view in views {
            self.gl.uniform_matrix4fv_with_f32_array(
                self.gl
                    .get_uniform_location(&self.program, "camera")
                    .as_ref(),
                false,
                &view.camera.as_slice(),
            );

            let camera_pos = view.camera.try_inverse().unwrap_throw() * nalgebra::Vector4::z();

            self.gl.uniform3f(
                self.gl
                    .get_uniform_location(&self.program, "camera_pos")
                    .as_ref(),
                camera_pos[0] / camera_pos[3],
                camera_pos[1] / camera_pos[3],
                camera_pos[2] / camera_pos[3],
            );

            self.gl.uniform1f(
                self.gl
                    .get_uniform_location(&self.program, "brightness")
                    .as_ref(),
                brightness,
            );

            self.gl.uniform1f(
                self.gl
                    .get_uniform_location(&self.program, "opacity")
                    .as_ref(),
                opacity,
            );

            self.gl.viewport(
                view.viewport_start[0],
                view.viewport_start[1],
                view.viewport_size[0],
                view.viewport_size[1],
            );
            self.gl
                .draw_arrays(GL::TRIANGLES, 0, 6 * 3 * (WORLD_SIZE + 1) as i32);
        }

        self.gl.bind_framebuffer(GL::FRAMEBUFFER, None);
        self.gl.use_program(Some(&self.postprocess_program));
        self.gl.bind_vertex_array(Some(&self.postprocess_vao));

        self.gl.viewport(
            0,
            0,
            self.canvas.width() as i32,
            self.canvas.height() as i32,
        );
        self.gl.clear_color(0., 0., 0., 1.);
        self.gl.clear(GL::COLOR_BUFFER_BIT);

        self.gl
            .bind_texture(GL::TEXTURE_2D, Some(&self.framebuffer_tex));
        self.gl.uniform1i(
            self.gl.get_uniform_location(&self.program, "tex").as_ref(),
            0,
        );

        self.gl.draw_arrays(GL::TRIANGLES, 0, 6);
    }
}

fn compile_program(gl: &GL, v_source: &str, f_source: &str) -> web_sys::WebGlProgram {
    let vertex_shader = gl.create_shader(GL::VERTEX_SHADER).unwrap_throw();
    gl.shader_source(&vertex_shader, v_source);
    gl.compile_shader(&vertex_shader);

    web_sys::console::log_1(&gl.get_shader_info_log(&vertex_shader).unwrap_throw().into());

    let fragment_shader = gl.create_shader(GL::FRAGMENT_SHADER).unwrap_throw();
    gl.shader_source(&fragment_shader, f_source);
    gl.compile_shader(&fragment_shader);

    web_sys::console::log_1(
        &gl.get_shader_info_log(&fragment_shader)
            .unwrap_throw()
            .into(),
    );

    let program = gl.create_program().unwrap_throw();
    gl.attach_shader(&program, &vertex_shader);
    gl.attach_shader(&program, &fragment_shader);
    gl.link_program(&program);

    gl.delete_shader(Some(&vertex_shader));
    gl.delete_shader(Some(&fragment_shader));

    program
}

const VERTEX_SHADER_SOURCE: &str = r"#version 300 es

in vec3 pos;
in vec3 normal;

out vec3 vpos;
out vec3 vnormal;

uniform mat4 camera;

void main() {
    vpos = pos;
    vnormal = normal;

    vec4 out_pos = camera * vec4(vpos, 1.0);
    out_pos.z = 0.0;

    gl_Position = out_pos;
}
";

// When we sum this over all surfaces, we get
//     (col_0 - col_1) * d_1 + (col_1 - col_2) * d_2 + (col_2 - col_3) * d_3 + ... + (col_(n-1) - col_n) * d_n
//   = col_0 * d_1 - col_1 * d_1 + col_1 * d_2 - col_2 * d_2 + col_2 * d_3 - col_3 * d_3 + ... + col_(n-1) * d_n - col_n * d_n
//   = col_0 * d_1 + col_1 * (d_2 - d_1) + col_2 * (d_3 - d_2) + ... + col_(n-1) * (d_n - d_(n-1)) + col_n * d_n
//
// This is what we want, plus (col_n * d_n). So if col_n is black, this is the correct answer.
const FRAGMENT_SHADER_SOURCE: &str = r"#version 300 es
precision mediump float;
precision mediump usampler3D;

in vec3 vpos;
in vec3 vnormal;

out vec4 color;

uniform vec3 camera_pos;
uniform usampler3D world;
uniform float brightness;
uniform float opacity;
uniform vec3 colors[6];

const float world_size = 128.0;

vec3 get_color(vec3 pos) {
    return colors[texture(world, pos / world_size).r];
}

// ∫opacity^x dx
float light_integral(float dist) {
    if (opacity == 0.0) {
        return dist;
    } else {
        return (pow(1.0 - opacity, dist) - 1.0) / log(1.0 - opacity);
    }
}

void main() {
    vec3 n = vnormal * sign(dot(vnormal, vpos - camera_pos)) * 0.5;
    vec3 near_color = get_color(vpos - n);
    vec3 far_color = get_color(vpos + n);

    color = vec4(near_color - far_color, 0.0) * light_integral(distance(vpos, camera_pos)) * brightness;
}
";

// Value separately defined in fragment shader above.
pub const WORLD_SIZE: usize = 128;

fn as_f32_array(v: &[f32]) -> js_sys::Float32Array {
    let memory_buffer = wasm_bindgen::memory()
        .dyn_into::<js_sys::WebAssembly::Memory>()
        .unwrap_throw()
        .buffer();

    let location = v.as_ptr() as u32 / 4;

    js_sys::Float32Array::new(&memory_buffer).subarray(location, location + v.len() as u32)
}

const POSTPROCESS_VERTEX_SHADER_SOURCE: &str = r"#version 300 es

in vec2 coord;
out vec2 vcoord;

void main() {
    vcoord = coord;
    gl_Position = vec4(coord * 2.0 - 1.0, 0.0, 1.0);
}

";

const POSTPROCESS_FRAGMENT_SHADER_SOURCE: &str = r"#version 300 es

precision mediump float;

in vec2 vcoord;
out vec4 color;
uniform sampler2D tex;

void main() {
    color = texture(tex, vcoord);
}

";
