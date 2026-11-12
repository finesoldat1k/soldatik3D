use crate::vulkan::buffer::Vertex;
use glam::Vec3;

pub struct Collider {
    pub min: Vec3,
    pub max: Vec3,
}

fn add_floor(v: &mut Vec<Vertex>, colliders: &mut Vec<Collider>, x1: f32, x2: f32, z1: f32, z2: f32, y: f32, color: [f32; 3]) {
    let uv_x = (x2 - x1) * 0.5; 
    let uv_z = (z2 - z1) * 0.5;
    v.push(Vertex { pos: [x1, y, z1], color, normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], emission: 0.0 });
    v.push(Vertex { pos: [x2, y, z1], color, normal: [0.0, 1.0, 0.0], uv: [uv_x, 0.0], emission: 0.0 });
    v.push(Vertex { pos: [x2, y, z2], color, normal: [0.0, 1.0, 0.0], uv: [uv_x, uv_z], emission: 0.0 });
    v.push(Vertex { pos: [x1, y, z1], color, normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], emission: 0.0 });
    v.push(Vertex { pos: [x2, y, z2], color, normal: [0.0, 1.0, 0.0], uv: [uv_x, uv_z], emission: 0.0 });
    v.push(Vertex { pos: [x1, y, z2], color, normal: [0.0, 1.0, 0.0], uv: [0.0, uv_z], emission: 0.0 });
    colliders.push(Collider { min: Vec3::new(x1, y - 1.0, z1), max: Vec3::new(x2, y, z2) });
}

fn add_ceiling(v: &mut Vec<Vertex>, colliders: &mut Vec<Collider>, x1: f32, x2: f32, z1: f32, z2: f32, y: f32, color: [f32; 3]) {
    let uv_x = (x2 - x1) * 0.5; 
    let uv_z = (z2 - z1) * 0.5;
    v.push(Vertex { pos: [x1, y, z1], color, normal: [0.0, -1.0, 0.0], uv: [0.0, 0.0], emission: 0.0 });
    v.push(Vertex { pos: [x1, y, z2], color, normal: [0.0, -1.0, 0.0], uv: [0.0, uv_z], emission: 0.0 });
    v.push(Vertex { pos: [x2, y, z2], color, normal: [0.0, -1.0, 0.0], uv: [uv_x, uv_z], emission: 0.0 });
    v.push(Vertex { pos: [x1, y, z1], color, normal: [0.0, -1.0, 0.0], uv: [0.0, 0.0], emission: 0.0 });
    v.push(Vertex { pos: [x2, y, z2], color, normal: [0.0, -1.0, 0.0], uv: [uv_x, uv_z], emission: 0.0 });
    v.push(Vertex { pos: [x2, y, z1], color, normal: [0.0, -1.0, 0.0], uv: [uv_x, 0.0], emission: 0.0 });
    colliders.push(Collider { min: Vec3::new(x1, y, z1), max: Vec3::new(x2, y + 1.0, z2) });
}

fn add_wall_x(v: &mut Vec<Vertex>, colliders: &mut Vec<Collider>, x: f32, z1: f32, z2: f32, y1: f32, y2: f32, normal_dir: f32, color: [f32; 3]) {
    let nx = normal_dir; 
    let uv_y = (y2 - y1) * 0.5; 
    let uv_z = (z2 - z1) * 0.5;
    if nx > 0.0 { 
        v.push(Vertex { pos: [x, y1, z1], color, normal: [nx, 0.0, 0.0], uv: [0.0, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x, y1, z2], color, normal: [nx, 0.0, 0.0], uv: [uv_z, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x, y2, z2], color, normal: [nx, 0.0, 0.0], uv: [uv_z, 0.0], emission: 0.0 });
        v.push(Vertex { pos: [x, y1, z1], color, normal: [nx, 0.0, 0.0], uv: [0.0, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x, y2, z2], color, normal: [nx, 0.0, 0.0], uv: [uv_z, 0.0], emission: 0.0 });
        v.push(Vertex { pos: [x, y2, z1], color, normal: [nx, 0.0, 0.0], uv: [0.0, 0.0], emission: 0.0 });
    } else { 
        v.push(Vertex { pos: [x, y1, z1], color, normal: [nx, 0.0, 0.0], uv: [0.0, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x, y2, z2], color, normal: [nx, 0.0, 0.0], uv: [uv_z, 0.0], emission: 0.0 });
        v.push(Vertex { pos: [x, y1, z2], color, normal: [nx, 0.0, 0.0], uv: [uv_z, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x, y1, z1], color, normal: [nx, 0.0, 0.0], uv: [0.0, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x, y2, z1], color, normal: [nx, 0.0, 0.0], uv: [0.0, 0.0], emission: 0.0 });
        v.push(Vertex { pos: [x, y2, z2], color, normal: [nx, 0.0, 0.0], uv: [uv_z, 0.0], emission: 0.0 });
    }
    colliders.push(Collider { min: Vec3::new(x - 0.5, y1, z1), max: Vec3::new(x + 0.5, y2, z2) });
}

