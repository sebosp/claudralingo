// The background: the edges of a low poly mesh drifting slowly, its triangles join and dissolve,
// its vertices take the colors of the fingers and layers of the most missed keys. Under the text
// to type and the hands, the lines blur away.

// The output of the default vertex shader of the UI materials (bevy_ui_render).
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius_x: vec4<f32>,
    @location(3) border_radius_y: vec4<f32>,
    @location(4) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@group(1) @binding(0) var<uniform> color0: vec4<f32>;
@group(1) @binding(1) var<uniform> color1: vec4<f32>;
@group(1) @binding(2) var<uniform> color2: vec4<f32>;
@group(1) @binding(3) var<uniform> color3: vec4<f32>;
// x the time in seconds, y unused, z the opacity of the edges and the vertices, w the number of
// columns of the mesh.
@group(1) @binding(4) var<uniform> params: vec4<f32>;
// The areas where the mesh blurs, in uv units: min x, min y, max x, max y.
@group(1) @binding(5) var<uniform> blur0: vec4<f32>;
@group(1) @binding(6) var<uniform> blur1: vec4<f32>;

// 1 inside an area, 0 far from it, in pixels.
fn blur_in(p: vec2<f32>, area: vec4<f32>, size: vec2<f32>, feather: f32) -> f32 {
    let lo = area.xy * size;
    let hi = area.zw * size;
    if hi.x <= lo.x {
        return 0.0;
    }
    let center = (lo + hi) * 0.5;
    let half_size = (hi - lo) * 0.5;
    let q = abs(p - center) - half_size;
    let d = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
    return 1.0 - smoothstep(-feather, feather, d);
}

const TAU: f32 = 6.2831853;

fn hash1(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn hash2(p: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)));
    return fract(sin(q) * 43758.5453);
}

fn palette(i: u32) -> vec4<f32> {
    let k = i % 4u;
    if k == 0u {
        return color0;
    }
    if k == 1u {
        return color1;
    }
    if k == 2u {
        return color2;
    }
    return color3;
}

// The vertex of the grid point g wanders inside its cell, the triangles never fold.
fn vertex_at(g: vec2<f32>, t: f32) -> vec2<f32> {
    let h = hash2(g);
    let wobble = vec2<f32>(
        sin(t * (0.11 + 0.09 * h.x) + h.y * TAU),
        cos(t * (0.13 + 0.07 * h.y) + h.x * TAU),
    );
    return g + vec2<f32>(0.5) + 0.33 * wobble;
}

// The color of a vertex drifts through the palette.
fn vertex_color(g: vec2<f32>, t: f32) -> vec4<f32> {
    let phase = hash1(g) * 4.0 + t * 0.02;
    let i = u32(floor(phase));
    return mix(palette(i), palette(i + 1u), smoothstep(0.0, 1.0, fract(phase)));
}

