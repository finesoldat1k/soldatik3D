#version 450

layout(binding = 0) uniform sampler2D blur_input;
layout(location = 0) in vec2 frag_uv;
layout(location = 0) out vec4 out_color;

layout(push_constant) uniform BlurParams {
    vec2 direction;
} params;

void main() {
    const float weights[5] = { 0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216 };
    vec2 texel_size = 1.0 / textureSize(blur_input, 0);

    vec3 result = texture(blur_input, frag_uv).rgb * weights[0];
    for (int i = 1; i < 5; i++) {
        vec2 offset = params.direction * texel_size * float(i) * 3.0;
        result += texture(blur_input, frag_uv + offset).rgb * weights[i];
        result += texture(blur_input, frag_uv - offset).rgb * weights[i];
    }
    out_color = vec4(result, 1.0);
}
