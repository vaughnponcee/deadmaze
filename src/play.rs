//! The game on top of the renderer: endless zombie waves, three guns, loot,
//! and the first-person gun drawn over the view.
//!
//! Zombies find you with a flow field: a breadth-first search outward from
//! the player's cell, redone only when you step into a new cell. Each zombie
//! walks downhill on it, so a hundred zombies cost one BFS, not a hundred.
//! Shots are hitscan: a ray against the wall grid (DDA) and against each
//! zombie's circle; the nearest hit wins.

use crate::engine::{Game, FIRE, PLAYER_RADIUS};
use crate::map::{Rng, SpriteKind};
use crate::ray::cast;
use crate::tex::{rgba, T};
use crate::viewmodel::{self, Mesh, Pose};

pub const PISTOL: usize = 0;
pub const SHOTGUN: usize = 1;
pub const RIFLE: usize = 2;

pub struct Gun {
    pub cooldown: f32, // seconds between shots
    pub damage: f32,
    pub pellets: u32,
    pub spread: f32,   // radians, total
    pub auto: bool,    // holds to keep firing
    pub kick: f32,     // view-model recoil
}

pub const GUNS: [Gun; 3] = [
    Gun { cooldown: 0.28, damage: 38.0, pellets: 1, spread: 0.0, auto: false, kick: 0.6 },
    Gun { cooldown: 0.85, damage: 16.0, pellets: 9, spread: 0.20, auto: false, kick: 1.0 },
    Gun { cooldown: 0.085, damage: 22.0, pellets: 1, spread: 0.025, auto: true, kick: 0.35 },
];

#[derive(Clone, Copy, Debug)]
pub struct Zombie {
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub speed: f32,
    pub flash: f32,   // hit flash, fades
    pub attack: f32,  // attack cooldown
    pub step: f32,    // walk cycle
    pub dead_t: f32,  // seconds since death (corpses fade after a while)
}

#[derive(Clone, Copy, Debug)]
pub struct Loot {
    pub x: f32,
    pub y: f32,
    pub kind: SpriteKind,
}

pub struct Dm {
    pub hp: f32,
    pub gun: usize,
    pub owned: [bool; 3],
    pub bullets: u32, // rifle (the pistol never runs dry, so you can't get stuck)
    pub shells: u32,
    pub wave: u32,
    pub to_spawn: u32,
    pub spawn_t: f32,
    pub rest_t: f32,  // break between waves
    pub score: u32,
    pub kills: u32,
    pub dead: bool,
    pub cool: f32,
    pub muzzle: f32,
    pub kick: f32,
    pub hurt: f32,
    pub bob: f32,
    pub fire_held: bool,
    pub zombies: Vec<Zombie>,
    pub loot: Vec<Loot>,
    pub flow: Vec<u16>,
    pub flow_cell: (i32, i32),
    pub pickup: u32,       // which loot was picked up last (SpriteKind as u32)
    pub pickup_n: u32,     // increments on every pickup, for the HUD
    pub rng: Rng,
    // view model
    pub sway: f32,
    pub switch_t: f32, // 1 -> 0 while the old gun goes down and the new one comes up
    pub prev_gun: usize,
    pub last_gun: usize,
    pub vm: Vec<(Mesh, [f32; 3])>,
    pub vdepth: Vec<f32>,
    /// blood on the floor: BLOOD_RES texels per map cell, 0 = clean
    pub blood: Vec<u8>,
    /// bodies lying on the floor, same grid: 0 none, 1 skin, 2 shirt, 3 trousers, 4 shoes
    pub body: Vec<u8>,
    pub blood_w: usize,
}

pub const BLOOD_RES: usize = 16;

