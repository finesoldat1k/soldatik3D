//! Collision broadphase and a robust AABB character controller.
//!
//! The demo does not use a physics engine: the facility is made of static
//! axis-aligned boxes, and a small uniform grid keeps collision queries local.
//! Movement is substepped and resolved one axis at a time, which gives smooth
//! wall sliding and prevents tunnelling even on a slow frame. Grounded players
//! step over low ledges automatically, and coyote time + jump buffering make
//! jumps forgiving on touch screens.

use runtime::Aabb;
use runtime::glam::Vec3;

/// Uniform grid cell size in world units. The facility is roughly 65×60 m.
const CELL_SIZE: f32 = 4.0;
/// Tolerance used to ignore touching faces (a player standing on a floor must
/// not count as overlapping it).
const EPS: f32 = 0.001;

pub const PLAYER_RADIUS: f32 = 0.34;
pub const PLAYER_HEIGHT: f32 = 1.78;
pub const EYE_HEIGHT: f32 = 1.62;

const STEP_HEIGHT: f32 = 0.45;
const SNAP_DOWN: f32 = 0.12;
const MAX_SUBSTEP: f32 = 0.22;
const GRAVITY: f32 = 24.0;
const TERMINAL_FALL: f32 = 32.0;
const WALK_SPEED: f32 = 5.6;
const SPRINT_SPEED: f32 = 8.2;
const GROUND_ACCEL: f32 = 22.0;
const AIR_ACCEL: f32 = 4.5;
const GROUND_FRICTION: f32 = 14.0;
const JUMP_SPEED: f32 = 8.1;
const COYOTE_TIME: f32 = 0.11;
const JUMP_BUFFER_TIME: f32 = 0.14;
const FLY_SPEED: f32 = 14.0;

/// Static colliders plus a uniform XZ grid for fast queries.
pub struct PhysicsWorld {
    colliders: Vec<Aabb>,
    cells: Vec<Vec<u32>>,
    min_x: f32,
    min_z: f32,
    cells_x: usize,
    cells_z: usize,
}

impl PhysicsWorld {
    pub fn new(colliders: Vec<Aabb>) -> Self {
        let (min_x, max_x, min_z, max_z) = colliders.iter().fold(
            (
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
            ),
            |(min_x, max_x, min_z, max_z), collider| {
                (
                    min_x.min(collider.min.x),
                    max_x.max(collider.max.x),
                    min_z.min(collider.min.z),
                    max_z.max(collider.max.z),
                )
            },
        );
        let (min_x, max_x, min_z, max_z) = if min_x.is_finite() {
            (
                min_x - CELL_SIZE,
                max_x + CELL_SIZE,
                min_z - CELL_SIZE,
                max_z + CELL_SIZE,
            )
        } else {
            (-CELL_SIZE, CELL_SIZE, -CELL_SIZE, CELL_SIZE)
        };
        let cells_x = (((max_x - min_x) / CELL_SIZE).ceil() as usize).max(1);
        let cells_z = (((max_z - min_z) / CELL_SIZE).ceil() as usize).max(1);
        let mut cells = vec![Vec::new(); cells_x * cells_z];
        for (index, collider) in colliders.iter().enumerate() {
            let (x0, x1) = cell_span(collider.min.x, collider.max.x, min_x, cells_x);
            let (z0, z1) = cell_span(collider.min.z, collider.max.z, min_z, cells_z);
            for z in z0..=z1 {
                for x in x0..=x1 {
                    cells[z * cells_x + x].push(index as u32);
                }
            }
        }
        Self {
            colliders,
            cells,
            min_x,
            min_z,
            cells_x,
            cells_z,
        }
    }

    pub fn colliders(&self) -> &[Aabb] {
        &self.colliders
    }

