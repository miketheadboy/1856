// The look (src/look.rs, docs/ART.md): printed matter over the whole frame.
// Order matters and follows the press: the plates go down (registration),
// the ink spreads into the paper (bleed), the paper shows (fiber, grain),
// then age (desaturate, sepia) and the edges of the sheet (vignette).

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct Look {
    time: f32,
    grain: f32,
    grain_size: f32,
    fiber: f32,
    bleed: f32,
    misreg: f32,
    vignette: f32,
    sepia: f32,
    desat: f32,
    halftone: f32,
    tension: f32,
    night: f32,
    boil_fps: f32,
    tension_misreg: f32,
    tension_grain: f32,
    night_vignette: f32,
}

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> look: Look;

fn hash(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 45.32);
    return fract(r.x * r.y);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

fn lum(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

// How much a color belongs to the second plate: the warm accents, oxblood
// and brass. Saturated and red-leaning; the greens and the slate blue stay
// on the key plate and stay in register.
fn accent(c: vec3<f32>) -> f32 {
    let sat = max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
    let warm = c.r - max(c.g * 0.8, c.b);
    return smoothstep(0.12, 0.3, sat) * smoothstep(0.02, 0.15, warm);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(screen));
    let px = 1.0 / dims;
    let uv = in.uv;
    var c = textureSample(screen, samp, uv).rgb;

    // Registration: the accent plate went down a hair off. Where either the
    // true or the shifted pixel is accent, take the shifted one.
    let off_px = look.misreg + look.tension_misreg * look.tension;
    let shifted = textureSample(screen, samp, uv + px * off_px * vec2<f32>(1.0, -0.6)).rgb;
    c = mix(c, shifted, max(accent(shifted), accent(c)) * 0.85);

    // Ink bleed: dark ink creeps a pixel into the paper around it.
    let n = min(
        min(textureSample(screen, samp, uv + vec2<f32>(px.x, 0.0)).rgb,
            textureSample(screen, samp, uv - vec2<f32>(px.x, 0.0)).rgb),
        min(textureSample(screen, samp, uv + vec2<f32>(0.0, px.y)).rgb,
            textureSample(screen, samp, uv - vec2<f32>(0.0, px.y)).rgb),
    );
    let ink = 1.0 - smoothstep(0.08, 0.35, lum(n));
    c = mix(c, min(c, n), look.bleed * ink);

    // Halftone in the shadows, off by default: a screen at 45 degrees.
    if (look.halftone > 0.0) {
        let p = in.position.xy;
        let r = vec2<f32>(p.x + p.y, p.x - p.y) * 0.7071 / 4.0;
        let d = length(fract(r) - 0.5);
        let dark = 1.0 - lum(c);
        let dot_ink = 1.0 - smoothstep(dark * 0.6 - 0.05, dark * 0.6 + 0.05, d);
        c = mix(c, c * (1.0 - dot_ink * 0.6), look.halftone * smoothstep(0.3, 0.8, dark));
    }

    // Paper: long fibers (stretched noise) and grain that changes in
    // printed frames, not every video frame.
    let fib = noise(in.position.xy * vec2<f32>(0.012, 0.05));
    c *= 1.0 - look.fiber * fib;
    let frame = floor(look.time * max(look.boil_fps, 1.0));
    let cell = floor(in.position.xy / max(look.grain_size, 1.0));
    let g = hash(cell + vec2<f32>(frame * 17.31, frame * 9.13)) - 0.5;
    // Grain lives in the paper, not in solid ink: strongest in the
    // midtones and lights, almost none in the blacks.
    let amount = look.grain + look.tension_grain * look.tension;
    let tone = lum(c);
    let paper = 0.06 + 3.76 * tone * (1.0 - tone);
    c += g * amount * paper;

    // Age: a little less saturated, a little browner.
    let l = lum(c);
    c = mix(c, vec3<f32>(l), look.desat);
    c = mix(c, l * vec3<f32>(1.07, 0.95, 0.78), look.sepia);

    // The edges of the sheet, darker at night.
    let aspect = vec2<f32>(dims.x / dims.y, 1.0);
    let v = length((uv - 0.5) * aspect);
    let edge = smoothstep(0.45, 1.05, v) * (look.vignette + look.night_vignette * look.night);
    c = mix(c, c * vec3<f32>(0.55, 0.42, 0.30), clamp(edge, 0.0, 1.0));

    return vec4<f32>(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
