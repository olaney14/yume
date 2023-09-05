extern crate json;

use std::{path::PathBuf, io::BufReader, sync::Arc, collections::HashMap};

use audio::{SoundEffect, SoundEffectBank, Song};
use debug::Debug;
use game::{Input, RenderState, QueuedLoad, WarpPos, IntProperty, LevelPropertyType, Transition, TransitionType};
use player::Player;
use rodio::{OutputStream, Sink};
use sdl2::{image::{InitFlag}, keyboard::Keycode, sys::{SDL_Delay, SDL_GetTicks}, pixels::Color};
use ui::{Ui, MenuState, MenuType};
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
mod save;

pub const START_MAP: &str = "res/maps/nexus.tmx";
pub const DEBUG: bool = true;
pub const MAIN_MENU_MUSIC: &str = "res/audio/music/travel.ogg";
pub const MAIN_MENU_MUSIC_SPEED: f32 = 0.25;
pub const MAIN_MENU_MUSIC_VOLUME: f32 = 0.5;

fn find_sdl_gl_driver() -> Option<u32> {
    for (index, item) in sdl2::render::drivers().enumerate() {
        if item.name == "opengl" {
            return Some(index as u32);
        }
    }

    None
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG);
    let window = video_subsystem
        .window("yume", 800, 600)
        .opengl()
        .position_centered()
        .build()
        .map_err(|e| e.to_string()).unwrap();

    let mut canvas = window
        .into_canvas()
        .index(find_sdl_gl_driver().expect("No OpenGL driver found"))
        .target_texture()
        .build()
        .map_err(|e| e.to_string()).unwrap();
    let texture_creator = canvas.texture_creator();

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut sfx = SoundEffectBank::new(Arc::new(stream_handle));

    // TODO uhhhhhhh
    // so rust thinks that the reference in line ?? is still being used here
    // idk how to fix that
    let mut ui = Ui::new(&PathBuf::from("res/textures/ui/themes/menu.png"), Some("res/textures/ui/fonts/menu.png"), &texture_creator);
    ui.init(&mut sfx);

    let mut player = Player::new(&texture_creator);
    player.unlocked_effects.push(effect::Effect::Speed);
    let mut input = Input::new();
    // CHANGED
    // let mut world = World::load_from_file(&START_MAP.to_owned(), &texture_creator, &mut None);
    let mut world = World::new(&texture_creator);
    let mut song = Song::new(PathBuf::from(MAIN_MENU_MUSIC));
    song.default_speed = MAIN_MENU_MUSIC_SPEED;
    song.speed = MAIN_MENU_MUSIC_SPEED;
    song.volume = MAIN_MENU_MUSIC_VOLUME;
    song.default_volume = MAIN_MENU_MUSIC_VOLUME;
    song.dirty = true;
    world.song = Some(song);
    world.onload(&sink);
    if let Some(def) = world.default_pos {
        player.set_x(def.0 * 16);
        player.set_y(def.1 * 16);
    }

    canvas.set_scale(2.0, 2.0).unwrap();

    world.paused = true;
    ui.show_menu(MenuType::MainMenu);

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

        ui.update(&input, &mut player, &mut world, &sink, &mut sfx);

        if ui.effect_get_timer > 0 {
            ui.effect_get_timer -= 1;
            if ui.effect_get_timer == 0 {
                ui.effect_get = None;
                world.paused = false;
                player.frozen = false;
                player.frozen_time = 0;
            }
        }

        if world.special_context.new_game {
            world.queued_load = Some(QueuedLoad {
                map: String::from(START_MAP),
                pos: WarpPos { x: IntProperty::Level(LevelPropertyType::DefaultX), y: IntProperty::Level(LevelPropertyType::DefaultY) }
            });
            world.transition = Some(Transition::new(TransitionType::FadeScreenshot, 2, true, 32));
            world.special_context.new_game = false;
            world.paused = false;
        }

        if !ui.open {
            if !world.paused {
                player.update(&input, &mut world, &mut sfx);
            }
            world.update(&mut player, &mut sfx, &sink);
            if player.effect_just_changed {
                player.effect_just_changed = false;
            }
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

        ui.draw(&player, &mut canvas, &render_state);

        if world.transition_context.take_screenshot {
            let mut screenshot = world.transition_context.screenshot.take().unwrap();
            canvas.with_texture_canvas(&mut screenshot, |tex_canvas| {
                tex_canvas.set_draw_color(world.background_color);
                tex_canvas.set_blend_mode(sdl2::render::BlendMode::None);
                tex_canvas.clear();
                tex_canvas.set_blend_mode(sdl2::render::BlendMode::Blend);

                if !ui.menu_state.menu_screenshot {
                    if world.looping {
                        world.draw_looping(tex_canvas, &player, &render_state);
                    } else {
                        world.draw(tex_canvas, &player, &render_state);
                    }
                }

                ui.draw(&player, tex_canvas, &render_state);
            }).unwrap();
            world.transition_context.screenshot = Some(screenshot);
            world.transition_context.take_screenshot = false;
            ui.menu_state.menu_screenshot = false;
        }

        canvas.present();

        if world.queued_load.is_some() && world.transition.is_some() && world.transition.as_ref().unwrap().progress == 100 {
            let transition = world.transition.clone();
            let map = world.queued_load.as_ref().unwrap().map.clone();
            let name = PathBuf::from(map.clone()).file_stem().map(|f| f.to_str().unwrap_or("error").to_string());
            let default = world.default_pos.clone();
            player.moving = false;
            player.move_timer = 0;
            let warp_pos = world.queued_load.as_ref().unwrap().pos.clone();

            if let Some(new_name) = name {
                if new_name != world.name {
                    let old_flags = std::mem::replace(&mut world.global_flags, HashMap::new());
                    world = World::load_from_file(&map, &texture_creator, &mut Some(world));
                    world.global_flags = old_flags;
                    world.transition = transition;
                    world.onload(&sink);
                } else {
                    world.reset();
                }
            }
            if let Some(x) = warp_pos.x.get(Some(&player), Some(&world)) {
                player.set_x(x * 16);
            }
            if let Some(y) = warp_pos.y.get(Some(&player), Some(&world)) {
                player.set_y(y * 16);
            }

            player.frozen = false;
            ui.clear = false;
            ui.open = false;
        }

        if ui.menu_state.should_quit {
            break 'mainloop;
        }

        unsafe {
            let time = time_left(next_time);
            SDL_Delay(time);
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
