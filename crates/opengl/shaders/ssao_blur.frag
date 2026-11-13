#version 330 core

uniform sampler2D ssao_input;
uniform vec2 u_direction;

in vec2 frag_uv;
out vec4 out_color;

float gaussian_weight(int index) {
    if (index == 0) return 0.227027;
    if (index == 1) return 0.1945946;
    if (index == 2) return 0.1216216;
    if (index == 3) return 0.054054;
    return 0.016216;
}

void main() {
    vec2 texel = 1.0 / vec2(textureSize(ssao_input, 0));
    float r = texture(ssao_input, frag_uv).r * gaussian_weight(0);
    for (int i = 1; i < 5; i++) {
        vec2 o = u_direction * texel * float(i);
        float weight = gaussian_weight(i);
        r += texture(ssao_input, frag_uv + o).r * weight;
        r += texture(ssao_input, frag_uv - o).r * weight;
    }
    out_color = vec4(vec3(r), 1.0);
}
