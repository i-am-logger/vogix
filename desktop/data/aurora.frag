#version 440
// The built-in "aurora" live background: slow flow-noise curtains blending
// the theme's background ramp with the accent slots — palette arrives as
// uniforms, so one shader serves every theme and both polarities. Written
// from first principles (value-noise fbm + domain warp); no third-party
// shader code.

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    float time;
    vec4 base00;
    vec4 base01;
    vec4 accentA; // `active`
    vec4 accentB; // `link`
};

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
        mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
        u.y);
}

float fbm(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 4; i++) {
        v += a * vnoise(p);
        p = p * 2.03 + vec2(17.0, 9.0);
        a *= 0.5;
    }
    return v;
}

void main() {
    vec2 uv = qt_TexCoord0;
    float t = time * 0.02;

    // Domain-warped curtains, drifting slowly upward.
    vec2 p = vec2(uv.x * 3.0, uv.y * 2.0 - t);
    float warp = fbm(p + vec2(t * 0.7, 0.0));
    float band = fbm(p * 1.7 + vec2(warp * 1.5, -t));

    // The vertical falloff keeps the aurora in the upper half.
    float falloff = smoothstep(1.0, 0.15, uv.y);
    float curtain = smoothstep(0.35, 0.85, band) * falloff;

    vec3 ground = mix(base00.rgb, base01.rgb, uv.y * 0.6 + warp * 0.1);
    vec3 glow = mix(accentA.rgb, accentB.rgb, vnoise(p * 0.7 + t));
    vec3 color = ground + glow * curtain * 0.35;

    fragColor = vec4(color, 1.0) * qt_Opacity;
}