fn barycentric(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> vec3<f32> {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    let d = v0.x * v1.y - v1.x * v0.y;
    let v = (v2.x * v1.y - v1.x * v2.y) / d;
    let w = (v0.x * v2.y - v2.x * v0.y) / d;
    return vec3<f32>(1.0 - v - w, v, w);
}

// How much a face is there: the faces join and dissolve, each at its own pace.
fn life(id: vec2<f32>, k: f32, t: f32) -> f32 {
    let h = hash1(id + vec2<f32>(k * 17.0, k * 3.0));
    let s = 0.5 + 0.5 * sin(t * (0.05 + 0.08 * h) + h * TAU);
    return smoothstep(0.3, 0.95, s);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let t = params.x;
    let cell = in.size.x / max(params.w, 1.0);
    let p = in.uv * in.size / cell;
    let base = floor(p) - vec2<f32>(1.0);

    // The triangle of the fragment: the vertices stay in their cells, so it is one of the two
    // triangles of the four quads around.
    var w = vec3<f32>(0.0);
    var ga = vec2<f32>(0.0);
    var gb = vec2<f32>(0.0);
    var gc = vec2<f32>(0.0);
    var va = vec2<f32>(0.0);
    var vb = vec2<f32>(0.0);
    var vc = vec2<f32>(0.0);
    var id = vec2<f32>(0.0);
    var face = 0.0;
    var found = false;
    for (var j = 0; j < 2; j++) {
        for (var i = 0; i < 2; i++) {
            let g = base + vec2<f32>(f32(i), f32(j));
            // A var: the arrays indexed with a variable can not be constants.
            var corners = array<vec2<f32>, 4>(g, g + vec2<f32>(1.0, 0.0), g + vec2<f32>(0.0, 1.0), g + vec2<f32>(1.0, 1.0));
            // The quads are cut along one diagonal or the other.
            let flip = hash1(g + vec2<f32>(0.37, 0.71)) > 0.5;
            for (var k = 0; k < 2; k++) {
                var tri = vec3<i32>(0, 1, 3);
                if !flip && k == 1 {
                    tri = vec3<i32>(0, 3, 2);
                } else if flip && k == 0 {
                    tri = vec3<i32>(0, 1, 2);
                } else if flip && k == 1 {
                    tri = vec3<i32>(1, 3, 2);
                }
                let a = vertex_at(corners[tri.x], t);
                let b = vertex_at(corners[tri.y], t);
                let c = vertex_at(corners[tri.z], t);
                let bary = barycentric(p, a, b, c);
                if !found && min(bary.x, min(bary.y, bary.z)) >= -0.0001 {
                    found = true;
                    w = bary;
                    ga = corners[tri.x];
                    gb = corners[tri.y];
                    gc = corners[tri.z];
                    va = a;
                    vb = b;
                    vc = c;
                    id = g;
                    face = f32(k);
                }
            }
        }
    }

    let ca = vertex_color(ga, t);
    let cb = vertex_color(gb, t);
    let cc = vertex_color(gc, t);
    let color = ca * w.x + cb * w.y + cc * w.z;
    let alive = life(id, face, t);

    // Sharp on the empty space, very blurred under the text and the hands.
    let px = in.uv * in.size;
    let feather = 0.04 * in.size.y;
    let blur = max(blur_in(px, blur0, in.size, feather), blur_in(px, blur1, in.size, feather));

    // The edges, about one pixel wide, up to twenty when blurred, fainter as they widen. The
    // derivatives are taken out of the loops.
    let e = min(w.x, min(w.y, w.z));
    let width = mix(1.5, 22.0, blur) * fwidth(e);
    let soft = 1.0 - smoothstep(0.0, width, e);
    let edge = soft * soft;
    // The vertices, small dots in their own color, a wide soft glow when blurred.
    let da = length(p - va) * cell;
    let db = length(p - vb) * cell;
    let dc = length(p - vc) * cell;
    var dot_color = ca;
    var dist = da;
    if db < dist {
        dist = db;
        dot_color = cb;
    }
    if dc < dist {
        dist = dc;
        dot_color = cc;
    }
    let radius = mix(0.035, 0.12, blur) * cell;
    let aa = mix(1.0, radius, blur);
    let on_dot = 1.0 - smoothstep(radius - aa, radius + aa, dist);
    let glow = exp(-dist * dist / (mix(0.015, 0.08, blur) * cell * cell)) * 0.3;

    // No faces: the lines join and dissolve with their triangle.
    let fade = mix(1.0, 0.3, blur);
    let edge_alpha = params.z * (0.15 + 0.85 * alive) * edge * fade;
    let dot_alpha = min(1.0, params.z * 2.5) * max(on_dot, glow) * fade;
    var rgb = color.rgb;
    var alpha = edge_alpha;
    if dot_alpha > alpha {
        rgb = mix(rgb, dot_color.rgb, on_dot);
        alpha = dot_alpha;
    }
    if !found {
        alpha = 0.0;
    }
    return vec4<f32>(rgb, alpha);
}
