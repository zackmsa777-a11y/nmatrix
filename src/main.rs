use nmatrix::engine::{Cell, Palette, Scene};
use nmatrix::effects::Effects;
use nmatrix::options::{Controls, HELP, InputDecoder, Options};
use nmatrix::render::{ColorDepth, Renderer, blend_frames};
use nmatrix::terminal::Terminal;
use std::fmt::Write;
use std::io;
use std::time::{Duration, Instant};

struct Fade {
    frame: Vec<Cell>,
    age: f32,
}

fn run(options: Options) -> io::Result<i32> {
    if std::env::var("TERM").unwrap_or_default() == "dumb" {
        return Err(io::Error::other(
            "nmatrix needs an ANSI-compatible terminal",
        ));
    }
    let terminal = Terminal::open(0, 1)?;
    let depth = ColorDepth::detect();
    let mut renderer = Renderer::new(depth);
    let mut effects = Effects::default();
    let mut effect_dt = 0.0;
    let seed = options.seed;
    let interval = Duration::from_secs_f64(1.0 / options.fps as f64);
    let mut controls = Controls::new(options);
    let origin = Instant::now();
    let mut last = origin;
    let mut next_frame = origin;
    let mut input = InputDecoder::default();
    let (raw_width, raw_height) = terminal.size()?;
    let mut width = raw_width.clamp(1, 1024);
    let mut height = raw_height.clamp(1, 512);
    let mut scene = Scene::new(
        width,
        height.saturating_sub(1).max(1),
        controls.mode,
        controls.density,
        controls.glyphs,
        seed,
    );
    let mut fade: Option<Fade> = None;
    let mut palette_fade: Option<(Palette, f32)> = None;
    let mut demo_time = 0.0;
    let mut generation = 0u64;
    loop {
        let wait = next_frame
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(16));
        let bytes = terminal.read(wait)?;
        let now = Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;
        if Terminal::signal() != 0 {
            return Ok(128 + Terminal::signal());
        }
        if !controls.paused && !controls.help && !controls.picker {
            scene.update(dt * controls.speed);
            effect_dt += dt * controls.speed;
            if controls.demo {
                demo_time += dt;
            }
        }
        if let Some(f) = &mut fade {
            f.age += dt;
        }
        if let Some((_, age)) = &mut palette_fade {
            *age += dt;
        }
        let before = (
            controls.mode,
            controls.density,
            controls.glyphs,
            controls.palette,
            controls.hud,
            controls.help,
            controls.picker,
        );
        let mut dirty = false;
        input.push(&bytes, now - origin);
        for key in input.events(now - origin) {
            dirty |= controls.key(key);
            if !controls.running {
                return Ok(0);
            }
        }
        if dirty {
            demo_time = 0.0;
        }
        if controls.demo && demo_time >= 12.0 {
            controls.mode = controls.mode.next(1);
            controls.palette = controls.palette.next(1);
            demo_time = 0.0;
            dirty = true;
        }
        let (w, h) = terminal.size()?;
        let w = w.clamp(1, 1024);
        let h = h.clamp(1, 512);
        let resized = (w, h) != (width, height) || before.4 != controls.hud;
        let changed_scene = before.0 != controls.mode
            || before.1 != controls.density
            || before.2 != controls.glyphs;
        if changed_scene || resized {
            effects.reset();
            let old_frame = scene.frame().to_vec();
            width = w;
            height = h;
            generation += 1;
            scene = Scene::new(
                width,
                height.saturating_sub(usize::from(controls.hud)).max(1),
                controls.mode,
                controls.density,
                controls.glyphs,
                seed.wrapping_add(generation * 7919),
            );
            fade = if resized {
                None
            } else {
                Some(Fade {
                    frame: old_frame,
                    age: 0.0,
                })
            };
            renderer.invalidate();
            dirty = true;
        }
        if before.3 != controls.palette {
            palette_fade = Some((before.3, 0.0));
        }
        if before.5 != controls.help || before.6 != controls.picker {
            renderer.invalidate();
        }
        if dirty || now >= next_frame {
            let mut output = String::new();
            if width < 20 || height < 6 {
                if resized {
                    output.push_str("\x1b[2J");
                }
                output.push_str("\x1b[H\x1b[0m");
                output.push_str(&"Resize terminal / q quit"[..(width - 1).min(24)]);
                output.push_str("\x1b[K");
            } else {
                let scene_height = scene.height;
                let scene_time = scene.time;
                let target = scene.frame();
                let blended;
                let frame = if let Some(f) = &fade {
                    if f.age < 0.6 {
                        blended = blend_frames(&f.frame, target, f.age / 0.6);
                        &blended[..]
                    } else {
                        fade = None;
                        target
                    }
                } else {
                    target
                };
                let palette_from = palette_fade.and_then(|(from, age)| {
                    if age < 0.5 {
                        Some((from, age / 0.5))
                    } else {
                        None
                    }
                });
                if palette_from.is_none() {
                    palette_fade = None;
                }
                let frame = effects.apply(frame, width, effect_dt, scene_time, controls.effects);
                effect_dt = 0.0;
                output.push_str(&renderer.render(
                    &frame,
                    width,
                    scene_height,
                    controls.palette,
                    palette_from,
                ));
                if controls.hud {
                    hud(&mut output, width, height, depth, &controls);
                }
                if controls.help {
                    help_overlay(&mut output, width, scene.height, depth);
                }
                if controls.picker { picker_overlay(&mut output, width, scene.height, depth, &controls); }
            }
            terminal.write(&output)?;
            next_frame = now + interval;
        }
    }
}

