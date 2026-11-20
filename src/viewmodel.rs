//! First-person arms and guns as real low-poly 3D meshes, rasterized on the
//! CPU with flat shading and a depth buffer: the PS2 look. Everything is built
//! from tapered boxes at startup, then posed every frame (bob, sway, recoil,
//! weapon switch) and drawn over the 3D view.

use crate::tex::rgba;

#[derive(Clone, Copy)]
pub struct Tri {
    pub v: [[f32; 3]; 3],
    pub color: [f32; 3],
}

pub type Mesh = Vec<Tri>;

/// A box centred at `c` with half-size `h`, its front (+z) face scaled by
/// `taper`, rotated by `pitch` (around x) then `yaw` (around y).
fn part(m: &mut Mesh, c: [f32; 3], h: [f32; 3], taper: f32, pitch: f32, yaw: f32, color: [f32; 3]) {
    let mut p = [[0.0f32; 3]; 8];
    for (i, q) in p.iter_mut().enumerate() {
        let sx = if i & 1 == 0 { -1.0 } else { 1.0 };
        let sy = if i & 2 == 0 { -1.0 } else { 1.0 };
        let sz = if i & 4 == 0 { -1.0 } else { 1.0 };
        let k = if sz > 0.0 { taper } else { 1.0 };
        let v = [sx * h[0] * k, sy * h[1] * k, sz * h[2]];
        let v = rot_x(v, pitch);
        let v = rot_y(v, yaw);
        *q = [v[0] + c[0], v[1] + c[1], v[2] + c[2]];
    }
    // 6 faces, 2 triangles each, wound so the normal points out
    const F: [[usize; 4]; 6] = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
    for f in F {
        m.push(Tri { v: [p[f[0]], p[f[1]], p[f[2]]], color });
        m.push(Tri { v: [p[f[0]], p[f[2]], p[f[3]]], color });
    }
}

#[inline]
fn rot_x(v: [f32; 3], a: f32) -> [f32; 3] {
    let (s, c) = a.sin_cos();
    [v[0], v[1] * c - v[2] * s, v[1] * s + v[2] * c]
}
#[inline]
fn rot_y(v: [f32; 3], a: f32) -> [f32; 3] {
    let (s, c) = a.sin_cos();
    [v[0] * c + v[2] * s, v[1], -v[0] * s + v[2] * c]
}
#[inline]
fn rot_z(v: [f32; 3], a: f32) -> [f32; 3] {
    let (s, c) = a.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]]
}

const SKIN: [f32; 3] = [196.0, 142.0, 108.0];
const KNUCKLE: [f32; 3] = [178.0, 124.0, 94.0];
const SLEEVE: [f32; 3] = [62.0, 72.0, 54.0];
const CUFF: [f32; 3] = [44.0, 50.0, 40.0];
const STEEL: [f32; 3] = [96.0, 100.0, 110.0];
const DARK: [f32; 3] = [42.0, 44.0, 50.0];
const WOOD: [f32; 3] = [126.0, 80.0, 44.0];

/// A hand closed around a grip at `c` (the grip runs along y), fingers
/// wrapping toward -x for the right hand (side = 1) or +x for the left.
fn hand(m: &mut Mesh, c: [f32; 3], side: f32, tilt: f32) {
    // palm on the outside of the grip
    part(m, [c[0] + 0.022 * side, c[1], c[2] - 0.004], [0.012, 0.034, 0.026], 1.0, tilt, 0.0, SKIN);
    // four fingers wrapped round the front, stacked down the grip
    for i in 0..4 {
        let y = c[1] + 0.024 - i as f32 * 0.016;
        part(m, [c[0] - 0.004 * side, y, c[2] + 0.024], [0.022, 0.007, 0.009], 0.95, tilt, 0.0, if i % 2 == 0 { SKIN } else { KNUCKLE });
    }
    // thumb over the top, pointing forward
    part(m, [c[0] - 0.014 * side, c[1] + 0.036, c[2] + 0.004], [0.008, 0.008, 0.024], 0.8, tilt - 0.3, 0.2 * side, SKIN);
    // wrist and forearm back toward the bottom corner of the screen
    part(m, [c[0] + 0.018 * side, c[1] - 0.052, c[2] - 0.05], [0.026, 0.026, 0.04], 1.0, 0.75, 0.0, CUFF);
    part(m, [c[0] + 0.04 * side, c[1] - 0.12, c[2] - 0.16], [0.036, 0.036, 0.12], 0.9, 0.55, -0.25 * side, SLEEVE);
}

