#version 450

layout(binding = 0) uniform sampler2D ssao_input;
layout(location = 0) in vec2 frag_uv;
layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PC {
    vec2 direction;
} pc;

void main() {
    const float w[5] = { 0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216 };
    vec2 texel = 1.0 / textureSize(ssao_input, 0);
    float r = texture(ssao_input, frag_uv).r * w[0];
    for (int i = 1; i < 5; i++) {
        vec2 o = pc.direction * texel * float(i);
        r += texture(ssao_input, frag_uv + o).r * w[i];
        r += texture(ssao_input, frag_uv - o).r * w[i];
    }
    out_color = vec4(vec3(r), 1.0);
}