fn hud(output: &mut String, width: usize, height: usize, depth: ColorDepth, c: &Controls) {
    let state = if c.paused {
        "PAUSED"
    } else if c.demo {
        "DEMO"
    } else {
        "LIVE"
    };
    let text = format!(
        " NMATRIX / {} / {} / {:.1}x / {:.1} density / {} / {}",
        c.mode.name().to_ascii_uppercase(),
        c.palette.name().to_ascii_uppercase(),
        c.speed,
        c.density,
        c.glyphs.name(),
        state
    );
    let hints = "  Tab scenes  c color  ? help  q quit ";
    let mut line = text;
    if c.effects.echo { line.push_str(" / ECHO"); }
    if c.effects.pulse { line.push_str(" / PULSE"); }
    if c.effects.scanlines { line.push_str(" / SCAN"); }
    if line.len() + hints.len() < width {
        line.push_str(&" ".repeat(width - 1 - line.len() - hints.len()));
        line.push_str(hints);
    }
    line.truncate(width - 1);
    write!(
        output,
        "\x1b[{height};1H{}{}{}\x1b[K",
        depth.background(),
        depth.foreground([114, 148, 162]),
        line
    )
    .unwrap();
}

fn help_overlay(output: &mut String, width: usize, height: usize, depth: ColorDepth) {
    let lines = [
        "+-------------------------------------------------------+",
        "|                  NMATRIX / CONTROLS                   |",
        "|                                                       |",
        "|  Tab       choose any of the twelve scenes             |",
        "|  1-9 / 0   quick scene selection                       |",
        "|  arrows/m  switch mode     c       color palette       |",
        "|  +/-       speed           [ ]     scene density       |",
        "|  g         glyph set       d       automatic demo      |",
        "|  e         echo trails     p       brightness pulse    |",
        "|  s         scanlines       r       surprise me         |",
        "|  n         name banner                                |",
        "|  space     pause           h       hide controls       |",
        "|  q / Esc / Ctrl+C          quit                       |",
        "|                                                       |",
        "|               ? closes this panel                     |",
        "+-------------------------------------------------------+",
    ];
    let box_width = lines[0].len().min(width - 2);
    let x = (width - box_width) / 2 + 1;
    let count = lines.len().min(height);
    let y = (height - count) / 2 + 1;
    for (offset, line) in lines.iter().take(count).enumerate() {
        write!(
            output,
            "\x1b[{};{}H{}{}{}",
            y + offset,
            x,
            depth.background(),
            depth.foreground([164, 211, 211]),
            &line[..box_width]
        )
        .unwrap();
    }
}

fn picker_overlay(output: &mut String, width: usize, height: usize, depth: ColorDepth, c: &Controls) {
    let modes = nmatrix::engine::Mode::ALL;
    let count = height.saturating_sub(2).min(modes.len()).max(1);
    let start = c.selection.saturating_sub(count/2).min(modes.len()-count);
    let box_width = 48.min(width-2);
    let x = (width-box_width)/2+1;
    let y = (height-count-2)/2+1;
    let mut lines = vec![" NMATRIX / SCENES".to_string()];
    for (index, mode) in modes.iter().enumerate().skip(start).take(count) {
        lines.push(format!(" {} {:02}  {}", if index == c.selection { ">" } else { " " }, index+1, mode.name().to_uppercase()));
    }
    lines.push(" Up/Down choose / Enter play / Esc close".into());
    for (row, mut line) in lines.into_iter().enumerate() {
        line.truncate(box_width);
        let highlight = row>0 && row<=count && start+row-1 == c.selection;
        let color = if highlight { [138, 255, 184] } else { [137, 170, 183] };
        write!(output, "\x1b[{};{}H{}{}{line:width$}", y+row, x, depth.background(), depth.foreground(color), width=box_width).unwrap();
    }
}

fn main() {
    let options = match Options::parse(std::env::args().skip(1).collect()) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("nmatrix: {error}");
            std::process::exit(2);
        }
    };
    if options.help {
        print!("{HELP}");
        return;
    }
    if options.version {
        println!("nmatrix {} / Rust", env!("CARGO_PKG_VERSION"));
        return;
    }
    match run(options) {
        Ok(status) => std::process::exit(status),
        Err(error) => {
            if Terminal::signal() != 0 {
                std::process::exit(128 + Terminal::signal());
            }
            eprintln!("nmatrix: {error}");
            std::process::exit(1);
        }
    }
}
