#version 330 core

uniform sampler2D world_sampler;
uniform sampler2D shadow_sampler;
uniform samplerCube irradiance_map;
uniform samplerCube prefiltered_map;

layout(std140) uniform LightBuffer {
    vec4 position_radius[8];
    vec4 color_intensity[8];
} lights;

uniform mat4 u_light_vp;
uniform vec4 u_light_dir_he;   // xyz = light direction (toward surface), w = ortho_half_extent
uniform vec4 u_light_color;    // rgb = color * intensity, a = sun_angular_diameter
uniform vec4 u_camera_pos;
uniform vec4 u_screen_and_debug;
uniform vec4 u_material;

in vec3 frag_color;
in vec3 frag_normal;
in vec3 frag_pos;
in vec2 frag_uv;
in float frag_emission;
in vec2 frag_material;

out vec4 out_color;

#define PI 3.14159265359
#define INV_PI 0.31830988618
// PREFILTERED_MIP_LEVELS - 1 in crates/assets/src/environment.rs.
#define PREFILTERED_MAX_LOD 7.0

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

// Karis' split-sum environment BRDF approximation.
vec2 env_brdf(float roughness, float NdotV) {
    const vec4 c0 = vec4(-1.0, -0.0275, -0.572, 0.022);
    const vec4 c1 = vec4(1.0, 0.0425, 1.04, -0.04);
    vec4 r = roughness * c0 + c1;
    float a004 = min(r.x * r.x, exp2(-9.28 * NdotV)) * r.x + r.y;
    return vec2(-1.04, 1.04) * a004 + r.zw;
}

// Normal-offset shadows: slide the lookup along the surface normal by roughly
// one shadow texel at grazing angles. This removes acne on floors and walls
// without a large constant depth bias that would detach contact shadows.
float calc_shadow(vec3 pos, vec3 normal, vec3 light_dir) {
    float NdotL = max(dot(normal, light_dir), 0.0);
    float shadow_size = float(textureSize(shadow_sampler, 0).x);
    float texel_world = 2.0 * max(u_light_dir_he.w, 1.0) / shadow_size;
    float slope = sqrt(max(1.0 - NdotL * NdotL, 0.0)) / max(NdotL, 0.15);
    vec3 sample_pos = pos + normal * min(slope, 4.0) * texel_world;

    vec4 lp = u_light_vp * vec4(sample_pos, 1.0);
    vec3 proj = lp.xyz / lp.w;
    proj.xy = proj.xy * 0.5 + 0.5;
    if (proj.z > 1.0 || proj.z < 0.0) return 1.0;
    if (proj.x < 0.0 || proj.x > 1.0 || proj.y < 0.0 || proj.y > 1.0) return 1.0;

    vec2 texel = 1.0 / vec2(textureSize(shadow_sampler, 0));
    float bias = max(0.003 * (1.0 - NdotL), 0.001);

    float shadow = 0.0;
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            float closest = texture(shadow_sampler, proj.xy + vec2(x, y) * texel).r;
            shadow += (proj.z - bias > closest) ? 0.0 : 1.0;
        }
    }
    return shadow / 9.0;
}

// Directional light (sun) with PCF shadow
vec3 compute_directional_light(vec3 N, vec3 V, vec3 albedo, vec3 F0,
                               float metallic, float roughness,
                               vec3 light_dir, vec3 light_color,
                               float sun_angular_diameter) {
    vec3 L = light_dir;
    float NdotL = max(dot(N, L), 0.0);
    if (NdotL <= 0.0) return vec3(0.0);

    vec3 H = normalize(V + L);
    float NDF = D_GGX(N, H, roughness);
    float G = G_SmithGGX(N, V, L, roughness);
    vec3 F = F_Schlick(F0, max(dot(H, V), 0.0));
    vec3 spec = (NDF * G * F) / (4.0 * max(dot(N, V), 0.0) * NdotL + 0.0001);
    vec3 kD = (vec3(1.0) - F) * (1.0 - metallic);

    float shadow = calc_shadow(frag_pos, N, L);

    vec3 radiance = light_color * NdotL * shadow;
    return (kD * albedo * INV_PI + spec) * radiance;
}

// Point/spot light (from UBO) without shadow
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
    vec3 radiance = light_color * NdotL * atten * 4.0;
    return (kD * albedo * INV_PI + spec) * radiance;
}

void main() {
    int mode = int(u_screen_and_debug.z);
    vec3 albedo = frag_color * texture(world_sampler, frag_uv).rgb;

    if (frag_emission > 0.0) {
        out_color = vec4(albedo * frag_emission, 1.0);
        return;
    }
    if (mode == 2) { out_color = vec4(vec3(gl_FragCoord.z), 1.0); return; }
    if (mode == 1) { out_color = vec4(normalize(frag_normal) * 0.5 + 0.5, 1.0); return; }

    vec3 N = normalize(frag_normal);
    vec3 V = normalize(u_camera_pos.xyz - frag_pos);

    // Per-vertex overrides keep polished metal, rough concrete and painted
    // surfaces in one draw call. Negative components fall back to the scene.
    float metallic = clamp(frag_material.x < 0.0 ? u_material.x : frag_material.x, 0.0, 1.0);
    float roughness = clamp(frag_material.y < 0.0 ? u_material.y : frag_material.y, 0.045, 1.0);
    vec3 F0 = mix(vec3(0.04), albedo, metallic);

    // === DIRECTIONAL LIGHT (PrimaryLight / Sun) ===
    // u_light_dir_he.xyz = light direction (normalized, pointing toward surface)
    // u_light_dir_he.w = ortho_half_extent for shadow
    // u_light_color.rgb = light color * intensity
    // u_light_color.a = sun angular diameter (radians)
    vec3 light_dir = normalize(u_light_dir_he.xyz);
    vec3 light_color = u_light_color.rgb;
    float sun_angular_diameter = u_light_color.a;

    vec3 direct_light = compute_directional_light(N, V, albedo, F0, metallic, roughness,
                                                  light_dir, light_color, sun_angular_diameter);

    // === ADDITIONAL 8 LOCAL LIGHTS (point lights, no shadow) ===
    for (int i = 0; i < 8; i++) {
        if (lights.position_radius[i].w < 0.01) continue;
        vec3 lp = lights.position_radius[i].xyz;
        vec3 lc = lights.color_intensity[i].rgb * lights.color_intensity[i].a;
        float r = lights.position_radius[i].w;
        direct_light += compute_extra_light(N, V, albedo, F0, metallic, roughness, lp, lc, r);
    }

    // === IBL (ambient from sky cubemap only - NO SUN) ===
    float NdotV = max(dot(N, V), 0.0);
    vec3 F_env = F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(1.0 - NdotV, 5.0);
    vec3 kD_env = (vec3(1.0) - F_env) * (1.0 - metallic);
    vec3 irradiance = texture(irradiance_map, N).rgb;
    vec3 diffuse_ibl = kD_env * albedo * INV_PI * irradiance;
    vec3 R = reflect(-V, N);
    vec3 prefiltered = textureLod(prefiltered_map, R, roughness * PREFILTERED_MAX_LOD).rgb;
    vec2 brdf = env_brdf(roughness, NdotV);
    vec3 specular_ibl = prefiltered * (F0 * brdf.x + brdf.y);
    vec3 ambient = diffuse_ibl + specular_ibl;

    out_color = vec4(direct_light + ambient, 1.0);
}