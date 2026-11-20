//! Procedural 64x64 textures and sprites (no asset files). Pixels are
//! packed RGBA little-endian (r | g<<8 | b<<16 | a<<24), i.e. exactly the
//! byte order of a canvas ImageData, so the framebuffer can be blitted as-is.

pub const T: usize = 64;

#[inline]
pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16 | (a as u32) << 24
}

fn hash(x: u32, y: u32, s: u32) -> u32 {
    let mut h = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ s.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

fn noise(x: u32, y: u32, s: u32) -> f32 {
    (hash(x, y, s) & 0xffff) as f32 / 65535.0
}

fn shade(c: [f32; 3], k: f32) -> u32 {
    let f = |v: f32| (v * k).clamp(0.0, 255.0) as u8;
    rgba(f(c[0]), f(c[1]), f(c[2]), 255)
}

pub struct Textures {
    pub walls: Vec<Vec<u32>>, // index = wall id - 1
    pub floor: Vec<u32>,
    pub ceil: Vec<u32>,
    pub sprites: Vec<Vec<u32>>, // in SpriteKind order (alpha 0 = transparent)
    pub guns: Vec<Vec<u32>>,    // pistol, shotgun, rifle, muzzle flash
}

impl Textures {
    pub fn generate() -> Textures {
        let mut brick = vec![0; T * T];
        let mut stone = vec![0; T * T];
        let mut wood = vec![0; T * T];
        let mut metal = vec![0; T * T];
        let mut floor = vec![0; T * T];
        let mut ceil = vec![0; T * T];
        for y in 0..T {
            for x in 0..T {
                let (xu, yu) = (x as u32, y as u32);
                let n = noise(xu, yu, 1);
                // brick: 16x8 bricks, staggered rows, 1px mortar
                let row = y / 8;
                let bx = (x + if row % 2 == 1 { 8 } else { 0 }) % T;
                let mortar = y % 8 == 0 || bx % 16 == 0;
                let tint = noise((bx / 16) as u32, row as u32, 7) * 0.35 + 0.8;
                brick[y * T + x] = if mortar { shade([150.0, 140.0, 128.0], 0.75 + 0.2 * n) }
                    else { shade([150.0, 62.0, 44.0], tint * (0.85 + 0.25 * n)) };
                // stone: irregular 16px blocks with darker seams
                let sx = x / 16;
                let sy = (y + (sx % 2) * 8) / 16;
                let seam = x % 16 == 0 || (y + (sx % 2) * 8) % 16 == 0;
                let k = noise(sx as u32, sy as u32, 3) * 0.3 + 0.75 + 0.15 * n;
                stone[y * T + x] = if seam { shade([70.0, 72.0, 78.0], 0.8) } else { shade([128.0, 130.0, 136.0], k) };
                // wood: vertical planks with sinusoidal grain
                let plank = x / 13;
                let grain = ((y as f32 * 0.35 + (plank as f32) * 3.1 + (x % 13) as f32 * 0.9).sin() * 0.5 + 0.5) * 0.25;
                wood[y * T + x] = if x % 13 == 0 { shade([60.0, 38.0, 20.0], 1.0) }
                    else { shade([132.0, 88.0, 50.0], 0.8 + grain + 0.1 * n) };
                // metal: 32px panels, rivets at corners, brushed noise
                let (px, py) = (x % 32, y % 32);
                let rivet = (px == 4 || px == 27) && (py == 4 || py == 27);
                let edge = px == 0 || py == 0;
                metal[y * T + x] = if rivet { shade([210.0, 214.0, 220.0], 1.0) }
                    else if edge { shade([55.0, 60.0, 68.0], 1.0) }
                    else { shade([96.0, 104.0, 116.0], 0.85 + 0.25 * noise(xu, yu / 3, 9)) };
                // floor: 32px stone tiles; ceiling: dark boards
                let tile = (x / 32 + y / 32) % 2;
                floor[y * T + x] = if x % 32 == 0 || y % 32 == 0 { shade([50.0, 48.0, 46.0], 1.0) }
                    else { shade(if tile == 0 { [96.0, 90.0, 82.0] } else { [84.0, 80.0, 74.0] }, 0.85 + 0.25 * n) };
                ceil[y * T + x] = if y % 16 == 0 { shade([20.0, 18.0, 16.0], 1.0) }
                    else { shade([58.0, 50.0, 42.0], 0.8 + 0.3 * noise(xu / 8, yu, 5)) };
            }
        }
        Textures { walls: vec![brick, stone, wood, metal], floor, ceil, sprites: vec![
            barrel(), pillar(), lamp(), exit(),
            ammo(), shells(), medkit(), gun_pickup(false), gun_pickup(true),
            zombie(0), zombie(1), corpse(),
        ],
            guns: vec![view_gun(0), view_gun(1), view_gun(2), flash()] }
    }
}

fn barrel() -> Vec<u32> {
    let mut s = vec![0; T * T];
    for y in 18..T {
        for x in 16..48 {
            let u = (x as f32 - 31.5) / 16.0; // -1..1 across the barrel
            let light = (1.0 - u * u).sqrt() * 0.75 + 0.25; // cylinder shading
            let band = y == 24 || y == 25 || y == 44 || y == 45 || y == 58 || y == 59;
            let c = if band { [70.0, 70.0, 76.0] } else { [120.0, 72.0, 36.0] };
            s[y * T + x] = shade(c, light * (0.9 + 0.2 * noise(x as u32, y as u32 / 2, 4)));
        }
    }
    s
}

fn pillar() -> Vec<u32> {
    let mut s = vec![0; T * T];
    for y in 0..T {
        let (x0, x1) = if y < 6 || y > 57 { (18, 46) } else { (22, 42) };
        for x in x0..x1 {
            let u = (x as f32 - 31.5) / ((x1 - x0) as f32 / 2.0);
            let light = (1.0 - u * u).max(0.0).sqrt() * 0.7 + 0.3;
            let flute = if (x - x0) % 5 == 0 && y >= 6 && y <= 57 { 0.8 } else { 1.0 };
            s[y * T + x] = shade([170.0, 166.0, 156.0], light * flute);
        }
    }
    s
}

fn lamp() -> Vec<u32> {
    let mut s = vec![0; T * T];
    for y in 20..T {
        for x in 30..34 {
            s[y * T + x] = shade([60.0, 60.0, 64.0], 1.0);
        }
    }
    for y in 4..22 {
        for x in 20..44 {
            let d = ((x as f32 - 31.5).powi(2) + (y as f32 - 12.5).powi(2)).sqrt();
            if d < 9.5 {
                let k = 1.3 - d / 12.0;
                s[y * T + x] = shade([255.0, 210.0, 140.0], k.max(0.6));
            }
        }
    }
    s
}

fn exit() -> Vec<u32> {
    let mut s = vec![0; T * T];
    for y in 0..T {
        for x in 0..T {
            let d = ((x as f32 - 31.5).powi(2) + ((y as f32 - 34.0) * 0.8).powi(2)).sqrt();
            if (16.0..26.0).contains(&d) {
                let swirl = ((y as f32 - 34.0).atan2(x as f32 - 31.5) * 4.0 + d * 0.5).sin() * 0.25 + 0.85;
                s[y * T + x] = shade([90.0, 255.0, 140.0], swirl);
            } else if d < 16.0 {
                s[y * T + x] = shade([30.0, 120.0, 60.0], 0.6 + 0.4 * noise(x as u32, y as u32, 11));
            }
        }
    }
    s
}

// ---------------------------------------------------------------- deadmaze sprites
fn rect(s: &mut [u32], x0: usize, y0: usize, x1: usize, y1: usize, c: [f32; 3], seed: u32) {
    for y in y0..y1.min(T) {
        for x in x0..x1.min(T) {
            s[y * T + x] = shade(c, 0.85 + 0.25 * noise(x as u32, y as u32, seed));
        }
    }
}

fn ammo() -> Vec<u32> {
    let mut s = vec![0; T * T];
    rect(&mut s, 20, 48, 44, 63, [92.0, 98.0, 60.0], 21);   // olive crate
    rect(&mut s, 20, 48, 44, 50, [130.0, 138.0, 90.0], 22);
    for i in 0..4 {
        rect(&mut s, 23 + i * 5, 40, 26 + i * 5, 48, [214.0, 170.0, 70.0], 23); // brass tips
    }
    s
}

fn shells() -> Vec<u32> {
    let mut s = vec![0; T * T];
    for i in 0..4 {
        let x = 22 + i * 5;
        rect(&mut s, x, 46, x + 4, 58, [190.0, 40.0, 36.0], 24);
        rect(&mut s, x, 58, x + 4, 62, [200.0, 170.0, 80.0], 25);
    }
    s
}

fn medkit() -> Vec<u32> {
    let mut s = vec![0; T * T];
    rect(&mut s, 18, 44, 46, 63, [225.0, 225.0, 220.0], 26);
    rect(&mut s, 29, 47, 35, 60, [210.0, 30.0, 30.0], 27);
    rect(&mut s, 24, 51, 40, 56, [210.0, 30.0, 30.0], 27);
    s
}

fn gun_pickup(rifle: bool) -> Vec<u32> {
    let mut s = vec![0; T * T];
    if rifle {
        rect(&mut s, 10, 52, 54, 56, [40.0, 42.0, 46.0], 28);
        rect(&mut s, 30, 56, 36, 62, [36.0, 36.0, 40.0], 29); // magazine
        rect(&mut s, 10, 51, 22, 58, [70.0, 50.0, 32.0], 30); // stock
    } else {
        rect(&mut s, 12, 52, 52, 55, [52.0, 54.0, 60.0], 28);
        rect(&mut s, 12, 55, 52, 57, [44.0, 46.0, 50.0], 28);
        rect(&mut s, 12, 52, 26, 60, [110.0, 70.0, 40.0], 30); // wooden stock
    }
    s
}

/// Front view of a shambling zombie; frame 0/1 swap the legs and arms.
fn zombie(frame: usize) -> Vec<u32> {
    let mut s = vec![0; T * T];
    let skin = [104.0, 132.0, 92.0];
    let shirt = [92.0, 78.0, 64.0];
    let pants = [52.0, 58.0, 74.0];
    // head
    for y in 10..22 {
        for x in 26..38 {
            let d = ((x as f32 - 31.5) / 6.0).powi(2) + ((y as f32 - 15.5) / 6.5).powi(2);
            if d < 1.0 {
                s[y * T + x] = shade(skin, 0.75 + 0.35 * noise(x as u32, y as u32, 31) - d * 0.2);
            }
        }
    }
    // glowing eyes and a mouth
    for (x, y) in [(29, 15), (30, 15), (33, 15), (34, 15)] {
        s[y * T + x] = rgba(255, 70, 40, 255);
    }
    rect(&mut s, 29, 19, 35, 20, [40.0, 20.0, 18.0], 32);
    // torso, torn shirt
    for y in 22..42 {
        for x in 23..41 {
            let torn = noise(x as u32 / 2, y as u32 / 3, 33) > 0.82;
            s[y * T + x] = shade(if torn { skin } else { shirt }, 0.75 + 0.3 * noise(x as u32, y as u32, 34));
        }
    }
    // a dark stain
    for y in 28..36 {
        for x in 27..33 {
            if noise(x as u32, y as u32, 35) > 0.4 {
                s[y * T + x] = shade([90.0, 18.0, 16.0], 0.9);
            }
        }
    }
    // arms reaching forward (seen head-on: raised, slightly apart)
    let lift = if frame == 0 { 0 } else { 3 };
    rect(&mut s, 17, 24 + lift, 23, 34 + lift, shirt, 36);
    rect(&mut s, 41, 27 - lift, 47, 37 - lift, shirt, 36);
    rect(&mut s, 17, 34 + lift, 23, 39 + lift, skin, 37);
    rect(&mut s, 41, 37 - lift, 47, 42 - lift, skin, 37);
    // legs: one forward, one back
    let (l0, l1) = if frame == 0 { (0, 3) } else { (3, 0) };
    rect(&mut s, 25, 42, 31, 62 - l0, pants, 38);
    rect(&mut s, 33, 42, 39, 62 - l1, pants, 38);
    rect(&mut s, 24, 61 - l0, 31, 64 - l0, [40.0, 34.0, 30.0], 39);
    rect(&mut s, 33, 61 - l1, 40, 64 - l1, [40.0, 34.0, 30.0], 39);
    s
}

fn corpse() -> Vec<u32> {
    let mut s = vec![0; T * T];
    for y in 52..64 {
        for x in 10..54 {
            let d = ((x as f32 - 32.0) / 22.0).powi(2) + ((y as f32 - 58.5) / 5.5).powi(2);
            if d < 1.0 {
                let pool = noise(x as u32 / 2, y as u32, 40) > 0.55;
                s[y * T + x] = shade(if pool { [110.0, 16.0, 14.0] } else { [92.0, 80.0, 66.0] }, 0.8 + 0.2 * noise(x as u32, y as u32, 41));
            }
        }
    }
    s
}

/// A rounded, shaded box: lit from the upper left like a metal part, with a
/// dark 1 px outline so it reads at any scale.
fn part(s: &mut [u32], x0: usize, y0: usize, x1: usize, y1: usize, c: [f32; 3], seed: u32) {
    let (w, h) = ((x1 - x0) as f32, (y1 - y0) as f32);
    for y in y0..y1.min(T) {
        for x in x0..x1.min(T) {
            let u = (x - x0) as f32 / w; // 0..1 across
            let v = (y - y0) as f32 / h;
            let edge = x == x0 || x + 1 == x1 || y == y0 || y + 1 == y1;
            let k = if edge { 0.35 } else {
                // cylinder-ish highlight at 30% across, darker toward the bottom
                let hl = 1.0 - ((u - 0.3) * 2.2).powi(2);
                0.55 + 0.55 * hl.max(0.0) - v * 0.15 + 0.08 * noise(x as u32, y as u32, seed)
            };
            s[y * T + x] = shade(c, k);
        }
    }
}

/// First-person gun, seen from behind the hands, barrel up the middle.
fn view_gun(kind: usize) -> Vec<u32> {
    let mut s = vec![0; T * T];
    let steel = [88.0, 92.0, 102.0];
    let dark = [46.0, 48.0, 56.0];
    let wood = [128.0, 82.0, 46.0];
    let skin = [196.0, 146.0, 112.0];
    let sleeve = [58.0, 66.0, 52.0];
    let hole = rgba(6, 6, 8, 255);
    match kind {
        0 => {
            part(&mut s, 26, 24, 38, 44, steel, 50);    // slide
            part(&mut s, 30, 21, 34, 25, dark, 51);     // front sight
            for y in 26..29 { for x in 30..34 { s[y * T + x] = hole; } } // muzzle
            part(&mut s, 28, 34, 36, 37, dark, 52);     // ejection port
            part(&mut s, 25, 42, 39, 56, dark, 53);     // frame and grip
            part(&mut s, 20, 46, 30, 64, skin, 54);     // left hand
            part(&mut s, 34, 44, 44, 64, skin, 55);     // right hand, thumb over
            part(&mut s, 18, 58, 46, 64, sleeve, 56);
        }
        1 => {
            part(&mut s, 23, 8, 31, 40, steel, 50);     // two barrels
            part(&mut s, 33, 8, 41, 40, steel, 50);
            for y in 10..14 { for x in 25..29 { s[y * T + x] = hole; } for x in 35..39 { s[y * T + x] = hole; } }
            part(&mut s, 31, 9, 33, 38, dark, 51);      // rib
            part(&mut s, 20, 30, 44, 44, wood, 55);     // fore-end
            part(&mut s, 14, 34, 24, 50, skin, 54);     // hand under the fore-end
            part(&mut s, 22, 44, 42, 58, dark, 52);     // receiver
            part(&mut s, 30, 50, 46, 64, skin, 57);     // trigger hand
            part(&mut s, 10, 50, 22, 64, sleeve, 56);
            part(&mut s, 40, 58, 54, 64, sleeve, 56);
        }
        _ => {
            part(&mut s, 29, 4, 35, 34, dark, 50);      // barrel
            for y in 5..8 { for x in 31..33 { s[y * T + x] = hole; } }
            part(&mut s, 27, 16, 37, 22, dark, 53);     // front sight block
            part(&mut s, 25, 26, 39, 48, steel, 51);    // receiver
            part(&mut s, 28, 28, 36, 32, dark, 52);     // scope rail
            part(&mut s, 23, 32, 31, 46, skin, 54);     // support hand
            part(&mut s, 29, 44, 37, 58, dark, 53);     // magazine
            part(&mut s, 35, 46, 45, 64, skin, 57);     // trigger hand
            part(&mut s, 16, 54, 30, 64, sleeve, 56);
        }
    }
    s
}

fn flash() -> Vec<u32> {
    let mut s = vec![0; T * T];
    for y in 0..T {
        for x in 0..T {
            let (dx, dy) = (x as f32 - 31.5, y as f32 - 34.0);
            let a = dy.atan2(dx);
            let spikes = 0.7 + 0.3 * (a * 7.0).sin().abs();
            let d = (dx * dx + dy * dy).sqrt() / (20.0 * spikes);
            if d < 1.0 {
                let k = 1.0 - d;
                s[y * T + x] = rgba(255, (200.0 + 55.0 * k) as u8, (90.0 + 150.0 * k * k) as u8, 255);
            }
        }
    }
    s
}
