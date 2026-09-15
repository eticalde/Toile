struct Uniforms {
    mvp: mat4x4<f32>,
    // Target width and height in pixels, then the tape's half-width in pixels.
    view: vec4<f32>,
    color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs_main(
    @location(0) here: vec3<f32>,
    @location(1) there: vec3<f32>,
    @location(2) side: f32,
) -> @builtin(position) vec4<f32> {
    let a = u.mvp * vec4<f32>(here, 1.0);
    let b = u.mvp * vec4<f32>(there, 1.0);
    // Widened in pixels rather than in metres, so the tape reads the same at
    // every zoom; the depth stays the centreline's, so the body still hides
    // whatever runs behind it.
    let half = u.view.xy * 0.5;
    let pa = a.xy / max(a.w, 1e-4) * half;
    let pb = b.xy / max(b.w, 1e-4) * half;
    let d = pb - pa;
    let along = d / max(length(d), 1e-6);
    let across = vec2<f32>(-along.y, along.x) * side * u.view.z;
    return vec4<f32>(a.xy + across / half * a.w, a.z, a.w);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return u.color;
}
