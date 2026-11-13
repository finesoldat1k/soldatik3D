use std::fmt;

use crate::camera::Camera;

/// Stable identifier of a camera inside a [`CameraRig`].
///
/// IDs are never reused: removing a camera does not invalidate other IDs, and
/// spawning a new camera always returns a fresh value. This makes it safe for
/// gameplay code to remember which camera it controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CameraId(u32);

impl CameraId {
    /// Raw numeric value, useful for serialization and console output.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Builds an ID from its raw value, for example after parsing user input.
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }
}

impl fmt::Display for CameraId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "cam-{}", self.0)
    }
}

/// A camera together with its human-readable name.
#[derive(Clone, Debug)]
pub struct NamedCamera {
    pub id: CameraId,
    pub name: String,
    pub camera: Camera,
}

/// A collection of cameras with exactly one active camera.
///
/// The rig is the engine's central camera registry. Games spawn cameras once
/// (or at runtime), then move/rotate the active one or switch to another. The
/// runtime always renders through [`CameraRig::active`].
///
/// ```
/// use scene::{Camera, CameraRig};
/// use scene::glam::Vec3;
///
/// let mut cameras = CameraRig::new();
/// let overview = cameras.spawn("overview", Camera::at(Vec3::new(0.0, 10.0, 20.0)));
/// assert!(cameras.select(overview));
/// assert_eq!(cameras.active().id, overview);
/// ```
#[derive(Clone, Debug)]
pub struct CameraRig {
    cameras: Vec<NamedCamera>,
    active: usize,
    next_id: u32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraRig {
    /// Creates a rig with a single active camera named `main`.
    pub fn new() -> Self {
        let mut rig = Self {
            cameras: Vec::new(),
            active: 0,
            next_id: 0,
        };
        rig.spawn("main", Camera::default());
        rig
    }

    /// Creates an empty rig. At least one camera must be spawned before it can
    /// be used for rendering.
    pub fn empty() -> Self {
        Self {
            cameras: Vec::new(),
            active: 0,
            next_id: 0,
        }
    }

    /// Spawns a camera under a name and returns its stable ID.
    ///
    /// If the name is already taken, a numeric suffix (`.2`, `.3`, ...) is
    /// appended. The first camera spawned into an empty rig becomes active.
    pub fn spawn(&mut self, name: impl Into<String>, camera: Camera) -> CameraId {
        let id = CameraId(self.next_id);
        self.next_id += 1;
        let name = self.unique_name(name.into());
        if self.cameras.is_empty() {
            self.active = 0;
        }
        self.cameras.push(NamedCamera { id, name, camera });
        id
    }

    /// Spawns a camera with an automatically generated name.
    pub fn add(&mut self, camera: Camera) -> CameraId {
        let name = format!("camera-{}", self.next_id + 1);
        self.spawn(name, camera)
    }

    /// Removes a camera. The last remaining camera cannot be removed.
    ///
    /// If the active camera is removed, the next camera in the list becomes
    /// active. Returns the removed camera on success.
    pub fn remove(&mut self, id: CameraId) -> Option<NamedCamera> {
        if self.cameras.len() <= 1 {
            return None;
        }
        let index = self.cameras.iter().position(|entry| entry.id == id)?;
        let removed = self.cameras.remove(index);
        if self.active >= self.cameras.len() {
            self.active = self.cameras.len() - 1;
        } else if index < self.active {
            self.active -= 1;
        }
        Some(removed)
    }

    /// Makes the camera with `id` active. Returns `false` for unknown IDs.
    pub fn select(&mut self, id: CameraId) -> bool {
        match self.cameras.iter().position(|entry| entry.id == id) {
            Some(index) => {
                self.active = index;
                true
            }
            None => false,
        }
    }

    /// Makes the camera with an exact `name` active.
    pub fn select_name(&mut self, name: &str) -> bool {
        match self.cameras.iter().position(|entry| entry.name == name) {
            Some(index) => {
                self.active = index;
                true
            }
            None => false,
        }
    }

    /// Selects the next camera in spawn order and returns its ID.
    pub fn select_next(&mut self) -> Option<CameraId> {
        if self.cameras.is_empty() {
            return None;
        }
        self.active = (self.active + 1) % self.cameras.len();
        Some(self.active().id)
    }

    /// Selects the previous camera in spawn order and returns its ID.
    pub fn select_previous(&mut self) -> Option<CameraId> {
        if self.cameras.is_empty() {
            return None;
        }
        self.active = (self.active + self.cameras.len() - 1) % self.cameras.len();
        Some(self.active().id)
    }

    /// The camera the engine renders with.
    ///
    /// # Panics
    ///
    /// Panics if the rig is empty. Use [`CameraRig::is_empty`] if a rig might
    /// have been created with [`CameraRig::empty`].
    pub fn active(&self) -> &NamedCamera {
        &self.cameras[self.active]
    }

    /// Renames the active camera. Returns the final unique name, or `None` if
    /// the rig is empty.
    pub fn rename_active(&mut self, name: impl Into<String>) -> Option<String> {
        let name = self.unique_name_excluding(name.into(), self.active);
        let entry = self.cameras.get_mut(self.active)?;
        entry.name = name.clone();
        Some(name)
    }

    /// Mutable access to the active camera.
    pub fn active_mut(&mut self) -> &mut NamedCamera {
        &mut self.cameras[self.active]
    }

    pub fn active_id(&self) -> Option<CameraId> {
        self.cameras.get(self.active).map(|entry| entry.id)
    }

    pub fn active_name(&self) -> Option<&str> {
        self.cameras
            .get(self.active)
            .map(|entry| entry.name.as_str())
    }

    pub fn get(&self, id: CameraId) -> Option<&NamedCamera> {
        self.cameras.iter().find(|entry| entry.id == id)
    }

    pub fn get_mut(&mut self, id: CameraId) -> Option<&mut NamedCamera> {
        self.cameras.iter_mut().find(|entry| entry.id == id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&NamedCamera> {
        self.cameras.iter().find(|entry| entry.name == name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &NamedCamera> {
        self.cameras.iter()
    }

    pub fn len(&self) -> usize {
        self.cameras.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cameras.is_empty()
    }

    fn unique_name(&self, base: String) -> String {
        self.unique_name_excluding(base, usize::MAX)
    }

    fn unique_name_excluding(&self, base: String, skip: usize) -> String {
        let taken = |name: &str| {
            self.cameras
                .iter()
                .enumerate()
                .any(|(index, entry)| index != skip && entry.name == name)
        };
        if !taken(&base) {
            return base;
        }
        let mut suffix = 2;
        loop {
            let candidate = format!("{base}.{suffix}");
            if !taken(&candidate) {
                return candidate;
            }
            suffix += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glam::Vec3;

    fn camera_at(x: f32) -> Camera {
        Camera::at(Vec3::new(x, 0.0, 0.0))
    }

    #[test]
    fn first_spawned_camera_becomes_active() {
        let mut rig = CameraRig::empty();
        let id = rig.spawn("player", camera_at(1.0));
        assert_eq!(rig.active_id(), Some(id));
        assert_eq!(rig.active_name(), Some("player"));
    }

    #[test]
    fn spawn_generates_unique_names_and_ids() {
        let mut rig = CameraRig::empty();
        let first = rig.spawn("view", camera_at(0.0));
        let second = rig.spawn("view", camera_at(1.0));
        assert_ne!(first, second);
        assert_eq!(rig.get(first).unwrap().name, "view");
        assert_eq!(rig.get(second).unwrap().name, "view.2");
    }

    #[test]
    fn next_and_previous_wrap_around() {
        let mut rig = CameraRig::empty();
        let first = rig.spawn("a", camera_at(0.0));
        let second = rig.spawn("b", camera_at(1.0));
        assert_eq!(rig.select_next(), Some(second));
        assert_eq!(rig.select_next(), Some(first));
        assert_eq!(rig.select_previous(), Some(second));
    }

    #[test]
    fn selecting_by_name_and_id() {
        let mut rig = CameraRig::empty();
        let first = rig.spawn("a", camera_at(0.0));
        rig.spawn("b", camera_at(1.0));
        assert!(rig.select_name("b"));
        assert_eq!(rig.active_name(), Some("b"));
        assert!(rig.select(first));
        assert_eq!(rig.active_name(), Some("a"));
        assert!(!rig.select(CameraId::from_raw(999)));
    }

    #[test]
    fn removing_the_active_camera_selects_the_next_one() {
        let mut rig = CameraRig::empty();
        let first = rig.spawn("a", camera_at(0.0));
        let second = rig.spawn("b", camera_at(1.0));
        rig.spawn("c", camera_at(2.0));
        assert!(rig.select(second));
        assert!(rig.remove(second).is_some());
        assert_eq!(rig.active_id(), rig.iter().nth(1).map(|entry| entry.id));
        assert_eq!(rig.active_name(), Some("c"));
        assert!(rig.select(first));
    }

    #[test]
    fn the_last_camera_cannot_be_removed() {
        let mut rig = CameraRig::empty();
        let id = rig.spawn("only", camera_at(0.0));
        assert!(rig.remove(id).is_none());
        assert_eq!(rig.len(), 1);
    }

    #[test]
    fn renaming_avoids_duplicate_names() {
        let mut rig = CameraRig::empty();
        rig.spawn("a", camera_at(0.0));
        rig.spawn("b", camera_at(1.0));
        assert!(rig.select_name("b"));
        assert_eq!(rig.rename_active("a").as_deref(), Some("a.2"));
        assert_eq!(rig.active_name(), Some("a.2"));
    }
}
