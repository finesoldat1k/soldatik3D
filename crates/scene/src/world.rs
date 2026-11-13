use crate::camera::Camera;
use crate::camera_rig::{CameraId, CameraRig, NamedCamera};
use crate::scene::Scene;

/// The runtime world owned by the engine.
///
/// A world combines the renderable scene (meshes, lights, material) with the
/// camera rig. Games create it with [`World::default`] or [`World::new`] and
/// receive it as `&mut World` every frame, so gameplay code can move cameras,
/// spawn new ones and select the active view without owning engine internals.
///
/// ```
/// use scene::{Camera, World};
/// use scene::glam::Vec3;
///
/// let mut world = World::default();
/// let camera = world.spawn_camera("overview", Camera::at(Vec3::Y * 12.0));
/// assert!(world.select_camera(camera));
/// assert_eq!(world.active_camera_name(), "overview");
/// ```
#[derive(Clone, Debug, Default)]
pub struct World {
    scene: Scene,
    cameras: CameraRig,
}

impl World {
    /// Creates a world around an existing scene and a default `main` camera.
    pub fn new(scene: Scene) -> Self {
        Self {
            scene,
            cameras: CameraRig::new(),
        }
    }

    /// Creates a world with explicit scene and camera rig. An empty rig is
    /// allowed for tools, but the runtime requires an active camera to render.
    pub fn with_cameras(scene: Scene, cameras: CameraRig) -> Self {
        Self { scene, cameras }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Mutable scene access.
    ///
    /// Geometry and material changes are *not* re-uploaded automatically: the
    /// renderer builds its GPU resources once from the scene returned by
    /// `Game::create_world`. Dynamic meshes are a planned backend extension.
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn cameras(&self) -> &CameraRig {
        &self.cameras
    }

    pub fn cameras_mut(&mut self) -> &mut CameraRig {
        &mut self.cameras
    }

    /// Spawns a camera and returns its stable ID.
    pub fn spawn_camera(&mut self, name: impl Into<String>, camera: Camera) -> CameraId {
        self.cameras.spawn(name, camera)
    }

    pub fn remove_camera(&mut self, id: CameraId) -> Option<NamedCamera> {
        self.cameras.remove(id)
    }

    pub fn select_camera(&mut self, id: CameraId) -> bool {
        self.cameras.select(id)
    }

    pub fn select_camera_name(&mut self, name: &str) -> bool {
        self.cameras.select_name(name)
    }

    pub fn select_next_camera(&mut self) -> Option<CameraId> {
        self.cameras.select_next()
    }

    pub fn select_previous_camera(&mut self) -> Option<CameraId> {
        self.cameras.select_previous()
    }

    /// Active camera, or a default camera if the rig is empty.
    pub fn active_camera(&self) -> Camera {
        if self.cameras.is_empty() {
            Camera::default()
        } else {
            self.cameras.active().camera
        }
    }

    pub fn active_camera_ref(&self) -> Option<&NamedCamera> {
        (!self.cameras.is_empty()).then(|| self.cameras.active())
    }

    pub fn active_camera_mut(&mut self) -> Option<&mut NamedCamera> {
        (!self.cameras.is_empty()).then(|| self.cameras.active_mut())
    }

    pub fn active_camera_name(&self) -> &str {
        self.cameras.active_name().unwrap_or("main")
    }

    pub fn active_camera_id(&self) -> Option<CameraId> {
        self.cameras.active_id()
    }

    /// View-projection matrix for the active camera.
    pub fn view_projection(&self, aspect: f32) -> glam::Mat4 {
        let camera = self.active_camera();
        camera.projection_matrix(aspect) * camera.view_matrix()
    }
}
