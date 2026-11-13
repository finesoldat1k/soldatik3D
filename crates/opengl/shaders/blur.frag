#version 330 core

uniform sampler2D blur_input;
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
    vec2 texel_size = 1.0 / vec2(textureSize(blur_input, 0));

    vec3 result = texture(blur_input, frag_uv).rgb * gaussian_weight(0);
    for (int i = 1; i < 5; i++) {
        vec2 offset = u_direction * texel_size * float(i) * 3.0;
        float weight = gaussian_weight(i);
        result += texture(blur_input, frag_uv + offset).rgb * weight;
        result += texture(blur_input, frag_uv - offset).rgb * weight;
    }
    out_color = vec4(result, 1.0);
}
