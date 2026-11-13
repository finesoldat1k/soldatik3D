#version 330 core

uniform sampler2D hdr_input;
uniform sampler2D bloom_input;
uniform sampler2D ssao_input;

in vec2 frag_uv;
out vec4 out_color;

const float BLOOM_STRENGTH = 1.0;
const float SSAO_STRENGTH = 1.0;

vec3 ACESFilm(vec3 x) {
    float a = 2.51; float b = 0.03;
    float c = 2.43; float d = 0.59; float e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

void main() {
    vec3 hdr = texture(hdr_input, frag_uv).rgb;
    vec3 bloom = texture(bloom_input, frag_uv).rgb;
    float ao = texture(ssao_input, frag_uv).r;

    float ssao_factor = mix(1.0, ao, SSAO_STRENGTH);
    vec3 shaded = hdr * ssao_factor;
    vec3 with_bloom = shaded + bloom * BLOOM_STRENGTH;
    vec3 ldr = ACESFilm(with_bloom);
    ldr = pow(ldr, vec3(1.0 / 2.2));
    out_color = vec4(ldr, 1.0);
}
