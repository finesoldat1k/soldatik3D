#version 330 core

uniform sampler2D font_sampler;

in vec4 frag_color;
in vec2 frag_uv;

out vec4 out_color;

void main() {
    // Rectangles use UV (0, 0): they are solid colour and need no texture.
    if (frag_uv == vec2(0.0, 0.0)) {
        out_color = frag_color;
        return;
    }

    float alpha = texture(font_sampler, frag_uv).r;
    if (alpha < 0.1) discard;

    out_color = vec4(frag_color.rgb, frag_color.a * alpha);
}