    /// Appends indices of colliders whose XZ footprint touches `bounds`.
    /// The caller must filter by the Y axis because the grid is 2D.
    pub fn query(&self, bounds: &Aabb, out: &mut Vec<u32>) {
        out.clear();
        let (x0, x1) = cell_span(bounds.min.x, bounds.max.x, self.min_x, self.cells_x);
        let (z0, z1) = cell_span(bounds.min.z, bounds.max.z, self.min_z, self.cells_z);
        for z in z0..=z1 {
            let row = z * self.cells_x;
            for x in x0..=x1 {
                out.extend_from_slice(&self.cells[row + x]);
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}

fn cell_span(min: f32, max: f32, origin: f32, count: usize) -> (usize, usize) {
    let first = (((min - origin) / CELL_SIZE).floor() as isize).clamp(0, count as isize - 1);
    let last = (((max - origin) / CELL_SIZE).floor() as isize).clamp(0, count as isize - 1);
    (first as usize, last as usize)
}

/// Feet-centred character body with velocity and buffered jump state.
pub struct Character {
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub flying: bool,
    coyote: f32,
    jump_buffer: f32,
    scratch: Vec<u32>,
}

impl Character {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            on_ground: false,
            flying: false,
            coyote: 0.0,
            jump_buffer: 0.0,
            scratch: Vec::new(),
        }
    }

    pub fn teleport(&mut self, position: Vec3) {
        self.position = position;
        self.velocity = Vec3::ZERO;
        self.on_ground = false;
        self.coyote = 0.0;
        self.jump_buffer = 0.0;
    }

    pub fn set_flying(&mut self, flying: bool) {
        self.flying = flying;
        self.velocity = Vec3::ZERO;
        self.on_ground = false;
    }

    /// Advances the body by one frame.
    ///
    /// `wish` is a horizontal direction in world space (length is ignored).
    /// `jump_pressed` is the press edge, used for normal jumping. `jump_held`
    /// is the continuous Space state, used to ascend while flying.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        dt: f32,
        world: &PhysicsWorld,
        wish: Vec3,
        jump_pressed: bool,
        jump_held: bool,
        crouch: bool,
        sprint: bool,
    ) {
        let dt = dt.clamp(0.0, 0.05);
        if self.flying {
            self.update_flying(dt, wish, jump_held, crouch, sprint);
            return;
        }

        if jump_pressed {
            self.jump_buffer = JUMP_BUFFER_TIME;
        } else {
            self.jump_buffer = (self.jump_buffer - dt).max(0.0);
        }
        if self.on_ground {
            self.coyote = COYOTE_TIME;
        } else {
            self.coyote = (self.coyote - dt).max(0.0);
        }

        if self.jump_buffer > 0.0 && (self.on_ground || self.coyote > 0.0) {
            self.velocity.y = JUMP_SPEED;
            self.on_ground = false;
            self.coyote = 0.0;
            self.jump_buffer = 0.0;
        }

        let speed = if sprint { SPRINT_SPEED } else { WALK_SPEED };
        let target = wish.normalize_or_zero() * speed;
        let accel = if self.on_ground {
            GROUND_ACCEL
        } else {
            AIR_ACCEL
        };
        let blend = 1.0 - (-accel * dt).exp();
        self.velocity.x += (target.x - self.velocity.x) * blend;
        self.velocity.z += (target.z - self.velocity.z) * blend;
        if self.on_ground && wish.length_squared() < 1e-6 {
            let friction = (-GROUND_FRICTION * dt).exp();
            self.velocity.x *= friction;
            self.velocity.z *= friction;
        }

        self.velocity.y = (self.velocity.y - GRAVITY * dt).max(-TERMINAL_FALL);
        self.integrate(dt, world);
    }

    fn update_flying(&mut self, dt: f32, wish: Vec3, ascend: bool, crouch: bool, sprint: bool) {
        let speed = if sprint { FLY_SPEED * 1.8 } else { FLY_SPEED };
        let target = wish.normalize_or_zero() * speed;
        let blend = 1.0 - (-8.0 * dt).exp();
        self.velocity.x += (target.x - self.velocity.x) * blend;
        self.velocity.z += (target.z - self.velocity.z) * blend;
        let vertical = if ascend { 1.0 } else { 0.0 } - if crouch { 1.0 } else { 0.0 };
        // Faster vertical blend for more responsive flight controls
        let vertical_blend = 1.0 - (-20.0 * dt).exp();
        self.velocity.y += (vertical * speed - self.velocity.y) * vertical_blend;
        self.position += self.velocity * dt;
        self.on_ground = false;
    }

    fn integrate(&mut self, dt: f32, world: &PhysicsWorld) {
        let distance = self.velocity.length() * dt;
        let steps = ((distance / MAX_SUBSTEP).ceil() as usize).clamp(1, 16);
        let step_dt = dt / steps as f32;
        for _ in 0..steps {
            self.move_horizontal(self.velocity.x * step_dt, self.velocity.z * step_dt, world);
            self.move_vertical(self.velocity.y * step_dt, world);
        }
        // A grounded body should not accumulate downward velocity from gravity.
        if self.on_ground && self.velocity.y < 0.0 {
            self.velocity.y = 0.0;
        }
    }

    fn bounds_at(position: Vec3) -> Aabb {
        Aabb::new(
            position - Vec3::new(PLAYER_RADIUS, 0.0, PLAYER_RADIUS),
            position + Vec3::new(PLAYER_RADIUS, PLAYER_HEIGHT, PLAYER_RADIUS),
        )
    }