impl Default for Dm {
    fn default() -> Dm {
        Dm {
            hp: 100.0, gun: PISTOL, owned: [true, false, false], bullets: 0, shells: 0,
            wave: 0, to_spawn: 0, spawn_t: 0.0, rest_t: 2.0, score: 0, kills: 0, dead: false,
            cool: 0.0, muzzle: 0.0, kick: 0.0, hurt: 0.0, bob: 0.0, fire_held: false,
            zombies: Vec::with_capacity(64), loot: Vec::with_capacity(32),
            flow: vec![], flow_cell: (-1, -1), pickup: 0, pickup_n: 0, rng: Rng::new(12345),
            sway: 0.0, switch_t: 0.0, prev_gun: 0, last_gun: 0,
            vm: (0..3).map(viewmodel::build).collect(), vdepth: vec![], blood: vec![], body: vec![], blood_w: 0,
        }
    }
}

const ZOMBIE_R: f32 = 0.24;
const MAX_ALIVE: usize = 28;

impl Game {
    pub fn seed_play(&mut self, seed: u32) {
        let bw = self.map.w * BLOOD_RES;
        self.dm = Dm { rng: Rng::new(seed ^ 0x9e37_79b9), blood: vec![0; bw * self.map.h * BLOOD_RES], body: vec![0; bw * self.map.h * BLOOD_RES], blood_w: bw, ..Dm::default() };
    }

    pub fn alive(&self) -> usize {
        self.dm.zombies.iter().filter(|z| z.hp > 0.0).count()
    }

    pub fn select(&mut self, gun: usize) {
        if gun < 3 && self.dm.owned[gun] {
            self.dm.gun = gun;
        }
    }

    pub fn ammo(&self) -> i32 {
        match self.dm.gun {
            SHOTGUN => self.dm.shells as i32,
            RIFLE => self.dm.bullets as i32,
            _ => -1, // unlimited
        }
    }

    /// Called from update() after the player has moved.
    pub(crate) fn play(&mut self, dt: f32, input: u32, mouse_dx: f32) {
        let d = &mut self.dm;
        // view model: lag behind the mouse, then settle
        d.sway += ((-mouse_dx * 0.004).clamp(-0.12, 0.12) - d.sway) * (dt * 10.0).min(1.0);
        if d.gun != d.last_gun {
            d.prev_gun = d.last_gun;
            d.last_gun = d.gun;
            d.switch_t = 1.0;
        }
        d.switch_t = (d.switch_t - dt / 0.4).max(0.0);
        d.cool -= dt;
        d.muzzle = (d.muzzle - dt).max(0.0);
        d.kick = (d.kick - dt * 5.0).max(0.0);
        d.hurt = (d.hurt - dt * 1.5).max(0.0);
        if d.dead {
            return;
        }
        if input & 15 != 0 {
            d.bob += dt * if input & crate::engine::RUN != 0 { 13.0 } else { 9.0 };
        }
        self.waves(dt);
        self.update_flow();
        self.move_zombies(dt);
        self.shoot(input);
        self.collect();
    }

    // ---------------------------------------------------------------- waves
    fn waves(&mut self, dt: f32) {
        let alive = self.alive();
        let d = &mut self.dm;
        if d.to_spawn == 0 && alive == 0 {
            d.rest_t -= dt;
            if d.rest_t <= 0.0 {
                d.wave += 1;
                d.to_spawn = 5 + d.wave * 3;
                d.rest_t = 4.0;
                // a new gun shows up on waves 2 and 4
                if d.wave == 2 || d.wave == 4 {
                    let kind = if d.wave == 2 { SpriteKind::Shotgun } else { SpriteKind::Rifle };
                    if let Some((x, y)) = self.open_cell(3.0, 9.0) {
                        self.dm.loot.push(Loot { x, y, kind });
                    }
                }
            }
            return;
        }
        d.spawn_t -= dt;
        if d.to_spawn > 0 && d.spawn_t <= 0.0 && alive < MAX_ALIVE {
            let wave = d.wave as f32;
            if let Some((x, y)) = self.open_cell(9.0, 1e9) {
                let d = &mut self.dm;
                let speed = (1.05 + wave * 0.09).min(2.3) * (0.85 + d.rng.f32() * 0.3);
                d.zombies.push(Zombie { x, y, hp: 55.0 + wave * 9.0, speed, flash: 0.0, attack: 0.6, step: d.rng.f32() * 6.0, dead_t: 0.0 });
                d.to_spawn -= 1;
                d.spawn_t = (1.1 - wave * 0.05).max(0.25);
            }
        }
    }

