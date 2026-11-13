#version 450

layout(binding = 0) uniform sampler2D world_sampler;
layout(binding = 1) uniform sampler2D shadow_sampler;
layout(binding = 2) uniform samplerCube irradiance_map;
layout(binding = 3) uniform samplerCube prefiltered_map;

layout(binding = 4) uniform LightBuffer {
    vec4 position_radius[8];  // xyz = позиция, w = радиус
    vec4 color_intensity[8];  // rgb = цвет, a = интенсивность
} lights;

layout(location = 0) in vec3 frag_color;
layout(location = 1) in vec3 frag_normal;
layout(location = 2) in vec3 frag_pos;
layout(location = 3) in vec2 frag_uv;
layout(location = 4) in float frag_emission;

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform PushConstants {
    mat4 mvp; mat4 light_vp;
    vec4 light_pos; vec4 light_color; vec4 camera_pos; vec4 screen_and_debug;
    vec4 material;
} pc;

#define PI 3.14159265359

float D_GGX(vec3 N, vec3 H, float r) {
    float a = r * r;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float d = NdotH * NdotH * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

float G_SmithGGX(vec3 N, vec3 V, vec3 L, float r) {
    float a2 = r * r;
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float GV = NdotV / (NdotV * (1.0 - a2) + a2);
    float GL = NdotL / (NdotL * (1.0 - a2) + a2);
    return GV * GL;
}

vec3 F_Schlick(vec3 F0, float cosTheta) {
    return F0 + (1.0 - F0) * pow(1.0 - cosTheta, 5.0);
}

float calc_shadow(vec3 pos, vec3 normal) {
    vec4 lp = pc.light_vp * vec4(pos, 1.0);
    vec3 proj = lp.xyz / lp.w;
    proj.xy = proj.xy * 0.5 + 0.5;
    if (proj.z > 1.0 || proj.z < 0.0) return 1.0;
    if (proj.x < 0.0 || proj.x > 1.0 || proj.y < 0.0 || proj.y > 1.0) return 1.0;

    vec2 texel = 1.0 / vec2(textureSize(shadow_sampler, 0));
    vec3 light_dir = normalize(pc.light_pos.xyz - pos);
    float NdotL = max(dot(normal, light_dir), 0.0);
    float bias = max(0.004 * (1.0 - NdotL), 0.0008);

    float shadow = 0.0;
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            float closest = texture(shadow_sampler, proj.xy + vec2(x, y) * texel).r;
            shadow += (proj.z - bias > closest) ? 0.0 : 1.0;
        }
    }
    return shadow / 9.0;
}

// Одна точка освещения без тени (для 8 доп. ламп из UBO)
vec3 compute_extra_light(vec3 N, vec3 V, vec3 albedo, vec3 F0,
                         float metallic, float roughness,
                         vec3 light_pos, vec3 light_color, float radius) {
    vec3 delta = light_pos - frag_pos;
    float dist = length(delta);
    if (dist > radius) return vec3(0.0);

    vec3 L = delta / dist;
    float atten = pow(clamp(1.0 - dist / radius, 0.0, 1.0), 2.0);
    vec3 H = normalize(V + L);

    float NDF = D_GGX(N, H, roughness);
    float G = G_SmithGGX(N, V, L, roughness);
    vec3 F = F_Schlick(F0, max(dot(H, V), 0.0));
    vec3 spec = (NDF * G * F) / (4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001);
    vec3 kD = (vec3(1.0) - F) * (1.0 - metallic);

    float NdotL = max(dot(N, L), 0.0);
    vec3 radiance = light_color * NdotL * atten * 4.0;  // x4 — HDR вклад
    return (kD * albedo / PI + spec) * radiance;
}

void main() {
    int mode = int(pc.screen_and_debug.z);
    vec3 albedo = frag_color * texture(world_sampler, frag_uv).rgb;

    if (frag_emission > 0.0) {
        out_color = vec4(albedo * frag_emission, 1.0);
        return;
    }
    if (mode == 2) { out_color = vec4(vec3(gl_FragCoord.z), 1.0); return; }
    if (mode == 1) { out_color = vec4(normalize(frag_normal) * 0.5 + 0.5, 1.0); return; }

    vec3 N = normalize(frag_normal);
    vec3 V = normalize(pc.camera_pos.xyz - frag_pos);

    float metallic = clamp(pc.material.x, 0.0, 1.0);
    float roughness = clamp(pc.material.y, 0.04, 1.0);
    vec3 F0 = mix(vec3(0.04), albedo, metallic);

    // === ПРЯМОЙ СВЕТ (primary, с тенью) ===
    vec3 L_p = normalize(pc.light_pos.xyz - frag_pos);
    vec3 H_p = normalize(V + L_p);
    float NDF_p = D_GGX(N, H_p, roughness);
    float G_p = G_SmithGGX(N, V, L_p, roughness);
    vec3 F_p = F_Schlick(F0, max(dot(H_p, V), 0.0));
    vec3 spec_p = (NDF_p * G_p * F_p) / (4.0 * max(dot(N, V), 0.0) * max(dot(N, L_p), 0.0) + 0.0001);
    vec3 kD_p = (vec3(1.0) - F_p) * (1.0 - metallic);

    float NdotL_p = max(dot(N, L_p), 0.0);
    float shadow = calc_shadow(frag_pos, N);
    vec3 radiance_p = pc.light_color.rgb * NdotL_p * shadow * 5.0;
    vec3 direct_light = (kD_p * albedo / PI + spec_p) * radiance_p;

    // === ДОПОЛНИТЕЛЬНЫЕ 8 ЛАМП (без тени, из UBO) ===
    for (int i = 0; i < 8; i++) {
        if (lights.position_radius[i].w < 0.01) continue;
        vec3 lp = lights.position_radius[i].xyz;
        vec3 lc = lights.color_intensity[i].rgb * lights.color_intensity[i].a;
        float r = lights.position_radius[i].w;
        direct_light += compute_extra_light(N, V, albedo, F0, metallic, roughness, lp, lc, r);
    }

    // === IBL (ambient от неба) ===
    float NdotV = max(dot(N, V), 0.0);
    vec3 F_env = F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(1.0 - NdotV, 5.0);
    vec3 kD_env = (vec3(1.0) - F_env) * (1.0 - metallic);
    vec3 irradiance = texture(irradiance_map, N).rgb;
    vec3 diffuse_ibl = irradiance * albedo;
    vec3 R = reflect(-V, N);
    vec3 prefiltered = texture(prefiltered_map, R).rgb;
    vec3 specular_ibl = prefiltered * F_env;
    vec3 ambient = (kD_env * diffuse_ibl + specular_ibl) * 0.3;

    out_color = vec4(direct_light + ambient, 1.0);
}