    fn overlaps_at(&mut self, position: Vec3, world: &PhysicsWorld) -> bool {
        let bounds = Self::bounds_at(position);
        world.query(&bounds, &mut self.scratch);
        let hits = std::mem::take(&mut self.scratch);
        let result = hits
            .iter()
            .any(|&index| strict_overlap(&bounds, &world.colliders()[index as usize]));
        self.scratch = hits;
        result
    }

    fn move_horizontal(&mut self, dx: f32, dz: f32, world: &PhysicsWorld) {
        if dx.abs() < 1e-6 && dz.abs() < 1e-6 {
            return;
        }
        let target = self.position + Vec3::new(dx, 0.0, dz);
        if !self.overlaps_at(target, world) {
            self.position = target;
            return;
        }
        // Walking into a low step: rise by STEP_HEIGHT, continue horizontally,
        // then drop onto the support below. Fails against real walls.
        if self.on_ground && self.try_step_up(dx, dz, world) {
            return;
        }
        self.move_axis_x(dx, world);
        self.move_axis_z(dz, world);
    }

    fn try_step_up(&mut self, dx: f32, dz: f32, world: &PhysicsWorld) -> bool {
        let lifted = self.position + Vec3::Y * STEP_HEIGHT;
        if self.overlaps_at(lifted, world) {
            return false;
        }
        let target = lifted + Vec3::new(dx, 0.0, dz);
        if self.overlaps_at(target, world) {
            return false;
        }
        let Some(support) = self.support_below(target, STEP_HEIGHT + SNAP_DOWN, world) else {
            return false;
        };
        if support <= self.position.y + 0.001 {
            return false;
        }
        self.position = Vec3::new(target.x, support, target.z);
        self.velocity.y = 0.0;
        self.on_ground = true;
        true
    }

    fn move_axis_x(&mut self, dx: f32, world: &PhysicsWorld) {
        if dx.abs() < 1e-6 {
            return;
        }
        let mut target_x = self.position.x + dx;
        let bounds = Self::bounds_at(Vec3::new(target_x, self.position.y, self.position.z));
        world.query(&bounds, &mut self.scratch);
        let hits = std::mem::take(&mut self.scratch);
        let mut blocked = false;
        for &index in &hits {
            let collider = &world.colliders()[index as usize];
            if !vertical_overlap(self.position.y, collider) {
                continue;
            }
            if !overlap_z(self.position.z, collider) {
                continue;
            }
            if dx > 0.0 && self.position.x + PLAYER_RADIUS <= collider.min.x + EPS {
                target_x = target_x.min(collider.min.x - PLAYER_RADIUS - EPS);
                blocked = true;
            } else if dx < 0.0 && self.position.x - PLAYER_RADIUS >= collider.max.x - EPS {
                target_x = target_x.max(collider.max.x + PLAYER_RADIUS + EPS);
                blocked = true;
            }
        }
        self.scratch = hits;
        if blocked {
            self.velocity.x = 0.0;
        }
        self.position.x = target_x;
    }

    fn move_axis_z(&mut self, dz: f32, world: &PhysicsWorld) {
        if dz.abs() < 1e-6 {
            return;
        }
        let mut target_z = self.position.z + dz;
        let bounds = Self::bounds_at(Vec3::new(self.position.x, self.position.y, target_z));
        world.query(&bounds, &mut self.scratch);
        let hits = std::mem::take(&mut self.scratch);
        let mut blocked = false;
        for &index in &hits {
            let collider = &world.colliders()[index as usize];
            if !vertical_overlap(self.position.y, collider) {
                continue;
            }
            if !overlap_x(self.position.x, collider) {
                continue;
            }
            if dz > 0.0 && self.position.z + PLAYER_RADIUS <= collider.min.z + EPS {
                target_z = target_z.min(collider.min.z - PLAYER_RADIUS - EPS);
                blocked = true;
            } else if dz < 0.0 && self.position.z - PLAYER_RADIUS >= collider.max.z - EPS {
                target_z = target_z.max(collider.max.z + PLAYER_RADIUS + EPS);
                blocked = true;
            }
        }
        self.scratch = hits;
        if blocked {
            self.velocity.z = 0.0;
        }
        self.position.z = target_z;
    }