    /// A random open cell at a distance between `near` and `far` from the player.
    fn open_cell(&mut self, near: f32, far: f32) -> Option<(f32, f32)> {
        for _ in 0..200 {
            let x = self.dm.rng.below(self.map.w as u32) as i32;
            let y = self.dm.rng.below(self.map.h as u32) as i32;
            if self.map.get(x, y) != 0 {
                continue;
            }
            let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
            let dist = ((cx - self.x).powi(2) + (cy - self.y).powi(2)).sqrt();
            if dist >= near && dist <= far && !self.blocked(cx, cy, ZOMBIE_R) {
                return Some((cx, cy));
            }
        }
        None
    }

    // ---------------------------------------------------------------- zombies
    /// BFS distance (in cells) from the player to every open cell.
    fn update_flow(&mut self) {
        let cell = (self.x.floor() as i32, self.y.floor() as i32);
        let (w, h) = (self.map.w, self.map.h);
        if cell == self.dm.flow_cell && self.dm.flow.len() == w * h {
            return;
        }
        self.dm.flow_cell = cell;
        let flow = &mut self.dm.flow;
        flow.clear();
        flow.resize(w * h, u16::MAX);
        let mut queue = std::collections::VecDeque::with_capacity(w * h);
        flow[cell.1 as usize * w + cell.0 as usize] = 0;
        queue.push_back(cell);
        while let Some((x, y)) = queue.pop_front() {
            let dist = flow[y as usize * w + x as usize];
            for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if self.map.get(nx, ny) == 0 {
                    let i = ny as usize * w + nx as usize;
                    if flow[i] == u16::MAX {
                        flow[i] = dist + 1;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
    }

    fn flow_at(&self, x: i32, y: i32) -> u16 {
        if self.map.get(x, y) != 0 {
            return u16::MAX;
        }
        self.dm.flow[y as usize * self.map.w + x as usize]
    }

    fn move_zombies(&mut self, dt: f32) {
        let (px, py) = (self.x, self.y);
        let n = self.dm.zombies.len();
        for i in 0..n {
            let mut z = self.dm.zombies[i];
            z.flash = (z.flash - dt * 6.0).max(0.0);
            if z.hp <= 0.0 {
                z.dead_t += dt;
                self.dm.zombies[i] = z;
                continue;
            }
            let (cx, cy) = (z.x.floor() as i32, z.y.floor() as i32);
            let here = self.flow_at(cx, cy);
            let to_player = ((px - z.x).powi(2) + (py - z.y).powi(2)).sqrt();
            // close by (or same cell): go straight at you. Otherwise follow the
            // flow field downhill to the neighbouring cell nearest the player.
            let (tx, ty) = if here <= 1 || to_player < 1.5 {
                (px, py)
            } else {
                let mut best = (here, cx, cy);
                for (nx, ny) in [(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)] {
                    let f = self.flow_at(nx, ny);
                    if f < best.0 {
                        best = (f, nx, ny);
                    }
                }
                (best.1 as f32 + 0.5, best.2 as f32 + 0.5)
            };
            let (mut dx, mut dy) = (tx - z.x, ty - z.y);
            let len = (dx * dx + dy * dy).sqrt();
            if to_player > PLAYER_RADIUS + ZOMBIE_R + 0.2 && len > 1e-3 {
                // stagger a little while hurt
                let sp = z.speed * if z.flash > 0.0 { 0.35 } else { 1.0 } * dt;
                dx = dx / len * sp;
                dy = dy / len * sp;
                // keep apart from other zombies
                for (j, o) in self.dm.zombies.iter().enumerate() {
                    if j != i && o.hp > 0.0 {
                        let (ox, oy) = (z.x - o.x, z.y - o.y);
                        let d2 = ox * ox + oy * oy;
                        if d2 < 0.36 && d2 > 1e-6 {
                            let push = (0.6 - d2.sqrt()) * 1.5 * dt;
                            let dd = d2.sqrt();
                            dx += ox / dd * push;
                            dy += oy / dd * push;
                        }
                    }
                }
                if !self.blocked(z.x + dx, z.y, ZOMBIE_R) { z.x += dx; }
                if !self.blocked(z.x, z.y + dy, ZOMBIE_R) { z.y += dy; }
                z.step += dt * z.speed * 4.0;
            }
            // bite
            z.attack -= dt;
            if to_player < PLAYER_RADIUS + ZOMBIE_R + 0.4 && z.attack <= 0.0 {
                z.attack = 0.9;
                let dmg = 8.0 + self.dm.wave as f32 * 0.8;
                self.dm.hp -= dmg;
                self.dm.hurt = 1.0;
                if self.dm.hp <= 0.0 {
                    self.dm.hp = 0.0;
                    self.dm.dead = true;
                }
            }
            self.dm.zombies[i] = z;
        }
        // corpses sink away after 10 s
        // the dead are painted flat onto the floor (see paint_body), not drawn as sprites
        self.dm.zombies.retain(|z| z.hp > 0.0);
    }

    // ---------------------------------------------------------------- shooting
    fn shoot(&mut self, input: u32) {
        let fire = input & FIRE != 0;
        let gun = &GUNS[self.dm.gun];
        let pressed = fire && (gun.auto || !self.dm.fire_held);
        self.dm.fire_held = fire;
        if !pressed || self.dm.cool > 0.0 || self.dm.switch_t > 0.35 {
            return;
        }
        match self.dm.gun {
            SHOTGUN if self.dm.shells == 0 => { self.dm.gun = PISTOL; return; }
            RIFLE if self.dm.bullets == 0 => { self.dm.gun = PISTOL; return; }
            SHOTGUN => self.dm.shells -= 1,
            RIFLE => self.dm.bullets -= 1,
            _ => {}
        }
        self.dm.cool = gun.cooldown;
        self.dm.muzzle = 0.045;
        self.dm.kick = gun.kick;
        for p in 0..gun.pellets {
            let off = if gun.pellets > 1 {
                (p as f32 / (gun.pellets - 1) as f32 - 0.5) * gun.spread + (self.dm.rng.f32() - 0.5) * 0.03
            } else {
                (self.dm.rng.f32() - 0.5) * gun.spread
            };
            let a = self.angle + off;
            if let Some((i, t)) = self.trace(a.cos(), a.sin()) {
                let falloff = if t > 7.0 { 0.5 } else { 1.0 };
                self.damage(i, gun.damage * falloff, a);
            }
        }
    }

    /// Nearest zombie hit by a ray from the player, in front of any wall.
    pub fn trace(&self, dx: f32, dy: f32) -> Option<(usize, f32)> {
        let wall = cast(&self.map, self.x, self.y, dx, dy, 512).map_or(f32::INFINITY, |h| h.dist);
        let mut best: Option<(usize, f32)> = None;
        for (i, z) in self.dm.zombies.iter().enumerate() {
            if z.hp <= 0.0 {
                continue;
            }
            // ray-circle: closest approach along the ray, then back off to the surface
            let (ox, oy) = (z.x - self.x, z.y - self.y);
            let t = ox * dx + oy * dy;
            let perp2 = ox * ox + oy * oy - t * t;
            let r = ZOMBIE_R + 0.06;
            if t > 0.0 && perp2 < r * r {
                let th = t - (r * r - perp2).sqrt();
                if th < wall && best.map_or(true, |b| th < b.1) {
                    best = Some((i, th));
                }
            }
        }
        best
    }

    fn damage(&mut self, i: usize, dmg: f32, angle: f32) {
        let z = &mut self.dm.zombies[i];
        if z.hp <= 0.0 {
            return;
        }
        z.hp -= dmg;
        z.flash = 1.0;
        let (zx0, zy0) = (z.x, z.y);
        let r = 0.08 + self.dm.rng.f32() * 0.06;
        self.splat(zx0 + angle.cos() * 0.25, zy0 + angle.sin() * 0.25, r, 150);
        // knockback
        let (kx, ky) = (angle.cos() * 0.06, angle.sin() * 0.06);
        let (zx, zy) = (zx0 + kx, zy0 + ky);
        if !self.blocked(zx, zy, ZOMBIE_R) {
            let z = &mut self.dm.zombies[i];
            z.x = zx;
            z.y = zy;
        }
        let z = self.dm.zombies[i];
        if z.hp <= 0.0 {
            self.paint_body(z.x, z.y, angle);
            // a pool where it fell: a few overlapping blobs, not a circle
            for _ in 0..5 {
                let (ox, oy) = ((self.dm.rng.f32() - 0.5) * 0.5, (self.dm.rng.f32() - 0.5) * 0.5);
                let r = 0.14 + self.dm.rng.f32() * 0.16;
                self.splat(z.x + ox, z.y + oy, r, 230);
            }
            self.dm.kills += 1;
            self.dm.score += 100 + self.dm.wave * 10;
            // drops
            let r = self.dm.rng.below(100);
            let kind = match r {
                0..=21 => Some(SpriteKind::Ammo),
                22..=35 => Some(SpriteKind::Shells),
                36..=47 => Some(SpriteKind::Medkit),
                _ => None,
            };
            if let Some(kind) = kind {
                self.dm.loot.push(Loot { x: z.x, y: z.y, kind });
            }
        }
    }

    fn collect(&mut self) {
        let (px, py) = (self.x, self.y);
        let mut i = 0;
        while i < self.dm.loot.len() {
            let l = self.dm.loot[i];
            if (l.x - px).powi(2) + (l.y - py).powi(2) > 0.5 * 0.5 {
                i += 1;
                continue;
            }
            let d = &mut self.dm;
            let took = match l.kind {
                SpriteKind::Ammo => { d.bullets = (d.bullets + 40).min(400); true }
                SpriteKind::Shells => { d.shells = (d.shells + 8).min(60); true }
                SpriteKind::Medkit if d.hp < 100.0 => { d.hp = (d.hp + 30.0).min(100.0); true }
                SpriteKind::Shotgun => { d.owned[SHOTGUN] = true; d.shells += 12; d.gun = SHOTGUN; true }
                SpriteKind::Rifle => { d.owned[RIFLE] = true; d.bullets += 90; d.gun = RIFLE; true }
                _ => false,
            };
            if took {
                d.pickup = l.kind as u32;
                d.pickup_n += 1;
                d.score += 10;
                d.loot.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    // ---------------------------------------------------------------- drawing
    /// Everything the sprite pass draws: map props, loot, zombies.
    pub(crate) fn collect_sprites(&mut self) {
        self.draw.clear();
        for s in &self.map.sprites {
            self.draw.push((s.x, s.y, s.kind as usize, 0.0));
        }
        for l in &self.dm.loot {
            self.draw.push((l.x, l.y, l.kind as usize, 0.0));
        }
        for z in self.dm.zombies.iter().filter(|z| z.hp > 0.0) {
            let tex = if (z.step as i32) % 2 == 0 {
                SpriteKind::ZombieA
            } else {
                SpriteKind::ZombieB
            };
            self.draw.push((z.x, z.y, tex as usize, z.flash));
        }
    }

    /// Gun view model, muzzle flash and the hurt tint, drawn over the 3D view.
    pub(crate) fn overlay(&mut self) {
        let (w, h) = (self.w as f32, self.h as f32);
        let _ = w;
        let d = &self.dm;
        let t = self.time;
        // pose: walk bob, idle breathing, mouse sway, recoil, weapon switch
        let bob_x = d.bob.sin() * 0.008;
        let bob_y = (d.bob * 2.0).sin().abs() * 0.007 + (t * 1.7).sin() * 0.002;
        let drop = if d.switch_t > 0.0 { (std::f32::consts::PI * (1.0 - d.switch_t)).sin() * 0.16 } else { 0.0 };
        let shown = if d.switch_t > 0.5 { d.prev_gun } else { d.gun };
        let kick = d.kick.min(1.0);
        let pose = Pose {
            offset: [0.1 + bob_x - d.sway * 0.1, -0.12 - bob_y - drop - kick * 0.01, 0.36 - kick * 0.04],
            pitch: kick * 0.2 - drop * 1.2,
            yaw: -0.06 - d.sway,
            roll: d.sway * 0.6 + d.bob.sin() * 0.015,
        };
        let l = self.light_at(self.x, self.y);
        let l = [l[0].clamp(0.5, 1.25), l[1].clamp(0.46, 1.2), l[2].clamp(0.44, 1.15)];
        let (mesh, muzzle) = std::mem::take(&mut self.dm.vm[shown]);
        let mut depth = std::mem::take(&mut self.dm.vdepth);
        let (mx, my) = viewmodel::draw(&mut self.fb, &mut depth, self.w, self.h, &mesh, muzzle, &pose, l);
        self.dm.vm[shown] = (mesh, muzzle);
        self.dm.vdepth = depth;
        if self.dm.muzzle > 0.0 {
            let size = h * if shown == 1 { 0.22 } else { 0.15 };
            self.blit_tex(self.tex.guns.len() - 1, mx - size * 0.5, my - size * 0.55, size, [1.4, 1.4, 1.4]);
        }
        if self.dm.hurt > 0.0 {
            // red tint, stronger toward the edges
            let (wi, hi) = (self.w, self.h);
            let k = self.dm.hurt * 0.55;
            for y in 0..hi {
                let vy = (y as f32 / hi as f32 - 0.5) * 2.0;
                for x in 0..wi {
                    let vx = (x as f32 / wi as f32 - 0.5) * 2.0;
                    let e = ((vx * vx + vy * vy) * 0.6).min(1.0) * k;
                    let c = self.fb[y * wi + x];
                    let r = (c & 0xff) as f32;
                    let g = ((c >> 8) & 0xff) as f32 * (1.0 - e);
                    let b = ((c >> 16) & 0xff) as f32 * (1.0 - e);
                    self.fb[y * wi + x] = rgba((r + (200.0 - r) * e * 0.6) as u8, g as u8, b as u8, 255);
                }
            }
        }
    }

    /// Paint blood into the floor decal grid.
    pub fn splat(&mut self, x: f32, y: f32, r: f32, amount: u8) {
        let d = &mut self.dm;
        if d.blood.is_empty() {
            return;
        }
        let res = BLOOD_RES as f32;
        let (bw, bh) = (d.blood_w as i32, (d.blood.len() / d.blood_w) as i32);
        let (cx, cy, rr) = (x * res, y * res, r * res);
        for ty in (cy - rr - 1.0) as i32..=(cy + rr + 1.0) as i32 {
            for tx in (cx - rr - 1.0) as i32..=(cx + rr + 1.0) as i32 {
                if tx < 0 || ty < 0 || tx >= bw || ty >= bh {
                    continue;
                }
                let dd = ((tx as f32 + 0.5 - cx).powi(2) + (ty as f32 + 0.5 - cy).powi(2)).sqrt() / rr.max(0.5);
                // ragged edge: noise decides the last bit of the radius
                let edge = 0.7 + 0.3 * d.rng.f32();
                if dd < edge {
                    let i = (ty * bw + tx) as usize;
                    d.blood[i] = d.blood[i].max((amount as f32 * (1.0 - dd * 0.4)) as u8);
                }
            }
        }
    }

    /// Paint a zombie lying flat on the floor, head toward where the shot
    /// pushed it. Built in body space: u runs feet (-) to head (+), v across.
    pub fn paint_body(&mut self, x: f32, y: f32, angle: f32) {
        let d = &mut self.dm;
        if d.body.is_empty() {
            return;
        }
        let res = BLOOD_RES as f32;
        let (bw, bh) = (d.blood_w as i32, (d.body.len() / d.blood_w) as i32);
        // fall roughly along the shot, with some variety
        let a = angle + (d.rng.f32() - 0.5) * 0.8;
        let (ca, sa) = (a.cos(), a.sin());
        let arm = (d.rng.f32() - 0.5) * 0.3; // arms flung a bit differently each time
        let r = 0.6;
        for ty in ((y - r) * res) as i32..=((y + r) * res) as i32 {
            for tx in ((x - r) * res) as i32..=((x + r) * res) as i32 {
                if tx < 0 || ty < 0 || tx >= bw || ty >= bh {
                    continue;
                }
                let (px, py) = ((tx as f32 + 0.5) / res - x, (ty as f32 + 0.5) / res - y);
                let u = px * ca + py * sa; // along the body
                let v = -px * sa + py * ca; // across
                let part = if (u - 0.36).powi(2) + v * v < 0.085 * 0.085 {
                    1 // head
                } else if (-0.02..0.28).contains(&u) && v.abs() < 0.12 {
                    2 // torso
                } else if (0.05..0.34).contains(&(u - (v.abs() - 0.12) * arm)) && (0.12..0.19).contains(&v.abs()) {
                    if u > 0.26 { 1 } else { 2 } // arms, hands at the end
                } else if (-0.42..-0.02).contains(&u) && (0.015..0.1).contains(&v.abs()) {
                    if u < -0.36 { 4 } else { 3 } // legs, shoes
                } else {
                    0
                };
                if part != 0 {
                    d.body[(ty * bw + tx) as usize] = part;
                }
            }
        }
    }

    /// Floor texel with any blood on it mixed in.
    #[inline]
    pub fn blood_at(&self, texel: u32, fx: f32, fy: f32) -> u32 {
        let d = &self.dm;
        if d.blood.is_empty() || fx < 0.0 || fy < 0.0 {
            return texel;
        }
        let (tx, ty) = ((fx * BLOOD_RES as f32) as usize, (fy * BLOOD_RES as f32) as usize);
        if tx >= d.blood_w {
            return texel;
        }
        let i = ty * d.blood_w + tx;
        let Some(&b) = d.blood.get(i) else { return texel };
        let part = d.body[i];
        if part != 0 {
            // a little per-texel grit so the body isn't flat colour
            let n = 0.85 + 0.3 * ((i as u32).wrapping_mul(2654435761) >> 24) as f32 / 255.0;
            let c = match part { 1 => [104.0, 132.0, 92.0], 2 => [92.0, 78.0, 64.0], 3 => [52.0, 58.0, 74.0], _ => [40.0, 34.0, 30.0] };
            // soaked in blood where the pool is
            let k = b as f32 / 255.0 * 0.45;
            let f = |v: f32, to: f32| ((v * n) * (1.0 - k) + to * k).min(255.0) as u8;
            return rgba(f(c[0], 96.0), f(c[1], 8.0), f(c[2], 6.0), 255);
        }
        if b == 0 {
            return texel;
        }
        let k = b as f32 / 255.0 * 0.85;
        let mix = |c: u32, to: f32| ((c & 0xff) as f32 * (1.0 - k) + to * k) as u8;
        rgba(mix(texel, 96.0), mix(texel >> 8, 8.0), mix(texel >> 16, 6.0), 255)
    }

    fn blit_tex(&mut self, idx: usize, x0: f32, y0: f32, size: f32, l: [f32; 3]) {
        let (w, h) = (self.w as i32, self.h as i32);
        let xa = x0.max(0.0) as i32;
        let xb = ((x0 + size) as i32).min(w);
        let ya = y0.max(0.0) as i32;
        let yb = ((y0 + size) as i32).min(h);
        for y in ya..yb {
            let ty = (((y as f32 - y0) / size * T as f32) as usize).min(T - 1);
            for x in xa..xb {
                let tx = (((x as f32 - x0) / size * T as f32) as usize).min(T - 1);
                let texel = self.tex.guns[idx][ty * T + tx];
                if texel >> 24 == 0 {
                    continue;
                }
                let f = |v: u32, k: f32| ((v & 0xff) as f32 * k).min(255.0) as u8;
                self.fb[y as usize * self.w + x as usize] = rgba(f(texel, l[0]), f(texel >> 8, l[1]), f(texel >> 16, l[2]), 255);
            }
        }
    }
}
