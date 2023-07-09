extern crate json;

use std::{path::PathBuf, io::BufReader, sync::Arc};

use audio::{SoundEffect, SoundEffectBank};
use debug::Debug;
use game::{Input, RenderState};
use player::Player;
use rodio::{OutputStream, Sink};
use sdl2::{image::{InitFlag}, keyboard::Keycode, sys::{SDL_Delay, SDL_GetTicks}};
use ui::Ui;
use world::World;

extern crate sdl2;

mod tiles;
mod texture;
mod player;
mod game;
mod world;
mod loader;
mod audio;
mod entity;
mod ai;
mod ui;
mod debug;
mod effect;

pub const START_MAP: &str = "res/maps/dev.tmx";
pub const DEBUG: bool = true;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG);
    let window = video_subsystem
        .window("yume", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string()).unwrap();

    let mut canvas = window
        .into_canvas()
        .software()
        .target_texture()
        //.present_vsync()
        .build()
        .map_err(|e| e.to_string()).unwrap();
    let texture_creator = canvas.texture_creator();

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut sfx = SoundEffectBank::new(Arc::new(stream_handle));

    // TODO uhhhhhhh
    // so rust thinks that the reference in line 37 is still being used here
    // idk how to fix that
    let mut ui = Ui::new(&PathBuf::from("res/textures/ui/themes/vines.png"), Some("res/textures/ui/fonts/vines.png"), &texture_creator);
    ui.init(&mut sfx);

    let mut player = Player::new(&texture_creator);
    player.unlocked_effects.push(effect::Effect::Glasses);
    player.unlocked_effects.push(effect::Effect::Speed);
    let mut input = Input::new();

    // let sfx_test = SoundEffect::new(PathBuf::from("res/audio/sfx/effect.mp3"));
    // sfx_test.play(&stream_arc);

    let mut world = World::load_from_file(&START_MAP.to_owned(), &texture_creator, &mut None);
    world.onload(&sink);
    if let Some(def) = world.default_pos {
        player.set_x(def.0 * 16);
        player.set_y(def.1 * 16);
    }

    canvas.set_scale(2.0, 2.0).unwrap();

    let mut events = sdl_context.event_pump().unwrap();
    let mut render_state = RenderState::new((800, 600));

    let mut next_time = unsafe { SDL_GetTicks() } + TICK_INTERVAL;
    let mut debug = Debug {
        load_handle: None
    };

    'mainloop: loop {
        for event in events.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit { .. } | Event::KeyDown { 
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'mainloop,
                Event::KeyDown { keycode, repeat, .. } => {
                    if keycode.is_some() && !repeat {
                        input.pressed(keycode.unwrap());
                    }
                },
                Event::KeyUp { keycode, .. } => {
                    if keycode.is_some() {
                        input.released(keycode.unwrap());
                    }
                }
                _ => ()
            }
        }

        canvas.set_draw_color(world.background_color);
        canvas.clear();

        debug.update(&input, &mut world);

        ui.update(&input, &mut player, &sink, &mut sfx);

        if !ui.open {
            if !world.paused {
                player.update(&input, &mut world);
            }
            world.update(&mut player, &sink);
        }

        input.update();

        render_state.offset = (-player.x + (render_state.screen_extents.0 as i32 / 2) - 8, -player.y + (render_state.screen_extents.1 as i32 / 2) - 16);
        if world.clamp_camera {
            render_state.clamp.0 = false;
            render_state.clamp.1 = false;

            if world.width * 16 < render_state.screen_extents.0 {
                render_state.clamp.0 = true;
                render_state.offset.0 = ((render_state.screen_extents.0 / 2) - ((world.width * 16) / 2)) as i32;
            } else {
                if render_state.offset.0 > 0 {
                    render_state.offset.0 = 0;
                    render_state.clamp.0 = true;
                }

                if render_state.offset.0 - 400 < -(world.width as i32 * 16) {
                    render_state.offset.0 = -(world.width as i32 * 16) + 400;
                    render_state.clamp.0 = true;
                }
            }
            
            if world.height * 16 < render_state.screen_extents.1 {
                render_state.clamp.1 = true;
                render_state.offset.1 = ((render_state.screen_extents.1 / 2) - ((world.height * 16) / 2)) as i32;
            } else {
                if render_state.offset.1 > 0 {
                    render_state.offset.1 = 0;
                    render_state.clamp.1 = true;
                }

                if render_state.offset.1 - 300 < -(world.height as i32 * 16) {
                    render_state.offset.1 = -(world.height as i32 * 16) + 300;
                    render_state.clamp.1 = true;
                }
            }
        }

        if !ui.clear {
            if world.looping {
                world.draw_looping(&mut canvas, &player, &render_state);
            } else {
                world.draw(&mut canvas, &player, &render_state);
            }
        }

        ui.draw(&player, &mut canvas);

        canvas.present();

        if world.queued_load.is_some() && world.transition.is_some() && world.transition.as_ref().unwrap().progress == 100 {
            let transition = world.transition.clone();
            let map = world.queued_load.as_ref().unwrap().map.clone();
            let name = PathBuf::from(map.clone()).file_stem().map(|f| f.to_str().unwrap_or("error").to_string());
            let default = world.default_pos.clone();
            player.moving = false;
            player.move_timer = 0;
            match world.queued_load.as_ref().unwrap().pos.0 {
                game::WarpCoord::Pos(x) => player.set_x(x * 16),
                game::WarpCoord::Add(x) => player.set_x(player.x + x * 16),
                game::WarpCoord::Sub(x) => player.set_x(player.x - x * 16),
                game::WarpCoord::Default => player.set_x(default.unwrap_or((0, 0)).0 * 16),
                _ => ()
            }
            match world.queued_load.as_ref().unwrap().pos.1 {
                game::WarpCoord::Pos(y) => player.set_y(y * 16),
                game::WarpCoord::Add(y) => player.set_y(player.y + y * 16),
                game::WarpCoord::Sub(y) => player.set_y(player.y - y * 16),
                game::WarpCoord::Default => player.set_y(default.unwrap_or((0, 0)).1 * 16),
                _ => ()
            }
            if let Some(new_name) = name {
                if new_name != world.name {
                    world = World::load_from_file(&map, &texture_creator, &mut Some(world));
                    world.transition = transition;
                    world.onload(&sink);
                } else {
                    world.reset();
                }
            }
            player.frozen = false;
        }

        if ui.menu_state.should_quit {
            break 'mainloop;
        }

        unsafe {
            SDL_Delay(time_left(next_time));
            next_time += TICK_INTERVAL;
        }
    }
}

const TICK_INTERVAL: u32 = 16;

unsafe fn time_left(next_time: u32) -> u32 {
    let now = SDL_GetTicks();
    if next_time <= now {
        return 0;
    } else {
        return next_time - now;
    }
}