fn add_wall_z(v: &mut Vec<Vertex>, colliders: &mut Vec<Collider>, z: f32, x1: f32, x2: f32, y1: f32, y2: f32, normal_dir: f32, color: [f32; 3]) {
    let nz = normal_dir; 
    let uv_y = (y2 - y1) * 0.5; 
    let uv_x = (x2 - x1) * 0.5;
    if nz > 0.0 { 
        v.push(Vertex { pos: [x1, y1, z], color, normal: [0.0, 0.0, nz], uv: [0.0, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x2, y2, z], color, normal: [0.0, 0.0, nz], uv: [uv_x, 0.0], emission: 0.0 });
        v.push(Vertex { pos: [x2, y1, z], color, normal: [0.0, 0.0, nz], uv: [uv_x, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x1, y1, z], color, normal: [0.0, 0.0, nz], uv: [0.0, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x1, y2, z], color, normal: [0.0, 0.0, nz], uv: [0.0, 0.0], emission: 0.0 });
        v.push(Vertex { pos: [x2, y2, z], color, normal: [0.0, 0.0, nz], uv: [uv_x, 0.0], emission: 0.0 });
    } else { 
        v.push(Vertex { pos: [x1, y1, z], color, normal: [0.0, 0.0, nz], uv: [0.0, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x2, y1, z], color, normal: [0.0, 0.0, nz], uv: [uv_x, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x2, y2, z], color, normal: [0.0, 0.0, nz], uv: [uv_x, 0.0], emission: 0.0 });
        v.push(Vertex { pos: [x1, y1, z], color, normal: [0.0, 0.0, nz], uv: [0.0, uv_y], emission: 0.0 });
        v.push(Vertex { pos: [x2, y2, z], color, normal: [0.0, 0.0, nz], uv: [uv_x, 0.0], emission: 0.0 });
        v.push(Vertex { pos: [x1, y2, z], color, normal: [0.0, 0.0, nz], uv: [0.0, 0.0], emission: 0.0 });
    }
    colliders.push(Collider { min: Vec3::new(x1, y1, z - 0.5), max: Vec3::new(x2, y2, z + 0.5) });
}

fn add_light_fixture(v: &mut Vec<Vertex>, x: f32, y: f32, z: f32) {
    let h = Vec3::new(1.0, 0.05, 1.0);
    let p = Vec3::new(x, y, z);
    let color = [1.0, 1.0, 1.0]; 
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let n_bot = [0.0, -1.0, 0.0];
    v.push(Vertex { pos: [p.x-h.x, p.y-h.y, p.z-h.z], color, normal: n_bot, uv: uv[0], emission: 15.0 });
    v.push(Vertex { pos: [p.x+h.x, p.y-h.y, p.z+h.z], color, normal: n_bot, uv: uv[2], emission: 15.0 });
    v.push(Vertex { pos: [p.x+h.x, p.y-h.y, p.z-h.z], color, normal: n_bot, uv: uv[1], emission: 15.0 });
    v.push(Vertex { pos: [p.x-h.x, p.y-h.y, p.z-h.z], color, normal: n_bot, uv: uv[0], emission: 15.0 });
    v.push(Vertex { pos: [p.x-h.x, p.y-h.y, p.z+h.z], color, normal: n_bot, uv: uv[3], emission: 15.0 });
    v.push(Vertex { pos: [p.x+h.x, p.y-h.y, p.z+h.z], color, normal: n_bot, uv: uv[2], emission: 15.0 });
}

