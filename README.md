# The House

A static, walkable 3D entrance hall, rendered with [Bevy](https://bevy.org) compiled to WebAssembly. Frosted windows, a wind-driven snowstorm, snow-covered evergreens, diffused warm lighting, marked doors, interactive furnishings, and optional wind ambience.

## Requirements

- Rust (the version is pinned in `rust-toolchain.toml`; rustup installs it and the `wasm32-unknown-unknown` target automatically)
- `wasm-bindgen-cli` matching the `wasm-bindgen` crate version: `cargo install wasm-bindgen-cli --version 0.2.130`
- Node.js, then `npm install`
- A browser with WebGL 2

## Develop

```sh
npm run dev
```

This builds a debug wasm and starts Vite. Vite reloads the page when the TypeScript or CSS changes. After editing Rust, run `npm run wasm:dev` again.

## Build and deploy

```sh
npm run build    # optimized wasm, type check, bundle into dist/
npm run preview  # serve dist/ locally
npm run deploy   # build, then firebase deploy --only hosting
```

Firebase Hosting serves `dist/`.

## Explore

- Click **Step inside** to begin.
- Use **WASD** or **arrow keys** to walk and the mouse to look. If mouse capture is unavailable, drag to look.
- Face a nearby object and press **E** or click to interact. Try the table lamp or welcome note.
- Press **Esc** to pause. **How to explore** includes a reset button.
- On touchscreens, drag to look and use the on-screen movement and interaction buttons.
- Use **Sound off / on** to toggle ambient winter wind.

Only the entrance hall is built. The labeled doors identify future rooms and display a message when used.

## Layout

The Rust crate owns the 3D world; the web page owns everything that is DOM. They talk through a small set of wasm-bindgen exports.

- `crates/game/src/lib.rs`: app setup, camera, exposure and fog, and the exported `run` entry point
- `crates/game/src/bridge.rs`: commands from the page (input, interact, reset) and callbacks back to it (ready, target label, toast, error)
- `crates/game/src/hall.rs`: architecture, furnishings, doors, lighting, colliders and the lamp switch
- `crates/game/src/winter.rs`: forest, ground, drifts, and the billboarded snowstorm
- `crates/game/src/player.rs`: looking, walking with collision, view-ray targeting and interactions
- `crates/game/src/textures.rs`: canvas-painted textures (wood, rug, signs, needles, frost…) with mipmaps
- `crates/game/src/meshes.rs`: procedural meshes (boxes, cones, partial spheres) and mesh merging
- `web/index.html`, `web/src/style.css`: overlay interface
- `web/src/main.ts`: pointer lock and input forwarding, overlay state, and Web Audio wind

`web/src/wasm/` holds generated wasm-bindgen output and is not committed.

Fonts load from Google Fonts with local fallbacks. Everything else is bundled.
