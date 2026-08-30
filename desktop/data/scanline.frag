#version 440
// CRT scanlines for the HUD chrome: every other physical row darkened a
// touch. Deliberately STATIC — no time uniform, so the overlay costs a
// single draw per repaint and nothing per frame.

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    float pixelHeight;
};

void main() {
    float y = qt_TexCoord0.y * pixelHeight;
    float line = step(0.5, fract(y * 0.5));
    fragColor = vec4(0.0, 0.0, 0.0, line * 0.08) * qt_Opacity;
}
