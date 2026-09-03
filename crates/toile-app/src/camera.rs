/// An orbit camera around the origin.
pub struct Camera {
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.7,
            pitch: 0.35,
            distance: 1.15,
        }
    }
}

impl Camera {
    /// Point the camera looks at, slightly above the origin so the garment
    /// sits in frame rather than the avatar's centre.
    const TARGET: [f32; 3] = [0.0, 0.02, 0.0];
    const FOV_Y: f32 = 55.0;

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * 0.01;
        // Stop short of the poles, where the up vector degenerates.
        self.pitch = (self.pitch + dy * 0.01).clamp(-1.4, 1.4);
    }

    pub fn zoom(&mut self, scroll: f32) {
        self.distance = (self.distance * (1.0 - scroll * 0.002)).clamp(0.3, 4.0);
    }

    /// Model-view-projection for the given aspect ratio.
    pub fn mvp(&self, aspect: f32) -> [f32; 16] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let eye = [
            Self::TARGET[0] + self.distance * cp * sy,
            Self::TARGET[1] + self.distance * sp,
            Self::TARGET[2] + self.distance * cp * cy,
        ];
        mul4(
            perspective(Self::FOV_Y.to_radians(), aspect, 0.02, 20.0),
            look_at(eye, Self::TARGET, [0.0, 1.0, 0.0]),
        )
    }
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn norm3(a: [f32; 3]) -> [f32; 3] {
    let l = dot3(a, a).sqrt().max(1.0e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}

#[rustfmt::skip]
fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
    let f = norm3(sub3(target, eye));
    let s = norm3(cross3(f, up));
    let u = cross3(s, f);
    [
        s[0], u[0], -f[0], 0.0,
        s[1], u[1], -f[1], 0.0,
        s[2], u[2], -f[2], 0.0,
        -dot3(s, eye), -dot3(u, eye), dot3(f, eye), 1.0,
    ]
}

/// Right-handed perspective with clip z in `[0, 1]`, wgpu's convention.
fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fov_y * 0.5).tan();
    let mut m = [0.0f32; 16];
    m[0] = f / aspect;
    m[5] = f;
    m[10] = far / (near - far);
    m[11] = -1.0;
    m[14] = near * far / (near - far);
    m
}

/// Column-major 4×4 product.
fn mul4(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut m = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut acc = 0.0;
            for k in 0..4 {
                acc += a[k * 4 + row] * b[col * 4 + k];
            }
            m[col * 4 + row] = acc;
        }
    }
    m
}
