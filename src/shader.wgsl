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

    // Convert from 0-1 range to -1 to 1 clip space
    let x = pos.x * 2.0 - 1.0;
    let y = pos.y * 2.0 - 1.0;

    // Calculate aspect ratios
    let window_aspect = uniforms.window_width / uniforms.window_height;
    let fb_aspect = uniforms.fb_width / uniforms.fb_height; // 4:3 = 1.333...

    // Calculate scale to maintain 4:3 aspect ratio
    var scale_x = 1.0;
    var scale_y = 1.0;

    if (window_aspect > fb_aspect) {
        // Window is wider than 4:3 - add pillarboxing (black bars on sides)
        scale_x = fb_aspect / window_aspect;
    } else {
        // Window is taller than 4:3 - add letterboxing (black bars top/bottom)
        scale_y = window_aspect / fb_aspect;
    }

    // Apply scaling to maintain aspect ratio
    let scaled_x = x * scale_x;
    let scaled_y = y * scale_y;

    out.clip_position = vec4<f32>(scaled_x, scaled_y, 0.0, 1.0);

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
