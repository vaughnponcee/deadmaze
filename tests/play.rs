use deadmaze::engine::{Game, FIRE};
use deadmaze::map::{Map, SpriteKind};
use deadmaze::play::{Loot, Zombie, RIFLE, SHOTGUN};

const HALL: &str = "\
####################
#P.................#
#..................#
####################";

fn zombie(x: f32, y: f32, hp: f32) -> Zombie {
    Zombie { x, y, hp, speed: 1.5, flash: 0.0, attack: 0.5, step: 0.0, dead_t: 0.0 }
}

fn game() -> Game {
    let mut g = Game::new(Map::parse(HALL), 160, 90);
    g.seed_play(1);
    g.dm.rest_t = 1e9; // no waves unless a test wants them
    g
}

#[test]
fn pistol_shot_hits_the_zombie_in_front() {
    let mut g = game();
    g.dm.zombies.push(zombie(8.5, 1.5, 30.0));
    g.update(1.0 / 60.0, FIRE, 0.0);
    assert!(g.dm.zombies[0].hp <= 0.0, "hp {}", g.dm.zombies[0].hp);
    assert_eq!(g.dm.kills, 1);
}

#[test]
fn walls_stop_bullets() {
    let mut g = game();
    g.angle = std::f32::consts::FRAC_PI_2; // facing the wall below
    g.dm.zombies.push(zombie(1.5, 5.0, 30.0)); // behind it (outside the map)
    assert!(g.trace(0.0, 1.0).is_none());
}

#[test]
fn semi_auto_needs_a_new_click() {
    let mut g = game();
    g.dm.zombies.push(zombie(12.5, 1.5, 1000.0));
    for _ in 0..60 {
        g.update(1.0 / 60.0, FIRE, 0.0);
    }
    // held for a second: one pistol shot only
    assert!((g.dm.zombies[0].hp - 962.0).abs() < 1.0 || g.dm.zombies[0].hp > 960.0, "hp {}", g.dm.zombies[0].hp);
}

#[test]
fn zombies_walk_to_you_and_bite() {
    let mut g = game();
    g.dm.zombies.push(zombie(15.5, 2.5, 100.0));
    for _ in 0..60 * 12 {
        g.update(1.0 / 60.0, 0, 0.0);
    }
    assert!(g.dm.hp < 100.0, "never reached the player");
}

#[test]
fn waves_spawn_more_zombies_each_time() {
    let mut g = Game::new(Map::maze(8, 8, 3), 160, 90);
    g.seed_play(3);
    for _ in 0..60 * 3 {
        g.update(1.0 / 60.0, 0, 0.0);
    }
    assert_eq!(g.dm.wave, 1);
    let first = g.dm.to_spawn + g.alive() as u32;
    assert!(first >= 8);
    g.dm.zombies.clear();
    g.dm.to_spawn = 0;
    for _ in 0..60 * 5 {
        g.update(1.0 / 60.0, 0, 0.0);
    }
    assert_eq!(g.dm.wave, 2);
    assert!(g.dm.to_spawn + g.alive() as u32 > first);
}

#[test]
fn loot_is_picked_up_by_walking_over_it() {
    let mut g = game();
    g.dm.loot.push(Loot { x: 1.6, y: 1.5, kind: SpriteKind::Shotgun });
    g.update(1.0 / 60.0, 0, 0.0);
    assert!(g.dm.owned[SHOTGUN]);
    assert_eq!(g.dm.gun, SHOTGUN);
    assert!(g.dm.loot.is_empty());
}

#[test]
fn rifle_is_automatic_and_uses_ammo() {
    let mut g = game();
    g.dm.owned[RIFLE] = true;
    g.dm.bullets = 30;
    g.select(RIFLE);
    for _ in 0..60 {
        g.update(1.0 / 60.0, FIRE, 0.0);
    }
    assert!(g.dm.bullets < 22, "bullets left {}", g.dm.bullets);
}

#[test]
fn renders_with_zombies_on_screen() {
    let mut g = game();
    g.dm.zombies.push(zombie(4.5, 1.5, 100.0));
    g.render();
    assert!(g.stats.sprites_drawn >= 1);
}
