use nalgebra::vector;

type Vector2 = nalgebra::Vector2<f32>;
type Vector4 = nalgebra::Vector4<f32>;
type Mat4 = nalgebra::Matrix4<f32>;
#[derive(Copy, Clone)]
pub struct Camera {
    center: Vector2,
    extent: Vector2,
    near: f32,
    far: f32,
    zoom: f32,
}
impl Camera {
    pub fn new(center: Vector2, extent: Vector2) -> Self {
        Self {
            center,
            extent,
            near: 100.0,
            far: -100.0,
            zoom: 1.0,
        }
    }

    // note that the z axis points outwards
    pub fn pv_matrix(&self) -> Mat4 {
        ortho_projection(self.center, self.extent, self.near, self.far, self.zoom)
    }

    pub fn set_screen_extent(&mut self, ext: Vector2) {
        self.extent = ext;
    }

    pub fn screen_pt_to_world(&self, screen_pos: Vector2) -> Vector2 {
        let ndc_x = screen_pos[0] / self.extent[0] * 2.0 - 1.0;
        let ndc_y = screen_pos[1] / self.extent[1] * 2.0 - 1.0;
        Vector2::new(
            ndc_x * self.extent[0] / (self.zoom * 2.0) + self.center[0],
            ndc_y * self.extent[1] / (self.zoom * 2.0) + self.center[1],
        )
    }

    pub fn screen_vec_to_world(&self, screen_vec: Vector2) -> Vector2 {
        screen_vec / self.zoom
    }

    pub fn world_point_to_screen(&self, pt: Vector2, screen_size: Vector2) -> Vector2 {
        let padded = Vector4::new(pt[0], pt[1], 0.0, 1.0);
        let pv = self.pv_matrix();
        let transformed = pv * padded;
        let w_divided = Vector2::new(transformed[0], transformed[1]) / transformed[3];

        ((w_divided + vector![1.0, 1.0]) * 0.5).component_mul(&screen_size)
    }
}

pub fn ortho_projection(center: Vector2, extent: Vector2, near: f32, far: f32, zoom: f32) -> Mat4 {
    let view = Mat4::new(
        1.0, 0.0, 0.0, -center.x,
        0.0, 1.0, 0.0, -center.y,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );

    let ex = extent.x / zoom;
    let ey = extent.y / zoom;
    let n = near;
    let f = far;

    let projection = Mat4::new(
        2.0 / ex,    0.0,      0.0,           0.0,
        0.0,         2.0 / ey, 0.0,           0.0,
        0.0,         0.0,      1.0 / (f - n), n / (n - f),
        0.0,         0.0,      0.0,           1.0,
    );

    projection * view
}