/// Gun meshes, in gun space: x right, y up, z forward (muzzle at +z).
pub fn build(kind: usize) -> (Mesh, [f32; 3]) {
    let mut m = Mesh::new();
    let muzzle;
    match kind {
        0 => {
            // pistol: slide, frame, grip, sights
            part(&mut m, [0.0, 0.03, 0.06], [0.016, 0.018, 0.085], 1.0, 0.0, 0.0, STEEL);
            part(&mut m, [0.0, 0.052, 0.13], [0.004, 0.006, 0.004], 1.0, 0.0, 0.0, DARK);
            part(&mut m, [0.0, 0.052, -0.012], [0.01, 0.006, 0.005], 1.0, 0.0, 0.0, DARK);
            part(&mut m, [0.0, 0.008, 0.05], [0.014, 0.01, 0.07], 1.0, 0.0, 0.0, DARK);
            part(&mut m, [0.0, -0.04, 0.0], [0.014, 0.045, 0.02], 1.0, -0.25, 0.0, DARK);
            part(&mut m, [0.0, -0.01, 0.04], [0.003, 0.014, 0.014], 1.0, 0.0, 0.0, DARK); // trigger guard
            hand(&mut m, [0.0, -0.035, 0.0], 1.0, -0.25);
            // support hand cupped underneath from the left
            part(&mut m, [-0.03, -0.05, 0.01], [0.016, 0.03, 0.03], 1.0, -0.2, 0.0, SKIN);
            part(&mut m, [-0.06, -0.12, -0.12], [0.034, 0.034, 0.11], 0.9, 0.55, 0.3, SLEEVE);
            muzzle = [0.0, 0.03, 0.15];
        }
        1 => {
            // shotgun: two barrels, rib, wooden fore-end and stock
            part(&mut m, [-0.012, 0.03, 0.22], [0.011, 0.011, 0.22], 1.0, 0.0, 0.0, STEEL);
            part(&mut m, [0.012, 0.03, 0.22], [0.011, 0.011, 0.22], 1.0, 0.0, 0.0, STEEL);
            part(&mut m, [0.0, 0.043, 0.22], [0.004, 0.003, 0.21], 1.0, 0.0, 0.0, DARK);
            part(&mut m, [0.0, 0.012, 0.2], [0.026, 0.018, 0.08], 1.0, 0.0, 0.0, WOOD);
            part(&mut m, [0.0, 0.02, 0.02], [0.022, 0.024, 0.05], 1.0, 0.0, 0.0, DARK);
            part(&mut m, [0.0, -0.02, -0.06], [0.02, 0.04, 0.07], 1.0, -0.35, 0.0, WOOD);
            hand(&mut m, [0.0, -0.03, -0.02], 1.0, -0.35);
            // left hand under the fore-end
            part(&mut m, [-0.004, -0.008, 0.2], [0.028, 0.012, 0.03], 1.0, 0.0, 0.0, SKIN);
            for i in 0..3 {
                part(&mut m, [-0.03, 0.012, 0.184 + i as f32 * 0.016], [0.006, 0.016, 0.007], 1.0, 0.0, 0.0, KNUCKLE);
            }
            part(&mut m, [-0.05, -0.08, 0.06], [0.034, 0.034, 0.14], 0.9, 0.45, 0.35, SLEEVE);
            muzzle = [0.0, 0.03, 0.44];
        }
        _ => {
            // rifle: long barrel, receiver, magazine, front sight
            part(&mut m, [0.0, 0.034, 0.26], [0.007, 0.007, 0.2], 1.0, 0.0, 0.0, DARK);
            part(&mut m, [0.0, 0.03, 0.06], [0.02, 0.026, 0.11], 1.0, 0.0, 0.0, STEEL);
            part(&mut m, [0.0, 0.062, 0.05], [0.006, 0.006, 0.07], 1.0, 0.0, 0.0, DARK);
            part(&mut m, [0.0, 0.06, 0.3], [0.004, 0.018, 0.005], 1.0, 0.0, 0.0, DARK);
            part(&mut m, [0.0, -0.03, 0.1], [0.012, 0.045, 0.018], 1.0, 0.25, 0.0, DARK);
            part(&mut m, [0.0, 0.03, 0.2], [0.018, 0.018, 0.06], 1.0, 0.0, 0.0, DARK);
            hand(&mut m, [0.0, -0.025, -0.02], 1.0, -0.3);
            part(&mut m, [-0.01, 0.006, 0.2], [0.026, 0.012, 0.03], 1.0, 0.0, 0.0, SKIN);
            for i in 0..3 {
                part(&mut m, [-0.026, 0.026, 0.186 + i as f32 * 0.015], [0.006, 0.014, 0.006], 1.0, 0.0, 0.0, KNUCKLE);
            }
            part(&mut m, [-0.05, -0.08, 0.06], [0.034, 0.034, 0.14], 0.9, 0.45, 0.35, SLEEVE);
            muzzle = [0.0, 0.034, 0.47];
        }
    }
    (m, muzzle)
}

