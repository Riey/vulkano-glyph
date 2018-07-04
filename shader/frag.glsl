#version 450

layout(set = 0, binding = 1) uniform sampler2D font_tex;

layout(location = 0) in vec2 f_tex_pos;
layout(location = 1) in vec4 f_color;

layout(location = 0) out vec4 Target0;

void main() {
    float alpha = /*texture(font_tex, f_tex_pos).r*/1.0;
    if (alpha <= 0.0) {
        discard;
    }
    Target0 = f_color * vec4(1.0, 1.0, 1.0, alpha);
}
