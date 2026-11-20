//! deadmaze: an endless zombie FPS on a from-scratch software raycaster, no dependencies.
//!
//! The same engine compiles natively (tests, benchmarks, PPM snapshots) and
//! to `wasm32-unknown-unknown`, where a tiny `extern "C"` API exposes the
//! framebuffer to a browser canvas. No wasm-bindgen.

pub mod engine;
pub mod map;
pub mod play;
pub mod ray;
pub mod tex;

use engine::Game;
use map::Map;

// ---------------------------------------------------------------- C ABI (used by the wasm build)
// wasm32 is single-threaded; a single global game instance is the simplest
// FFI shape. Every entry point checks for initialization.
static mut GAME: Option<Game> = None;

#[allow(static_mut_refs)]
fn game() -> Option<&'static mut Game> {
    // SAFETY: wasm32-unknown-unknown has one thread and these functions are
    // never re-entered; natively they are only used from tests on one thread.
    unsafe { GAME.as_mut() }
}

/// Create a maze level (cells_w x cells_h maze cells) and a w x h framebuffer.
#[no_mangle]
pub extern "C" fn rc_init(seed: u32, w: u32, h: u32, cells_w: u32, cells_h: u32) {
    let map = Map::maze(cells_w.clamp(3, 64) as usize, cells_h.clamp(3, 64) as usize, seed);
    // SAFETY: see game()
    let mut g = Game::new(map, w.max(8) as usize, h.max(8) as usize);
    g.seed_play(seed);
    // SAFETY: see game()
    unsafe { GAME = Some(g) }
}

#[no_mangle]
pub extern "C" fn rc_resize(w: u32, h: u32) {
    if let Some(g) = game() {
        g.resize(w as usize, h as usize);
    }
}

/// Advance `dt` seconds with input bits (see engine::FWD..RUN) and render.
/// Returns 1 once the player reached the exit.
#[no_mangle]
pub extern "C" fn rc_frame(dt: f32, input: u32, mouse_dx: f32) -> u32 {
    match game() {
        Some(g) => {
            g.update(dt, input, mouse_dx);
            g.render();
            g.won as u32
        }
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn rc_fb() -> *const u32 {
    game().map_or(std::ptr::null(), |g| g.fb.as_ptr())
}

/// 0 x, 1 y, 2 angle, 3 rays, 4 dda steps, 5 sprites drawn, 6 lights, 7 width, 8 height, 9 time
#[no_mangle]
pub extern "C" fn rc_stat(i: u32) -> f32 {
    let Some(g) = game() else { return 0.0 };
    match i {
        0 => g.x,
        1 => g.y,
        2 => g.angle,
        3 => g.stats.rays as f32,
        4 => g.stats.dda_steps as f32,
        5 => g.stats.sprites_drawn as f32,
        6 => g.stats.lights_active as f32,
        7 => g.w as f32,
        8 => g.h as f32,
        9 => g.time,
        10 => g.dm.hp,
        11 => g.dm.gun as f32,
        12 => g.ammo() as f32,
        13 => g.dm.wave as f32,
        14 => g.dm.score as f32,
        15 => g.alive() as f32 + g.dm.to_spawn as f32,
        16 => g.dm.dead as u32 as f32,
        17 => g.dm.kills as f32,
        18 => (g.dm.owned[0] as u32 | (g.dm.owned[1] as u32) << 1 | (g.dm.owned[2] as u32) << 2) as f32,
        19 => g.dm.pickup as f32,
        20 => g.dm.pickup_n as f32,
        21 => g.dm.rest_t,
        _ => 0.0,
    }
}

/// Debug and screenshots: put a zombie `dist` units in front of the player.
#[no_mangle]
pub extern "C" fn rc_debug_spawn(dist: f32) {
    if let Some(g) = game() {
        let (x, y) = (g.x + g.angle.cos() * dist, g.y + g.angle.sin() * dist);
        g.dm.zombies.push(play::Zombie { x, y, hp: 1000.0, speed: 0.0, flash: 0.0, attack: 99.0, step: 0.0, dead_t: 0.0 });
    }
}

/// Switch weapon (0 pistol, 1 shotgun, 2 rifle) if owned.
#[no_mangle]
pub extern "C" fn rc_weapon(i: u32) {
    if let Some(g) = game() {
        g.select(i as usize);
    }
}

/// Toggles: 0 minimap, 1 lighting, 2 fog, 3 torch, 4 textures.
#[no_mangle]
pub extern "C" fn rc_option(k: u32, v: u32) {
    if let Some(g) = game() {
        let on = v != 0;
        match k {
            0 => g.opt.minimap = on,
            1 => g.opt.lighting = on,
            2 => g.opt.fog = on,
            3 => g.opt.torch = on,
            4 => g.opt.textures = on,
            _ => {}
        }
    }
}

/// Write the framebuffer as a binary PPM (P6). Native-only helper.
pub fn to_ppm(fb: &[u32], w: usize, h: usize) -> Vec<u8> {
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    out.reserve(w * h * 3);
    for &p in fb {
        out.extend_from_slice(&[(p & 0xff) as u8, (p >> 8 & 0xff) as u8, (p >> 16 & 0xff) as u8]);
    }
    out
}