fn add_cube(v: &mut Vec<Vertex>, colliders: &mut Vec<Collider>, center: Vec3, size: Vec3, color: [f32; 3]) {
    let h = size * 0.5; 
    let p = center;
    let tile = 1.0;
    let uv = [[0.0, 0.0], [tile, 0.0], [tile, tile], [0.0, tile]];
    let faces = [
        ([p.x-h.x, p.y-h.y, p.z+h.z], [p.x+h.x, p.y-h.y, p.z+h.z], [p.x+h.x, p.y+h.y, p.z+h.z], [p.x-h.x, p.y+h.y, p.z+h.z], [0.0, 0.0, 1.0]),
        ([p.x+h.x, p.y-h.y, p.z-h.z], [p.x-h.x, p.y-h.y, p.z-h.z], [p.x-h.x, p.y+h.y, p.z-h.z], [p.x+h.x, p.y+h.y, p.z-h.z], [0.0, 0.0, -1.0]),
        ([p.x-h.x, p.y+h.y, p.z+h.z], [p.x+h.x, p.y+h.y, p.z+h.z], [p.x+h.x, p.y+h.y, p.z-h.z], [p.x-h.x, p.y+h.y, p.z-h.z], [0.0, 1.0, 0.0]),
        ([p.x-h.x, p.y-h.y, p.z-h.z], [p.x+h.x, p.y-h.y, p.z-h.z], [p.x+h.x, p.y-h.y, p.z+h.z], [p.x-h.x, p.y-h.y, p.z+h.z], [0.0, -1.0, 0.0]),
        ([p.x+h.x, p.y-h.y, p.z+h.z], [p.x+h.x, p.y-h.y, p.z-h.z], [p.x+h.x, p.y+h.y, p.z-h.z], [p.x+h.x, p.y+h.y, p.z+h.z], [1.0, 0.0, 0.0]),
        ([p.x-h.x, p.y-h.y, p.z-h.z], [p.x-h.x, p.y-h.y, p.z+h.z], [p.x-h.x, p.y+h.y, p.z+h.z], [p.x-h.x, p.y+h.y, p.z-h.z], [-1.0, 0.0, 0.0]),
    ];
    for (v0, v1, v2, v3, normal) in faces {
        v.push(Vertex { pos: v0, color, normal, uv: uv[0], emission: 0.0 });
        v.push(Vertex { pos: v1, color, normal, uv: uv[1], emission: 0.0 });
        v.push(Vertex { pos: v2, color, normal, uv: uv[2], emission: 0.0 });
        v.push(Vertex { pos: v0, color, normal, uv: uv[0], emission: 0.0 });
        v.push(Vertex { pos: v2, color, normal, uv: uv[2], emission: 0.0 });
        v.push(Vertex { pos: v3, color, normal, uv: uv[3], emission: 0.0 });
    }
    colliders.push(Collider { min: p - h, max: p + h });
}