    fn move_vertical(&mut self, dy: f32, world: &PhysicsWorld) {
        if dy.abs() < 1e-7 {
            // Still probe for support so `on_ground` stays accurate.
            self.refresh_ground(world);
            return;
        }
        let target_y = self.position.y + dy;
        let bounds = Self::bounds_at(Vec3::new(self.position.x, target_y, self.position.z));
        world.query(&bounds, &mut self.scratch);
        let hits = std::mem::take(&mut self.scratch);
        let mut candidate_y = target_y;
        let mut landed = false;
        let mut bumped_head = false;
        for &index in &hits {
            let collider = &world.colliders()[index as usize];
            if !overlap_x(self.position.x, collider) || !overlap_z(self.position.z, collider) {
                continue;
            }
            if dy > 0.0
                && self.position.y + PLAYER_HEIGHT <= collider.min.y + EPS
                && target_y + PLAYER_HEIGHT >= collider.min.y - EPS
            {
                candidate_y = candidate_y.min(collider.min.y - PLAYER_HEIGHT);
                bumped_head = true;
            } else if dy <= 0.0
                && collider.max.y <= self.position.y + EPS
                && target_y <= collider.max.y + EPS
            {
                candidate_y = candidate_y.max(collider.max.y);
                landed = true;
            }
        }
        self.scratch = hits;
        if bumped_head {
            self.velocity.y = self.velocity.y.min(0.0);
        }
        if landed {
            self.velocity.y = 0.0;
            self.on_ground = true;
        }
        self.position.y = candidate_y;

        if !landed && dy <= 0.0 {
            let floor = self.position.y - SNAP_DOWN;
            if let Some(support) = self
                .support_below(self.position, SNAP_DOWN, world)
                .filter(|support| *support >= floor)
            {
                self.position.y = support;
                self.velocity.y = 0.0;
                self.on_ground = true;
                return;
            }
            self.on_ground = false;
        } else if dy > 0.0 {
            self.on_ground = false;
        }
    }

    fn refresh_ground(&mut self, world: &PhysicsWorld) {
        if self.velocity.y > 0.0 {
            self.on_ground = false;
            return;
        }
        let floor = self.position.y - SNAP_DOWN;
        if let Some(support) = self
            .support_below(self.position, SNAP_DOWN, world)
            .filter(|support| *support >= floor)
        {
            self.position.y = support;
            self.on_ground = true;
            return;
        }
        self.on_ground = false;
    }

    /// Highest collider top under `position` within `max_drop`.
    fn support_below(
        &mut self,
        position: Vec3,
        max_drop: f32,
        world: &PhysicsWorld,
    ) -> Option<f32> {
        let probe = Aabb::new(
            Vec3::new(
                position.x - PLAYER_RADIUS + 0.02,
                position.y - max_drop,
                position.z - PLAYER_RADIUS + 0.02,
            ),
            Vec3::new(
                position.x + PLAYER_RADIUS - 0.02,
                position.y + 0.02,
                position.z + PLAYER_RADIUS - 0.02,
            ),
        );
        world.query(&probe, &mut self.scratch);
        let hits = std::mem::take(&mut self.scratch);
        let mut support: Option<f32> = None;
        for &index in &hits {
            let collider = &world.colliders()[index as usize];
            if collider.max.y > position.y + 0.02 || collider.max.y < position.y - max_drop {
                continue;
            }
            if collider.max.x <= probe.min.x
                || collider.min.x >= probe.max.x
                || collider.max.z <= probe.min.z
                || collider.min.z >= probe.max.z
            {
                continue;
            }
            support = Some(support.map_or(collider.max.y, |value| value.max(collider.max.y)));
        }
        self.scratch = hits;
        support
    }
}

fn strict_overlap(a: &Aabb, b: &Aabb) -> bool {
    a.min.x < b.max.x - EPS
        && a.max.x > b.min.x + EPS
        && a.min.y < b.max.y - EPS
        && a.max.y > b.min.y + EPS
        && a.min.z < b.max.z - EPS
        && a.max.z > b.min.z + EPS
}

fn vertical_overlap(feet: f32, collider: &Aabb) -> bool {
    feet + PLAYER_HEIGHT > collider.min.y + EPS && feet < collider.max.y - EPS
}

fn overlap_x(x: f32, collider: &Aabb) -> bool {
    x + PLAYER_RADIUS > collider.min.x + EPS && x - PLAYER_RADIUS < collider.max.x - EPS
}

