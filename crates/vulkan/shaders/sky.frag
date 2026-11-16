// Sky fragment shader - atmospheric scattering with procedural clouds and sun
#version 450

layout(location = 0) in vec2 frag_uv;

layout(location = 0) out vec4 out_color;

layout(push_constant) uniform SkyPushConstants {
    mat4 inv_proj;          // Inverse projection matrix
    mat4 inv_view;          // Inverse view matrix (camera to world)
    vec4 camera_pos;        // Camera position in world space
    vec4 light_dir;         // xyz = sun direction (normalized, toward surface), w = sun angular diameter
    vec4 light_color;       // rgb = sun color * intensity, a = unused
    vec4 sky_params;        // x = time, y = cloud speed, z = cloud scale, w = unused
    vec4 atmosphere;        // x = rayleigh scale, y = mie scale, z = exposure, w = white point
} pc;

#define PI 3.14159265359
#define TAU 6.28318530718

// Rayleigh scattering coefficients (scaled up for visibility)
vec3 rayleigh_coeff() {
    // Approximate for 680nm (red), 550nm (green), 440nm (blue) - scaled up 10000x
    return vec3(0.058, 0.135, 0.331);
}

// Mie scattering coefficient (scaled up)
const float mie_coeff = 0.02;

// Scale height for density falloff
const float rayleigh_height = 8000.0;
const float mie_height = 1200.0;
const float planet_radius = 6360000.0;
const float atmosphere_height = 100000.0;

// Hash function for noise
uint hash(uint x) {
    x ^= x >> 16;
    x *= 0x7feb352d;
    x ^= x >> 15;
    x *= 0x846ca68b;
    x ^= x >> 16;
    return x;
}

uint hash2(uvec2 p) {
    return hash(p.x ^ (p.y * 0x9e3779b9));
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f); // smoothstep
    
    float a = float(hash2(uvec2(i)));
    float b = float(hash2(uvec2(i + vec2(1.0, 0.0))));
    float c = float(hash2(uvec2(i + vec2(0.0, 1.0))));
    float d = float(hash2(uvec2(i + vec2(1.0, 1.0))));
    
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y) / 4294967295.0;
}

