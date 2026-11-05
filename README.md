# The House

A static, walkable 3D entrance hall built with Three.js. Frosted windows, a wind-driven snowstorm, snow-covered evergreens, diffused warm lighting, marked doors, interactive furnishings, and optional wind ambience.

## Preview

From this directory, run:

```sh
python3 -m http.server 8080 --directory public
```

Open http://localhost:8080 in a modern browser with WebGL enabled. Serve the files over HTTP; JavaScript modules do not load reliably from a `file://` URL.

## Explore

- Click **Step inside** to begin.
- Use **WASD** or **arrow keys** to walk and the mouse to look. If mouse capture is unavailable, drag to look.
- Face a nearby object and press **E** or click to interact. Try the table lamp or welcome note.
- Press **Esc** to pause. **How to explore** includes a reset button.
- On touchscreens, drag to look and use the on-screen movement and interaction buttons.
- Use **Sound off / on** to toggle ambient winter wind.

Only the entrance hall is built. The labeled doors identify future rooms and display a message when used.

## Files

- `public/index.html`: entrance interface
- `public/entrance.css`: layout and controls
- `public/scripts/entrance.js`: scene, movement, collision, and interactions
- `public/scripts/winter.js`: textured evergreen forest, snowdrifts, and layered snow particles
- `public/vendor/`: bundled Three.js and its MIT license

The existing Firebase configuration serves `public/`. The earlier game and its assets remain available at `/game.html`. Fonts use Google Fonts with local fallbacks; 3D assets and rendering code are bundled locally.
