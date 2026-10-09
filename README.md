# nmatrix 🟢

**A little terminal eye candy, built in Rust.**

Classic Matrix rain, colorful waves, spinning spirals, and a few other
ways to make your terminal look alive. Pick a vibe and let it run.

![Six nmatrix animation scenes](assets/preview.png)

## Pick your vibe

🌧️ **Rain** · 🌊 **Waterfall** · 〰️ **Waves** · 🌀 **Spiral** ·
⚡ **Glitch** · ✨ **Starfield**

Five palettes: **emerald, cyan, violet, amber, rainbow**. Smooth scene
transitions, glowing trails, and matrix / binary / hex glyphs.

```bash
nmatrix
nmatrix --demo --palette rainbow
nmatrix --mode spiral --palette violet
nmatrix --mode rain --glyphs binary --density 0.9
```

## Get it running

You'll need Linux and Rust installed. From this folder:

```bash
cargo build --release --locked
install -Dm755 target/release/nmatrix ~/.local/bin/nmatrix
nmatrix
```

Make sure `~/.local/bin` is in your PATH. You can also try it directly:

```bash
cargo run --release -- --demo --palette rainbow
```

## Play with it

| Key | What it does |
| --- | --- |
| `1`–`6`, left/right | Pick an animation |
| `c` | Change colors |
| `g` | Change glyphs |
| `[` / `]` | Less / more density |
| `+` / `-`, up/down | Faster / slower |
| Space | Pause |
| `d` | Auto-cycle scenes and colors |
| `h` | Hide the bottom bar |
| `?` | Show help |
| `q`, Escape, Ctrl+C | Quit |

Starts at 60 FPS. Demo changes scenes every 12 active seconds.
Use `nmatrix --help` for all the options.

## Tinker

```bash
cargo test --locked
```

20 tests cover the scenes, controls, resizing, and terminal cleanup.
Only one dependency: `libc`. No Python needed.

MIT licensed. Have fun with it.
