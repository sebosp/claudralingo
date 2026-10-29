// A key cap: a rounded box with a border, drawn a few pixels inside its node and anti-aliased
// with the derivatives of its distance, so the rotated keys (the Corne thumbs) keep smooth
// edges. The default UI shader draws the shape up to the edges of the node, where a rotated quad
// cuts its anti-aliasing.

// The output of the default vertex shader of the UI materials (bevy_ui_render).
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius_x: vec4<f32>,
    @location(3) border_radius_y: vec4<f32>,
    @location(4) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@group(1) @binding(0) var<uniform> fill: vec4<f32>;
@group(1) @binding(1) var<uniform> border: vec4<f32>;
// Logical pixels: x the corner radius, y the border width, z the margin to the node edges,
// w the width of the node.
@group(1) @binding(2) var<uniform> shape: vec4<f32>;
// The finger stripe at the bottom of the key.
@group(1) @binding(3) var<uniform> stripe: vec4<f32>;

fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // The size is in physical pixels, the shape in logical ones.
    let scale = in.size.x / max(shape.w, 1.0);
    let p = (in.uv - vec2<f32>(0.5)) * in.size;
    let half_size = in.size * 0.5 - vec2<f32>(shape.z * scale);
    let radius = min(shape.x * scale, min(half_size.x, half_size.y));
    let d = sd_rounded_box(p, half_size, radius);
    let aa = max(0.5 * fwidth(d), 0.0001);
    let coverage = 1.0 - smoothstep(-aa, aa, d);
    let on_border = smoothstep(-aa, aa, d + shape.y * scale);
    var color = mix(fill, border, on_border);
    // 36% of the width, 3 pixels high, 4 pixels above the bottom edge.
    let stripe_half = vec2<f32>(0.18 * in.size.x, 1.5 * scale);
    let stripe_center = vec2<f32>(0.0, half_size.y - (4.0 + 1.5) * scale);
    let ds = sd_rounded_box(p - stripe_center, stripe_half, 1.5 * scale);
    color = mix(color, stripe, stripe.a * (1.0 - smoothstep(-aa, aa, ds)));
    return vec4<f32>(color.rgb, color.a * coverage);
}
