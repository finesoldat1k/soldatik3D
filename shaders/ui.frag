#version 450

layout(binding = 0) uniform sampler2D font_sampler;

layout(location = 0) in vec4 frag_color;
layout(location = 1) in vec2 frag_uv;

layout(location = 0) out vec4 out_color;

void main() {
    // Если UV == (0,0), это просто цветной прямоугольник (панель), текстуру не читаем
    if (frag_uv == vec2(0.0, 0.0)) {
        out_color = frag_color;
        return;
    }

    float alpha = texture(font_sampler, frag_uv).r;
    if (alpha < 0.1) discard;

    out_color = vec4(frag_color.rgb, frag_color.a * alpha);
}
