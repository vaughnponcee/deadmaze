# deadmaze

An endless zombie shooter that runs in the browser. Every pixel is drawn on the CPU by a software raycaster written in Rust, with no crates, no OpenGL and no wasm-bindgen. The engine is the one from [raycast-rs](https://github.com/vaughnponcee/raycast-rs); this repo adds the game.

**Play:** https://vaughnponcee.github.io/play/deadmaze/

## What's in it

- **Endless waves.** Each wave sends more zombies, and they get faster and tougher. A short break between waves.
- **Three guns.** Pistol (never runs dry), shotgun (9 pellets, shells) and an automatic rifle (bullets). The shotgun turns up on wave 2, the rifle on wave 4.
- **Loot.** Zombies drop ammo, shells and medkits. Walk over them to pick them up.
- **Zombie pathfinding.** A breadth-first search from the player's cell builds a flow field, recomputed only when you step into a new cell. Every zombie walks downhill on it, so 28 zombies cost one search, not 28. Up close they come straight at you, and they push apart so they don't stack.
- **Hitscan shooting.** Each shot is a ray tested against the wall grid (DDA) and against each zombie's circle. Walls stop bullets, the nearest hit wins, and the shotgun spreads its pellets over the cone.
- **Feel.** Hit flash and knockback on zombies, gun bob and recoil, a muzzle flash that lights the corridor, a red tint when you get bitten.
- **Touch controls** on phones and tablets: a stick on the left, drag to look, a fire button.

## Controls

`WASD` move, mouse aim, click shoot, `Shift` run, `1` `2` `3` or the wheel to switch guns, `M` map, `Esc` pause.

## Build

```
rustup target add wasm32-unknown-unknown
node tools/build-web.mjs && node tools/serve.mjs     # http://127.0.0.1:8070
cargo test
```

## Numbers

- **Frame time:** 5.1 ms per frame in WebAssembly at 480x270 (headless Edge, 300 frames with two zombies in view and shooting). The page scales that up with pixelated filtering.
- **Size:** the wasm module is 107 KiB. All textures are generated in code at startup.
- **Tests:** 26. They cover:
  - raycasting distances;
  - maze connectivity: every open cell is reachable, so zombies can always get to you;
  - hits, walls blocking bullets, semi-auto versus automatic fire;
  - zombies reaching and biting the player;
  - wave growth and loot pickup.

## Limits

- **No sound** yet.
- **Lights pass through walls:** there are no shadows.
- **Zombies always know where you are:** they follow the flow field and never need to see you.
- **Single-threaded** and no SIMD.
- **Art:** all of it is procedural pixel art made from rectangles and noise.

## License

MIT
