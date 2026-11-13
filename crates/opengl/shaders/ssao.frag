#version 330 core

uniform sampler2D depth_input;
uniform mat4 u_proj;
uniform mat4 u_inv_proj;

in vec2 frag_uv;
out vec4 out_color;

const float SSAO_RADIUS = 0.7;
const float SSAO_BIAS = 0.025;
const int SSAO_SAMPLES = 16;

vec3 view_from_depth(vec2 uv, float d) {
    vec4 clip = vec4(uv * 2.0 - 1.0, d * 2.0 - 1.0, 1.0);
    vec4 v = u_inv_proj * clip;
    return v.xyz / max(v.w, 1e-6);
}

float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

void main() {
    float d = texture(depth_input, frag_uv).r;
    if (d >= 0.9999) { out_color = vec4(1.0); return; }

    vec3 pos = view_from_depth(frag_uv, d);
    vec2 texel = 1.0 / vec2(textureSize(depth_input, 0));

    // Reconstruct the normal from neighbouring view-space positions.
    vec2 uv_r = frag_uv + vec2(texel.x, 0.0);
    vec2 uv_u = frag_uv + vec2(0.0, texel.y);
    vec3 pos_r = view_from_depth(uv_r, texture(depth_input, uv_r).r);
    vec3 pos_u = view_from_depth(uv_u, texture(depth_input, uv_u).r);
    vec3 n = cross(pos_r - pos, pos_u - pos);
    float n_len = length(n);
    if (n_len < 1e-5) { out_color = vec4(1.0); return; }
    n /= n_len;
    if (dot(n, -pos) < 0.0) n = -n;

    vec3 up = abs(n.z) < 0.99 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0);
    vec3 t = normalize(cross(up, n));
    vec3 b = cross(n, t);

    float rand = hash12(frag_uv * vec2(textureSize(depth_input, 0)));

    float occ = 0.0;
    for (int i = 0; i < SSAO_SAMPLES; i++) {
        float fi = float(i);
        float r = sqrt((fi + 1.0) / float(SSAO_SAMPLES));
        float angle = fi * 2.399963 + rand * 6.2831853;
        vec3 local = vec3(cos(angle) * r, sin(angle) * r, sqrt(max(0.0, 1.0 - r * r)));
        vec3 sample_pos = pos + (t * local.x + b * local.y + n * local.z) * SSAO_RADIUS;

        vec4 clip = u_proj * vec4(sample_pos, 1.0);
        if (clip.w <= 0.0) continue;
        vec3 ndc = clip.xyz / clip.w;
        vec2 sample_uv = ndc.xy * 0.5 + 0.5;
        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) continue;

        float scene_d = texture(depth_input, sample_uv).r;
        if (scene_d >= 0.9999) continue;
        vec3 scene_pos = view_from_depth(sample_uv, scene_d);

        if (scene_pos.z > sample_pos.z + SSAO_BIAS) {
            float range = smoothstep(0.0, 1.0, SSAO_RADIUS / max(abs(pos.z - scene_pos.z), 1e-4));
            occ += range;
        }
    }
    float ao = 1.0 - occ / float(SSAO_SAMPLES);
    out_color = vec4(ao, ao, ao, 1.0);
}