/// Pose: where the gun sits in view space and how it is rotated.
#[derive(Clone, Copy, Default)]
pub struct Pose {
    pub offset: [f32; 3],
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

impl Pose {
    #[inline]
    pub fn apply(&self, v: [f32; 3]) -> [f32; 3] {
        let v = rot_x(v, -self.pitch); // positive pitch = muzzle up
        let v = rot_y(v, self.yaw);
        let v = rot_z(v, self.roll);
        [v[0] + self.offset[0], v[1] + self.offset[1], v[2] + self.offset[2]]
    }
}

/// Rasterize `mesh` posed by `pose` into `fb` (w x h) with its own depth
/// buffer. `light` scales the colours (the torch at the player's position).
/// Returns the screen position of `muzzle`.
pub fn draw(fb: &mut [u32], depth: &mut Vec<f32>, w: usize, h: usize, mesh: &Mesh, muzzle: [f32; 3], pose: &Pose, light: [f32; 3]) -> (f32, f32) {
    depth.clear();
    depth.resize(w * h, f32::INFINITY);
    let f = h as f32 * 1.05; // focal length in pixels
    let (cx, cy) = (w as f32 * 0.5, h as f32 * 0.5);
    let project = |v: [f32; 3]| -> [f32; 3] { [cx + v[0] / v[2] * f, cy - v[1] / v[2] * f, v[2]] };
    // light from above, a little left and toward the camera
    let ld = { let l = [-0.35f32, 0.8, -0.5]; let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt(); [l[0] / n, l[1] / n, l[2] / n] };
    for t in mesh {
        let a = pose.apply(t.v[0]);
        let b = pose.apply(t.v[1]);
        let c = pose.apply(t.v[2]);
        if a[2] < 0.02 || b[2] < 0.02 || c[2] < 0.02 {
            continue;
        }
        // flat shading from the face normal
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let mut n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
        n = [n[0] / nl, n[1] / nl, n[2] / nl];
        // back-face cull: the camera is at the origin
        if n[0] * a[0] + n[1] * a[1] + n[2] * a[2] >= 0.0 {
            continue;
        }
        let lambert = (n[0] * ld[0] + n[1] * ld[1] + n[2] * ld[2]).max(0.0);
        let k = 0.38 + 0.75 * lambert;
        let col = rgba(
            (t.color[0] * k * light[0]).min(255.0) as u8,
            (t.color[1] * k * light[1]).min(255.0) as u8,
            (t.color[2] * k * light[2]).min(255.0) as u8,
            255,
        );
        let (p0, p1, p2) = (project(a), project(b), project(c));
        raster(fb, depth, w, h, p0, p1, p2, col);
    }
    let m = project(pose.apply(muzzle));
    (m[0], m[1])
}

fn raster(fb: &mut [u32], depth: &mut [f32], w: usize, h: usize, a: [f32; 3], b: [f32; 3], c: [f32; 3], col: u32) {
    let minx = a[0].min(b[0]).min(c[0]).floor().max(0.0) as i32;
    let maxx = a[0].max(b[0]).max(c[0]).ceil().min(w as f32 - 1.0) as i32;
    let miny = a[1].min(b[1]).min(c[1]).floor().max(0.0) as i32;
    let maxy = a[1].max(b[1]).max(c[1]).ceil().min(h as f32 - 1.0) as i32;
    let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if area.abs() < 1e-6 || minx > maxx || miny > maxy {
        return;
    }
    let inv = 1.0 / area;
    for y in miny..=maxy {
        let py = y as f32 + 0.5;
        for x in minx..=maxx {
            let px = x as f32 + 0.5;
            let w0 = ((b[0] - px) * (c[1] - py) - (b[1] - py) * (c[0] - px)) * inv;
            let w1 = ((c[0] - px) * (a[1] - py) - (c[1] - py) * (a[0] - px)) * inv;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let z = w0 * a[2] + w1 * b[2] + w2 * c[2];
            let i = y as usize * w + x as usize;
            if z < depth[i] {
                depth[i] = z;
                fb[i] = col;
            }
        }
    }
}
