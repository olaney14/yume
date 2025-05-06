extern crate json;

use std::{path::PathBuf, sync::Arc, collections::HashMap, fs::File};

use audio::{SoundEffectBank, Song};
use debug::{Debug, ProfileInfo};
use game::{Input, RenderState, QueuedLoad, WarpPos, IntProperty, LevelPropertyType};
use player::Player;
use rodio::{OutputStream, Sink};
use save::{SaveInfo, SaveData};
use sdl2::{image::{InitFlag, LoadSurface}, keyboard::Keycode, pixels::Color, rect::Rect, surface::Surface, sys::{SDL_Delay, SDL_GetTicks}, video::FullscreenType};
use texture::Texture;
use transitions::{Transition, TransitionType};
use ui::{Ui, MenuType, Font};
use world::World;

extern crate sdl2;

mod actions;
mod ai;
mod audio;
mod debug;
mod effect;
mod entity;
mod game;
mod loader;
// mod optimize;
mod particles;
mod player;
mod save;
mod screen_event;
mod tiles;
mod transitions;
mod texture;
mod ui;
mod world;

pub const START_MAP: &str = "res/maps/bedroom.tmx";
pub const DEBUG: bool = true;
pub const MAIN_MENU_MUSIC: &str = "res/audio/music/travel.ogg";
pub const MAIN_MENU_MUSIC_SPEED: f32 = 0.25;
pub const MAIN_MENU_MUSIC_VOLUME: f32 = 0.5;
pub const MAIN_MENU_THEME: &str = "res/textures/ui/themes/system.png";
pub const MAIN_MENU_FONT: &str = "res/textures/ui/fonts/menu.png";

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
    let mut window = video_subsystem
        .window("yume", 640, 480)
        .opengl()
        .position_centered()
        .build()
        .map_err(|e| e.to_string()).unwrap();
    let window_icon = Surface::from_file("res/textures/icon.png").expect("Failed to load res/textures/icon.png. Make sure the executable is in the same directory as the res/ folder.");
    window.set_icon(window_icon);

    let mut canvas = window
        .into_canvas()
        .index(find_sdl_gl_driver().expect("No OpenGL driver found"))
        .target_texture()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string()).unwrap();
    let texture_creator = canvas.texture_creator();
    let mut render_state = RenderState::new((640, 480));

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut sfx = SoundEffectBank::new(Arc::new(stream_handle));

    // TODO uhhhhhhh
    // so rust thinks that the reference in line ?? is still being used here
    // idk how to fix that
    let mut ui = Ui::new(&PathBuf::from(MAIN_MENU_THEME), Some(MAIN_MENU_FONT), &texture_creator);
    //ui.init(&mut sfx);

    let mut save_info = SaveInfo::read_or_create_new().expect("failed to read or create save data, the .saves file may be missing or corrupted");

    let mut player = Player::new(&texture_creator);

    let mut input = Input::new();

    let mut world = World::new(&texture_creator, &render_state);
    let mut song = Song::new(PathBuf::from(MAIN_MENU_MUSIC));
    song.default_speed = MAIN_MENU_MUSIC_SPEED;
    song.speed = MAIN_MENU_MUSIC_SPEED;
    song.volume = MAIN_MENU_MUSIC_VOLUME;
    song.default_volume = MAIN_MENU_MUSIC_VOLUME;
    song.dirty = true;
    world.song = Some(song);
    world.onload(&player, &sink);
    if let Some(def) = world.default_pos {
        player.set_x(def.0 * 16);
        player.set_y(def.1 * 16);
    }

    canvas.set_scale(2.0, 2.0).unwrap();

    world.paused = true;
    ui.show_menu(MenuType::MainMenu);

    let mut events = sdl_context.event_pump().unwrap();

    let mut next_time = unsafe { SDL_GetTicks() } + TICK_INTERVAL;
    let mut debug = Debug {
        load_handle: None,
        profiler: ProfileInfo::new(),
        enable_profiling: false,
        enable_debug_overlay: false,
        mini_font: Font::new_mini(Texture::from_file(&PathBuf::from(ui::MINIFONT_PATH), &texture_creator).expect("failed to load debug font"))
    };

    'mainloop: loop {
        for event in events.poll_iter() {
            use sdl2::event::Event;
            match event {
                Event::Quit { .. } => break 'mainloop,
                Event::KeyDown { keycode, repeat, .. } => {
                    if keycode.is_some() && !repeat {
                        input.pressed(keycode.unwrap());
                    }
                },
                Event::KeyUp { keycode, .. } => {
                    if keycode.is_some() {
                        input.released(keycode.unwrap());
                    }
                },
                _ => ()
            }
        }

        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.clear();
        if !ui.clear {
            canvas.set_draw_color(world.background_color);
        } else {
            canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        }
        canvas.fill_rect(Rect::new(0, 0, 640, 480)).unwrap();

        debug.update(&input, &mut world, &mut player, &mut sfx);
        ui.update(&input, &mut player, &mut world, &save_info, &sink, &mut sfx);

        if world.special_context.write_save_to_pending {
            let save_data = SaveData::create(&player);
            save_data.save(world.special_context.pending_save as u32, &PathBuf::from("saves/".to_string() + &world.special_context.pending_save.to_string() + ".save"), &mut save_info).expect("failed to save game data");
            world.special_context.write_save_to_pending = false
        }

        if world.special_context.new_game {
            if let Some(load) = world.special_context.pending_load {
                let file = File::open(&PathBuf::from("saves/".to_string() + &load.to_string() + ".save")).expect("failed to open save file");
                let save_data: SaveData = serde_cbor::from_reader(&file).expect("failed to read save data. data may be corrupted");
                player = save_data.get_player(&texture_creator);
            } else {
                player = Player::new(&texture_creator);
            }
            world.special_context.pending_load = None;
            

            world.queued_load = Some(QueuedLoad {
                map: String::from(START_MAP),
                pos: WarpPos { x: IntProperty::Level(LevelPropertyType::DefaultX), y: IntProperty::Level(LevelPropertyType::DefaultY) }
            });
            world.transition = Some(Transition::new(TransitionType::FadeScreenshot, 2, 0, true, 32, false));
            world.special_context.new_game = false;
            world.paused = false;
        }

        if !ui.open {
            if !world.paused {
                player.update(&input, &mut world, &mut sfx);
            }
            world.update(&mut player, &mut sfx, &sink, &input, &mut render_state);
            if player.effect_just_changed {
                player.effect_just_changed = false;
            }
        }

        if input.get_just_pressed(Keycode::F4) {
            if render_state.fullscreen {
                canvas.set_scale(2.0, 2.0).unwrap();
                canvas.window_mut().set_fullscreen(FullscreenType::Off).unwrap();
            } else {
                canvas.set_scale(4.0, 4.0).unwrap();
                canvas.window_mut().set_fullscreen(FullscreenType::Desktop).unwrap();
                canvas.set_clip_rect(Rect::new(0, 0, render_state.screen_dims.0 / 2, render_state.screen_dims.1 / 2));
                let window_size = canvas.window().size();
                canvas.set_viewport(Rect::new(
                    (window_size.0 / 2 - (render_state.screen_dims.0)) as i32 / 4,
                    (window_size.1 / 2 - (render_state.screen_dims.1)) as i32 / 4,
                    render_state.screen_dims.0 / 2,
                    render_state.screen_dims.1 / 2
                ));
            }
            render_state.fullscreen = !render_state.fullscreen;
        }

        input.update();
        clamp_camera(&mut render_state, &world, &player);

        // if world.special_context.camera_slide {
        //     render_state.offset.0 += world.special_context.camera_slide_offset.0;
        //     render_state.offset.1 += world.special_context.camera_slide_offset.1;

        //     let direction_x = (world.special_context.camera_slide_target.0 - world.special_context.camera_slide_offset.0).signum();
        //     let direction_y = (world.special_context.camera_slide_target.1 - world.special_context.camera_slide_offset.1).signum();

        //     world.special_context.camera_slide_offset.0 += world.special_context.camera_slide_speed as i32 * direction_x;
        //     world.special_context.camera_slide_offset.1 += world.special_context.camera_slide_speed as i32 * direction_y;
        //     render_state.player_offset.0 += world.special_context.camera_slide_speed as i32 * direction_x;
        //     render_state.player_offset.1 += world.special_context.camera_slide_speed as i32 * direction_y;

        //     let direction_x1 = (world.special_context.camera_slide_target.0 - world.special_context.camera_slide_offset.0).signum();
        //     let direction_y1 = (world.special_context.camera_slide_target.1 - world.special_context.camera_slide_offset.1).signum();

        //     if direction_x != direction_x1 {
        //         world.special_context.camera_slide_offset.0 = world.special_context.camera_slide_offset.1;
        //     }

        //     if direction_y != direction_y1 {
        //         world.special_context.camera_slide_offset.1 = world.special_context.camera_slide_offset.1;
        //     }

        //     if direction_y != direction_y1 && direction_x != direction_x1 {
        //         if world.special_context.camera_slide_offset.0 == 0 && world.special_context.camera_slide_offset.1 == 0 {
        //             world.special_context.camera_slide = false;
        //         }
        //     }
        // }

        // If the ui is not clearing the screen and a menu screenshot is not being taken
        if !ui.clear && !ui.menu_state.menu_screenshot {
            if world.looping {
                world.draw_looping(&mut canvas, &player, &render_state);
            } else {
                world.draw(&mut canvas, &player, &render_state);
            }
        }

        // Exclude transitions from screenshots 
        if !ui.clear {
            world.draw_transitions(&mut canvas, &player, &render_state);
        }

        ui.draw(&player, &mut canvas, &save_info, &render_state);

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

                ui.draw(&player, tex_canvas, &save_info, &render_state);
            }).unwrap();
            world.transition_context.screenshot = Some(screenshot);
            world.transition_context.take_screenshot = false;
            ui.menu_state.menu_screenshot = false;
        }

        debug.draw(&mut canvas, &ui, &player, &render_state);

        canvas.present();

        if world.queued_load.is_some() && world.transition.is_some() && world.transition.as_ref().unwrap().progress >= 100 {
            let transition = world.transition.clone();
            let map = world.queued_load.as_ref().unwrap().map.clone();
            let name = PathBuf::from(map.clone()).file_stem().map(|f| f.to_str().unwrap_or("error").to_string());
            //let default = world.default_pos.clone();
            player.moving = false;
            player.move_timer = 0;
            let warp_pos = world.queued_load.as_ref().unwrap().pos.clone();

            let mut skip_end = false;

            if let Some(new_name) = name {
                if (new_name != world.name) || world.special_context.reload_on_warp {
                    world.special_context.reload_on_warp = false;
                    let mut old_song = None;
                    if let Some(song) = &world.song {
                        old_song = Some(song.path.clone());
                    }
                    let old_flags = std::mem::replace(&mut world.global_flags, HashMap::new());
                    world = World::load_from_file(&map, &texture_creator, &mut Some(world), &render_state).expect("failed to load map");
                    world.global_flags = old_flags;
                    world.transition = transition;

                    if let Some(song) = &mut world.song {
                        if let Some(transition) = &world.transition {
                            if transition.fade_music {
                                song.volume = 0.0;
                            }

                            if let Some(old_song) = old_song {
                                if transition.reset_same_music && old_song == song.path {
                                    song.reload(&sink);
                                }
                            }
                        }
                    }
                    
                    //world.onload(&player, &sink);
                } else {
                    world.reset();
                    world.transition_context.take_screenshot = true;
                }
            } else {
                if map == "" {
                    let old_flags = std::mem::replace(&mut world.global_flags, HashMap::new());
                    world = World::new(&texture_creator, &render_state);
                    world.global_flags = old_flags;
                    world.transition = transition;
                    let mut song = Song::new(PathBuf::from(MAIN_MENU_MUSIC));
                    song.default_speed = MAIN_MENU_MUSIC_SPEED;
                    song.speed = MAIN_MENU_MUSIC_SPEED;
                    song.volume = MAIN_MENU_MUSIC_VOLUME;
                    song.default_volume = MAIN_MENU_MUSIC_VOLUME;
                    song.dirty = true;
                    world.song = Some(song);
                    //world.onload(&player, &sink);

                    ui.menu_state.current_menu = MenuType::MainMenu;
                    ui.open = true;
                    ui.clear = true;
                    ui.menu_state.button_id = 2;
                    world.paused = true;
                    skip_end = true;
                }
            }

            if let Some(x) = warp_pos.x.get(Some(&player), Some(&world)) {
                player.set_x(x * 16);
            }
            if let Some(y) = warp_pos.y.get(Some(&player), Some(&world)) {
                player.set_y(y * 16);
            }

            world.onload(&player, &sink);

            if !skip_end {
                player.frozen = false;
                ui.clear = false;
                ui.open = false;
            }

            player.on_level_transition();
        }

        if ui.menu_state.should_quit {
            break 'mainloop;
        }

        unsafe {
            let time = time_left(next_time);
            SDL_Delay(time);
            // next_time += TICK_INTERVAL;
            next_time = SDL_GetTicks() + TICK_INTERVAL;
        }
    }
}

