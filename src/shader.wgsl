struct Uniforms {
    window_width: f32,
    window_height: f32,
    fb_width: f32,
    fb_height: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

// Generate a fullscreen quad (two triangles)
// 6 vertices: 0,1,2 for first triangle, 2,1,3 for second triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Vertex positions for two triangles forming a quad
    // Triangle 1: (0,0), (1,0), (0,1)
    // Triangle 2: (0,1), (1,0), (1,1)
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),  // bottom-left
        vec2<f32>(1.0, 0.0),  // bottom-right
        vec2<f32>(0.0, 1.0),  // top-left
        vec2<f32>(0.0, 1.0),  // top-left
        vec2<f32>(1.0, 0.0),  // bottom-right
        vec2<f32>(1.0, 1.0)   // top-right
    );

    let pos = positions[vertex_index];

    // Integer-only scaling when window is large enough
    var imgsize_x = uniforms.window_width;
    var imgsize_y = uniforms.window_height;
    var topleft_x = 0.0;
    var topleft_y = 0.0;

    if (uniforms.window_width > uniforms.fb_width && uniforms.window_height > uniforms.fb_height) {
        // Calculate integer scale factor
        let scale = floor(min(uniforms.window_width / uniforms.fb_width, uniforms.window_height / uniforms.fb_height));
        imgsize_x = uniforms.fb_width * scale;
        imgsize_y = uniforms.fb_height * scale;
        topleft_x = floor((uniforms.window_width - imgsize_x) / 2.0);
        topleft_y = floor((uniforms.window_height - imgsize_y) / 2.0);
    }

    // Convert to normalized device coordinates (-1 to 1)
    // Map from pixel coordinates to NDC
    let pixel_x = topleft_x + pos.x * imgsize_x;
    let pixel_y = topleft_y + pos.y * imgsize_y;

    let ndc_x = (pixel_x / uniforms.window_width) * 2.0 - 1.0;
    let ndc_y = (pixel_y / uniforms.window_height) * 2.0 - 1.0;

    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);

    // Texture coordinates (flip Y for correct orientation)
    out.tex_coords = vec2<f32>(pos.x, 1.0 - pos.y);

    return out;
}

@group(0) @binding(0)
var t_framebuffer: texture_2d<f32>;
@group(0) @binding(1)
var s_framebuffer: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_framebuffer, s_framebuffer, in.tex_coords);
}
