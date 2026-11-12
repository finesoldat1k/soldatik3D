#version 450

layout(binding = 0) uniform sampler2D hdr_input; // Исходный HDR
layout(binding = 1) uniform sampler2D bloom_input; // Размытый Bloom
layout(location = 0) in vec2 frag_uv;
layout(location = 0) out vec4 out_color;

// Сила Bloom (можно регулировать)
const float BLOOM_STRENGTH = 0.5;

void main() {
    vec3 hdr_color = texture(hdr_input, frag_uv).rgb;
    vec3 bloom_color = texture(bloom_input, frag_uv).rgb;
    
    vec3 final_color = hdr_color + bloom_color * BLOOM_STRENGTH;
    out_color = vec4(final_color, 1.0);
}