fn clamp_camera(render_state: &mut RenderState, world: &World, player: &Player) {
    render_state.offset = (-player.x + (render_state.screen_extents.0 as i32 / 2) - 8, -player.y + (render_state.screen_extents.1 as i32 / 2) - 16);

    if world.clamp_horizontal() {
        render_state.clamp.0 = false;
        if world.width * 16 < render_state.screen_extents.0 {
            render_state.clamp.0 = true;
            render_state.offset.0 = ((render_state.screen_extents.0 / 2) - ((world.width * 16) / 2)) as i32;
        } else {
            if render_state.offset.0 > 0 {
                render_state.offset.0 = 0;
                render_state.clamp.0 = true;
            }

            if render_state.offset.0 - (render_state.screen_dims.0 as i32 / 2) < -(world.width as i32 * 16) {
                render_state.offset.0 = -(world.width as i32 * 16) + (render_state.screen_dims.0 as i32 / 2);
                render_state.clamp.0 = true;
            }
        }
    }

    if world.clamp_vertical() {
        render_state.clamp.1 = false;

        if world.height * 16 < render_state.screen_extents.1 {
            render_state.clamp.1 = true;
            render_state.offset.1 = ((render_state.screen_extents.1 / 2) - ((world.height * 16) / 2)) as i32;
        } else {
            if render_state.offset.1 > 0 {
                render_state.offset.1 = 0;
                render_state.clamp.1 = true;
            }

            if render_state.offset.1 - (render_state.screen_dims.1 as i32 / 2) < -(world.height as i32 * 16) {
                render_state.offset.1 = -(world.height as i32 * 16) + (render_state.screen_dims.1 as i32 / 2);
                render_state.clamp.1 = true;
            }
        }
    }

    render_state.offset.0 += render_state.camera_slide_offset.0;
    render_state.offset.1 += render_state.camera_slide_offset.1;
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