pub fn create_room_vertices() -> (Vec<(Vec<Vertex>, Collider)>, Vec<Collider>) {
    let mut chunks = Vec::new();
    let mut all_colliders = Vec::new();
    
    let c_floor = [0.5, 0.5, 0.5];
    let c_ceil = [0.9, 0.9, 0.9];
    let c_wall = [0.95, 0.95, 0.9];
    let c_pool = [0.3, 0.5, 0.6];
    let c_pillar = [0.7, 0.7, 0.7];

    // ============ ЧАНК 1: СТАРТОВАЯ КОМНАТА ============
    {
        let mut v = Vec::new();
        let mut col = Vec::new();
        add_floor(&mut v, &mut col, -10.0, 10.0, -10.0, 10.0, -10.0, c_floor);
        add_ceiling(&mut v, &mut col, -10.0, 10.0, -10.0, 10.0, 0.0, c_ceil);
        add_wall_x(&mut v, &mut col, -10.0, -10.0, 10.0, -10.0, 0.0, 1.0, c_wall);
        add_wall_x(&mut v, &mut col, 10.0, -10.0, 10.0, -10.0, 0.0, -1.0, c_wall);
        add_wall_z(&mut v, &mut col, -10.0, -10.0, 10.0, -10.0, 0.0, 1.0, c_wall);
        add_wall_z(&mut v, &mut col, 10.0, -10.0, -3.0, -10.0, 0.0, -1.0, c_wall);
        add_wall_z(&mut v, &mut col, 10.0, 3.0, 10.0, -10.0, 0.0, -1.0, c_wall);
        add_wall_z(&mut v, &mut col, 10.0, -3.0, 3.0, -4.0, 0.0, -1.0, c_wall);
        add_light_fixture(&mut v, 0.0, -0.1, 0.0);
        add_light_fixture(&mut v, -5.0, -0.1, -5.0);
        add_light_fixture(&mut v, 5.0, -0.1, 5.0);
        all_colliders.extend(col);
        chunks.push((v, Collider { min: Vec3::new(-10.0, -10.0, -10.0), max: Vec3::new(10.0, 0.0, 10.0) }));
    }

    // ============ ЧАНК 2: КОРИДОР 1 ============
    {
        let mut v = Vec::new();
        let mut col = Vec::new();
        add_floor(&mut v, &mut col, -3.0, 3.0, 10.0, 40.0, -10.0, c_floor);
        add_ceiling(&mut v, &mut col, -3.0, 3.0, 10.0, 40.0, -4.0, c_ceil);
        add_wall_x(&mut v, &mut col, -3.0, 10.0, 40.0, -10.0, -4.0, 1.0, c_wall);
        add_wall_x(&mut v, &mut col, 3.0, 10.0, 40.0, -10.0, -4.0, -1.0, c_wall);
        add_light_fixture(&mut v, 0.0, -4.1, 15.0);
        add_light_fixture(&mut v, 0.0, -4.1, 22.0);
        add_light_fixture(&mut v, 0.0, -4.1, 30.0);
        all_colliders.extend(col);
        chunks.push((v, Collider { min: Vec3::new(-3.0, -10.0, 10.0), max: Vec3::new(3.0, -4.0, 40.0) }));
    }

    // ============ ЧАНК 3: ЗАЛ С КОЛОННАМИ ЛЕВАЯ ============
    {
        let mut v = Vec::new();
        let mut col = Vec::new();
        add_floor(&mut v, &mut col, -25.0, 0.0, 40.0, 80.0, -10.0, c_floor);
        add_ceiling(&mut v, &mut col, -25.0, 0.0, 40.0, 80.0, 5.0, c_ceil);
        add_wall_x(&mut v, &mut col, -25.0, 40.0, 80.0, -10.0, 5.0, 1.0, c_wall);
        add_wall_z(&mut v, &mut col, 40.0, -25.0, -3.0, -10.0, 5.0, 1.0, c_wall); // от -25 до -3
        add_wall_z(&mut v, &mut col, 40.0, -3.0, 0.0, -4.0, 5.0, 1.0, c_wall); // над проемом
        add_wall_z(&mut v, &mut col, 80.0, -25.0, 0.0, -10.0, 5.0, -1.0, c_wall);
        for &x in &[-15.0, -5.0] {
            for &z in &[50.0, 60.0, 70.0] {
                add_cube(&mut v, &mut col, Vec3::new(x, -2.5, z), Vec3::new(2.0, 15.0, 2.0), c_pillar);
            }
        }
        add_light_fixture(&mut v, -12.0, 4.9, 50.0);
        add_light_fixture(&mut v, -12.0, 4.9, 65.0);
        add_light_fixture(&mut v, -18.0, 4.9, 75.0);
        all_colliders.extend(col);
        chunks.push((v, Collider { min: Vec3::new(-25.0, -10.0, 40.0), max: Vec3::new(0.0, 5.0, 80.0) }));
    }

    // ============ ЧАНК 4: ЗАЛ С КОЛОННАМИ ПРАВАЯ ============
    {
        let mut v = Vec::new();
        let mut col = Vec::new();
        add_floor(&mut v, &mut col, 0.0, 25.0, 40.0, 80.0, -10.0, c_floor);
        add_ceiling(&mut v, &mut col, 0.0, 25.0, 40.0, 80.0, 5.0, c_ceil);
        add_wall_z(&mut v, &mut col, 40.0, 0.0, 3.0, -4.0, 5.0, 1.0, c_wall); // над проемом
        add_wall_z(&mut v, &mut col, 40.0, 3.0, 25.0, -10.0, 5.0, 1.0, c_wall); // правая часть
        add_wall_x(&mut v, &mut col, 25.0, 40.0, 55.0, -10.0, 5.0, -1.0, c_wall);
        add_wall_x(&mut v, &mut col, 25.0, 55.0, 65.0, -6.0, 5.0, -1.0, c_wall); // над проемом
        add_wall_x(&mut v, &mut col, 25.0, 65.0, 80.0, -10.0, 5.0, -1.0, c_wall);
        add_wall_z(&mut v, &mut col, 80.0, 0.0, 25.0, -10.0, 5.0, -1.0, c_wall);
        for &x in &[5.0, 15.0] {
            for &z in &[50.0, 60.0, 70.0] {
                add_cube(&mut v, &mut col, Vec3::new(x, -2.5, z), Vec3::new(2.0, 15.0, 2.0), c_pillar);
            }
        }
        add_light_fixture(&mut v, 12.0, 4.9, 50.0);
        add_light_fixture(&mut v, 12.0, 4.9, 65.0);
        add_light_fixture(&mut v, 18.0, 4.9, 75.0);
        all_colliders.extend(col);
        chunks.push((v, Collider { min: Vec3::new(0.0, -10.0, 40.0), max: Vec3::new(25.0, 5.0, 80.0) }));
    }

    // ============ ЧАНК 5: КОРИДОР 2 ============
    {
        let mut v = Vec::new();
        let mut col = Vec::new();
        add_floor(&mut v, &mut col, 25.0, 55.0, 55.0, 65.0, -10.0, c_floor);
        add_ceiling(&mut v, &mut col, 25.0, 55.0, 55.0, 65.0, -6.0, c_ceil);
        add_wall_z(&mut v, &mut col, 55.0, 25.0, 55.0, -10.0, -6.0, -1.0, c_wall);
        add_wall_z(&mut v, &mut col, 65.0, 25.0, 55.0, -10.0, -6.0, 1.0, c_wall);
        add_light_fixture(&mut v, 35.0, -6.1, 60.0);
        add_light_fixture(&mut v, 45.0, -6.1, 60.0);
        all_colliders.extend(col);
        chunks.push((v, Collider { min: Vec3::new(25.0, -10.0, 55.0), max: Vec3::new(55.0, -6.0, 65.0) }));
    }

    // ============ ЧАНК 6: БАССЕЙН ============
    {
        let mut v = Vec::new();
        let mut col = Vec::new();
        add_floor(&mut v, &mut col, 55.0, 65.0, 40.0, 80.0, -10.0, c_floor);
        add_floor(&mut v, &mut col, 80.0, 90.0, 40.0, 80.0, -10.0, c_floor);
        add_floor(&mut v, &mut col, 65.0, 80.0, 40.0, 50.0, -10.0, c_floor);
        add_floor(&mut v, &mut col, 65.0, 80.0, 70.0, 80.0, -10.0, c_floor);
        add_floor(&mut v, &mut col, 65.0, 80.0, 50.0, 70.0, -9.0, c_pool);
        add_ceiling(&mut v, &mut col, 55.0, 90.0, 40.0, 80.0, 2.0, c_ceil);
        add_wall_x(&mut v, &mut col, 90.0, 40.0, 80.0, -10.0, 2.0, -1.0, c_wall);
        add_wall_x(&mut v, &mut col, 55.0, 40.0, 55.0, -10.0, 2.0, 1.0, c_wall);
        add_wall_x(&mut v, &mut col, 55.0, 55.0, 65.0, -6.0, 2.0, 1.0, c_wall); // над проемом
        add_wall_x(&mut v, &mut col, 55.0, 65.0, 80.0, -10.0, 2.0, 1.0, c_wall);
        add_wall_z(&mut v, &mut col, 40.0, 55.0, 90.0, -10.0, 2.0, 1.0, c_wall);
        add_wall_z(&mut v, &mut col, 80.0, 55.0, 65.0, -6.0, 2.0, -1.0, c_wall); // над проемом
        add_wall_z(&mut v, &mut col, 80.0, 65.0, 90.0, -10.0, 2.0, -1.0, c_wall);
        add_light_fixture(&mut v, 62.0, 1.9, 45.0);
        add_light_fixture(&mut v, 72.0, 1.9, 60.0);
        add_light_fixture(&mut v, 85.0, 1.9, 75.0);
        all_colliders.extend(col);
        chunks.push((v, Collider { min: Vec3::new(55.0, -10.0, 40.0), max: Vec3::new(90.0, 2.0, 80.0) }));
    }

    // ============ ЧАНК 7: КОРИДОР 3 ============
    {
        let mut v = Vec::new();
        let mut col = Vec::new();
        add_floor(&mut v, &mut col, 55.0, 65.0, 80.0, 110.0, -10.0, c_floor);
        add_ceiling(&mut v, &mut col, 55.0, 65.0, 80.0, 110.0, -6.0, c_ceil);
        add_wall_x(&mut v, &mut col, 55.0, 80.0, 110.0, -10.0, -6.0, 1.0, c_wall);
        add_wall_x(&mut v, &mut col, 65.0, 80.0, 110.0, -10.0, -6.0, -1.0, c_wall);
        add_light_fixture(&mut v, 60.0, -6.1, 90.0);
        add_light_fixture(&mut v, 60.0, -6.1, 105.0);
        all_colliders.extend(col);
        chunks.push((v, Collider { min: Vec3::new(55.0, -10.0, 80.0), max: Vec3::new(65.0, -6.0, 110.0) }));
    }

    // ============ ЧАНК 8: ФИНАЛЬНЫЙ ЗАЛ ============
    {
        let mut v = Vec::new();
        let mut col = Vec::new();
        add_floor(&mut v, &mut col, 10.0, 65.0, 110.0, 140.0, -10.0, c_floor);
        add_ceiling(&mut v, &mut col, 10.0, 65.0, 110.0, 140.0, 5.0, c_ceil);
        add_wall_x(&mut v, &mut col, 10.0, 110.0, 140.0, -10.0, 5.0, 1.0, c_wall);
        add_wall_x(&mut v, &mut col, 65.0, 110.0, 140.0, -10.0, 5.0, -1.0, c_wall);
        add_wall_z(&mut v, &mut col, 140.0, 10.0, 65.0, -10.0, 5.0, -1.0, c_wall);
        add_wall_z(&mut v, &mut col, 110.0, 10.0, 55.0, -10.0, 5.0, 1.0, c_wall); // от 10 до 55
        add_wall_z(&mut v, &mut col, 110.0, 55.0, 65.0, -6.0, 5.0, 1.0, c_wall); // над проемом
        add_cube(&mut v, &mut col, Vec3::new(20.0, -9.0, 120.0), Vec3::new(2.0, 2.0, 2.0), [0.6, 0.6, 0.6]);
        add_cube(&mut v, &mut col, Vec3::new(35.0, -9.0, 125.0), Vec3::new(2.0, 2.0, 2.0), [0.6, 0.6, 0.6]);
        add_cube(&mut v, &mut col, Vec3::new(40.0, -9.0, 130.0), Vec3::new(2.0, 2.0, 2.0), [0.6, 0.6, 0.6]);
        add_cube(&mut v, &mut col, Vec3::new(30.0, -9.0, 135.0), Vec3::new(2.0, 2.0, 2.0), [0.6, 0.6, 0.6]);
        add_cube(&mut v, &mut col, Vec3::new(15.0, -2.5, 120.0), Vec3::new(2.0, 15.0, 2.0), c_pillar);
        add_cube(&mut v, &mut col, Vec3::new(60.0, -2.5, 120.0), Vec3::new(2.0, 15.0, 2.0), c_pillar);
        add_light_fixture(&mut v, 20.0, 4.9, 115.0);
        add_light_fixture(&mut v, 30.0, 4.9, 125.0);
        add_light_fixture(&mut v, 40.0, 4.9, 135.0);
        all_colliders.extend(col);
        chunks.push((v, Collider { min: Vec3::new(10.0, -10.0, 110.0), max: Vec3::new(65.0, 5.0, 140.0) }));
    }

    (chunks, all_colliders)
}

pub fn generate_wall_texture() -> Vec<u8> {
    let size = 256;
    let mut pixels = vec![0u8; size * size * 4];
    let noise = |x: u32, y: u32| -> f32 {
        let mut n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
        n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        ((n & 0xFFFF) as f32) / 65535.0
    };
    for y in 0..size as u32 {
        for x in 0..size as u32 {
            let idx = ((y * size as u32 + x) * 4) as usize;
            let mut val = 0.0;
            val += noise(x, y) * 0.5;
            val += noise(x * 2, y * 2) * 0.25;
            val += noise(x * 4, y * 4) * 0.125;
            val += noise(x * 8, y * 8) * 0.0625;
            let base = 100.0;
            let color = (base + val * 60.0) as u8;
            pixels[idx] = color; pixels[idx + 1] = color; pixels[idx + 2] = color; pixels[idx + 3] = 255;
        }
    }
    pixels
}