fn overlap_z(z: f32, collider: &Aabb) -> bool {
    z + PLAYER_RADIUS > collider.min.z + EPS && z - PLAYER_RADIUS < collider.max.z - EPS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_world() -> PhysicsWorld {
        PhysicsWorld::new(vec![Aabb::new(
            Vec3::new(-20.0, -1.0, -20.0),
            Vec3::new(20.0, 0.0, 20.0),
        )])
    }

    #[test]
    fn character_falls_and_lands_on_floor() {
        let world = floor_world();
        let mut character = Character::new(Vec3::new(0.0, 2.0, 0.0));
        character.update(1.0 / 60.0, &world, Vec3::ZERO, false, false, false, false);
        assert!(!character.on_ground);
        assert!(
            character.position.y > 1.9,
            "falling must not snap to the floor"
        );
        for _ in 0..120 {
            character.update(1.0 / 60.0, &world, Vec3::ZERO, false, false, false, false);
        }
        assert!(character.on_ground);
        assert!((character.position.y).abs() < 0.01);
    }

    #[test]
    fn character_walks_up_a_step() {
        let mut world = floor_world();
        let step = Aabb::new(Vec3::new(1.0, 0.0, -2.0), Vec3::new(3.0, 0.3, 2.0));
        // Add the step through a fresh world so the grid sees it.
        let mut colliders = world.colliders().to_vec();
        colliders.push(step);
        world = PhysicsWorld::new(colliders);
        let mut character = Character::new(Vec3::new(0.0, 0.001, 0.0));
        // Walk far enough to climb the step but stop before stepping off its
        // far edge, otherwise the character correctly falls back to the floor.
        for _ in 0..30 {
            character.update(1.0 / 60.0, &world, Vec3::X, false, false, false, false);
        }
        assert!(character.position.x > 1.5);
        assert!((character.position.y - 0.3).abs() < 0.01);
    }

    #[test]
    fn character_does_not_pass_through_walls() {
        let mut world = floor_world();
        let mut colliders = world.colliders().to_vec();
        colliders.push(Aabb::new(
            Vec3::new(2.0, 0.0, -3.0),
            Vec3::new(2.5, 3.0, 3.0),
        ));
        world = PhysicsWorld::new(colliders);
        let mut character = Character::new(Vec3::new(0.0, 0.001, 0.0));
        for _ in 0..180 {
            character.update(1.0 / 60.0, &world, Vec3::X, false, false, false, true);
        }
        assert!(character.position.x < 2.0);
    }

    #[test]
    fn jump_has_coyote_time() {
        let world = floor_world();
        let mut character = Character::new(Vec3::new(0.0, 0.001, 0.0));
        for _ in 0..30 {
            character.update(1.0 / 60.0, &world, Vec3::ZERO, false, false, false, false);
        }
        // Walk off the floor edge is not set up here; jump directly.
        character.update(1.0 / 60.0, &world, Vec3::ZERO, true, true, false, false);
        assert!(character.position.y > 0.0);
        assert!(!character.on_ground);
    }

    #[test]
    fn holding_jump_ascends_continuously_while_flying() {
        let world = floor_world();
        let mut character = Character::new(Vec3::new(0.0, 4.0, 0.0));
        character.set_flying(true);
        let start = character.position.y;
        for _ in 0..60 {
            character.update(1.0 / 60.0, &world, Vec3::ZERO, false, true, false, false);
        }
        assert!(
            character.position.y > start + 10.0,
            "holding Space must keep ascending, got {}",
            character.position.y - start
        );
        // After releasing jump, the character should decelerate and hover.
        // It may coast up a bit more due to momentum before stopping.
        let peak = character.position.y;
        for _ in 0..60 {
            character.update(1.0 / 60.0, &world, Vec3::ZERO, false, false, false, false);
        }
        // Should stop near peak (hovering), not fall.
        assert!(
            (character.position.y - peak).abs() < 1.0,
            "releasing jump in flight should hover, got y={}, peak={}",
            character.position.y,
            peak
        );
        // Now test descending with crouch
        for _ in 0..30 {
            character.update(1.0 / 60.0, &world, Vec3::ZERO, false, false, true, false);
        }
        assert!(character.position.y < peak - 2.0);
    }

    #[test]
    fn flying_ignores_world_collision() {
        let mut world = floor_world();
        let mut colliders = world.colliders().to_vec();
        colliders.push(Aabb::new(
            Vec3::new(-3.0, 0.0, -3.0),
            Vec3::new(3.0, 2.0, 3.0),
        ));
        world = PhysicsWorld::new(colliders);
        let mut character = Character::new(Vec3::new(0.0, 0.5, 0.0));
        character.set_flying(true);
        for _ in 0..30 {
            character.update(1.0 / 60.0, &world, Vec3::ZERO, false, true, false, false);
        }
        assert!(character.position.y > 2.5);
    }
}