float fbm(vec2 p, int octaves) {
    float value = 0.0;
    float amplitude = 0.5;
    float frequency = 1.0;
    for (int i = 0; i < octaves; i++) {
        value += amplitude * noise(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// Phase functions
float rayleigh_phase(float cos_theta) {
    return 0.75 * (1.0 + cos_theta * cos_theta);
}

float mie_phase(float cos_theta, float g) {
    float g2 = g * g;
    float denom = pow(1.0 + g2 - 2.0 * g * cos_theta, 1.5);
    return (1.0 - g2) / (4.0 * PI * denom);
}

// Transmittance (optical depth)
float transmittance(float optical_depth) {
    return exp(-optical_depth);
}

// Optical depth for a ray from origin to point at distance s
float optical_depth(vec3 origin, vec3 dir, float s, float height_scale) {
    // Simplified: exponential atmosphere
    // For more accuracy, use numerical integration
    float h0 = length(origin) - planet_radius;
    float h1 = length(origin + dir * s) - planet_radius;
    return height_scale * (exp(-h0 / height_scale) - exp(-h1 / height_scale)) * (height_scale / max(s, 0.001));
}

// Compute sun intensity with atmospheric extinction
vec3 compute_sun_color(vec3 sun_dir, vec3 view_dir, vec3 camera_pos, float sun_angular_diameter) {
    // Distance from camera to sun (effectively infinite, but we compute atmospheric path)
    float cos_sun_view = dot(sun_dir, view_dir);
    
    // Sun disk with soft edge
    float sun_angular_radius = sun_angular_diameter * 0.5;
    float sun_edge = smoothstep(cos(sun_angular_radius), cos(sun_angular_radius + 0.001), cos_sun_view);
    
    // Atmospheric extinction - sun dims near horizon
    float extinction = pow(max(sun_dir.y, 0.0), 0.5);
    
    vec3 sun_color = pc.light_color.rgb * sun_edge * extinction * 50000.0;
    
    // Add corona/glow
    float glow = pow(max(cos_sun_view, 0.0), 100.0) * 1000.0;
    sun_color += pc.light_color.rgb * glow;
    
    return sun_color;
}

// Procedural clouds
vec3 compute_clouds(vec3 view_dir, float time) {
    // Cloud layer at ~2-3km altitude
    float cloud_height = 3000.0;
    
    // Project view ray to cloud plane
    float t = (cloud_height - pc.camera_pos.y) / view_dir.y;
    if (t <= 0.0) return vec3(0.0);
    
    vec3 cloud_pos = pc.camera_pos.xyz + view_dir * t;
    
    // Cloud noise coordinates
    vec2 cloud_uv = cloud_pos.xz * pc.sky_params.z * 0.001 + vec2(time * pc.sky_params.y, 0.0);
    
    // Multi-layer cloud noise
    float cloud_noise = 0.0;
    cloud_noise += fbm(cloud_uv, 4) * 0.6;
    cloud_noise += fbm(cloud_uv * 2.0 + vec2(time * 0.1, 0.0), 3) * 0.3;
    cloud_noise += fbm(cloud_uv * 4.0 + vec2(time * 0.05, 100.0), 2) * 0.1;
    
    // Cloud shape with sharp edges
    float cloud_alpha = smoothstep(0.4, 0.55, cloud_noise);
    
    // Cloud color - white with slight tint from sun
    float sun_dot = max(dot(normalize(pc.light_dir.xyz), view_dir), 0.0);
    vec3 cloud_color = vec3(1.0, 0.98, 0.95) * (0.7 + sun_dot * 0.3);
    
    // Shadow on clouds
    float cloud_shadow = 1.0 - smoothstep(0.3, 0.5, cloud_noise);
    
    return cloud_color * cloud_alpha * cloud_shadow;
}

// Atmospheric scattering (simplified single scattering)
// Always returns a sky color - never black
vec3 compute_atmosphere(vec3 view_dir, vec3 sun_dir, vec3 camera_pos) {
    // Distance to atmosphere top
    float atmosphere_top = 100000.0;
    float t = (atmosphere_top - camera_pos.y) / max(view_dir.y, 0.001);
    if (t <= 0.0) t = 100000.0;
    
    float cos_theta = max(dot(view_dir, sun_dir), 0.0);
    float zenith = max(view_dir.y, 0.0);
    
    // Rayleigh scattering - strong blue sky
    vec3 rayleigh = rayleigh_coeff() * rayleigh_phase(cos_theta);
    float rayleigh_od = optical_depth(camera_pos, view_dir, t, rayleigh_height);
    vec3 rayleigh_color = rayleigh * (1.0 - exp(-rayleigh_od * pc.atmosphere.x)) * pc.light_color.rgb * 50.0;
    
    // Mie scattering (haze) - stronger
    float mie_phase_val = mie_phase(cos_theta, 0.8);
    float mie_od = optical_depth(camera_pos, view_dir, t, mie_height);
    vec3 mie_color = vec3(mie_coeff) * mie_phase_val * (1.0 - exp(-mie_od * pc.atmosphere.y)) * pc.light_color.rgb * 20.0;
    
    // Base sky color - always visible gradient
    vec3 zenith_color = vec3(0.15, 0.35, 0.75);
    vec3 horizon_color = vec3(0.65, 0.75, 0.95);
    vec3 base_sky = mix(horizon_color, zenith_color, pow(zenith, 0.6));
    
    // Combine with atmospheric scattering
    vec3 sky_color = base_sky + (rayleigh_color + mie_color) * pc.atmosphere.z * 10.0;
    
    // Add sun glow on horizon
    float sun_horizon = max(sun_dir.y, 0.0);
    sky_color += horizon_color * sun_horizon * 0.5;
    
    return sky_color;
}

void main() {
    // Reconstruct view direction in world space
    vec4 clip_pos = vec4(frag_uv * 2.0 - 1.0, 1.0, 1.0);
    vec4 view_pos = pc.inv_proj * clip_pos;
    view_pos /= view_pos.w;
    vec3 view_dir = normalize((pc.inv_view * vec4(view_pos.xyz, 0.0)).xyz);
    
    vec3 sun_dir = normalize(pc.light_dir.xyz);
    
    // Compute sky components
    vec3 color = vec3(0.0);
    
    // 1. Atmospheric scattering
    color += compute_atmosphere(view_dir, sun_dir, pc.camera_pos.xyz);
    
    // 2. Sun disk
    color += compute_sun_color(sun_dir, view_dir, pc.camera_pos.xyz, pc.light_dir.w);
    
    // 3. Clouds
    color += compute_clouds(view_dir, pc.sky_params.x);
    
    // Exposure and tonemapping
    color = color / (color + pc.atmosphere.w); // Reinhard
    color = pow(color, vec3(1.0 / 2.2)); // Gamma correction
    
    out_color = vec4(color, 1.0);
}