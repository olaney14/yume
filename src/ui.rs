use std::{path::PathBuf, collections::HashMap};

use rodio::Sink;
use sdl2::{render::{RenderTarget, Canvas, TextureCreator}, rect::Rect, keyboard::Keycode, pixels::Color};

use crate::{audio::SoundEffectBank, effect::Effect, game::{Input, IntProperty, LevelPropertyType, QueuedLoad, RenderState, WarpPos}, player::{self, Player}, save::SaveInfo, texture::Texture, tiles::Tileset, transitions::{Transition, TransitionType}, world::World};

const MENU_FRAME_TOP_RIGHT: u32 = 0;
const MENU_FRAME_TOP: u32 = 1;
const MENU_FRAME_TOP_LEFT: u32 = 2;
const MENU_FRAME_LEFT: u32 = 6;
const MENU_FRAME_CENTER: u32 = 7;
const MENU_FRAME_RIGHT: u32 = 8;
const MENU_FRAME_BOTTOM_LEFT: u32 = 12;
const MENU_FRAME_BOTTOM: u32 = 13;
const MENU_FRAME_BOTTOM_RIGHT: u32 = 14;
const MENU_SELECTION_BORDER_LEFT: u32 = 3;
const MENU_SELECTION_BORDER_RIGHT: u32 = 4;
const MENU_SELECTION_HIGHLIGHT: u32 = 9;
const MENU_ARROW_RIGHT: u32 = 10;
const MENU_ARROW_LEFT: u32 = 11;
const MENU_BUTTON_PADDING_VERT: u32 = 2;
const MENU_BUTTON_PADDING_HORIZ: u32 = 2;

const FONT_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .!,-�?§µ";
const DEFAULT_FONT: &str = "res/textures/ui/fonts/font.png";
const DEFAULT_FONT_WIDTH: u32 = 6;
const DEFAULT_FONT_HEIGHT: u32 = 10;
const DEFAULT_FONT_SPACING_HORIZ: u32 = 0;
const DEFAULT_FONT_SPACING_VERT: u32 = 1;

const MINIFONT_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .!,-�?§µ()[]{}<>'\"`+=/\\*|#%^:;";
pub const MINIFONT_PATH: &str = "res/textures/ui/fonts/minifont.png";
const MINIFONT_FONT_WIDTH: u32 = 3;
const MINIFONT_FONT_HEIGHT: u32 = 5;
const MINIFONT_FONT_SPACING_HORIZ: u32 = 1;
const MINIFONT_FONT_SPACING_VERT: u32 = 1;

const FONT_VINES: &str = "res/textures/ui/fonts/vines.png";

const BUTTONS_MAIN: u32 = 5;
const BUTTONS_TITLE: u32 = 3;

const SFX_VOLUME: f32 = 0.7;

const MAIN_MENU_WIDTH: u32 = 5;
const MAIN_MENU_HEIGHT: u32 = 4;
const MAIN_MENU_Y: u32 = 175;
const MAIN_MENU_TITLE_Y: u32 = 25;
const MAIN_MENU_TITLE: &str = "res/textures/ui/title.png";

pub enum MenuType {
    Home,
    Effects,
    Special,
    Me,
    Quit,
    MainMenu,
    SaveConfirm,

    /// True - save, False - load
    SaveLoad(bool)
}

pub struct MenuState {
    pub close_on_x: bool,
    pub current_menu: MenuType,
    pub button_id: i32,
    pub selection_flash: bool,
    pub timer: u32,
    pub should_quit: bool,
    pub switch_to_main: bool,
    pub menu_should_close: bool,
    pub menu_screenshot: bool,
    pub page_index: i32
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            close_on_x: false,
            current_menu: MenuType::Home,
            button_id: 0,
            selection_flash: true,
            timer: 0,
            should_quit: false,
            menu_should_close: false,
            menu_screenshot: false,
            switch_to_main: false,
            page_index: 0
        }
    }

    pub fn update(&mut self, input: &Input, player: &mut Player, world: &mut World, save_info: &SaveInfo, sfx: &mut SoundEffectBank) {
        if input.get_just_pressed(Keycode::X) {
            match self.current_menu {
                MenuType::Effects | MenuType::Quit | MenuType::Special | MenuType::Me => {
                    if matches!(self.current_menu, MenuType::Effects) { self.button_id = 0; }
                    if matches!(self.current_menu, MenuType::Special) { self.button_id = 1; }
                    if matches!(self.current_menu, MenuType::Me) { self.button_id = 2; }
                    if matches!(self.current_menu, MenuType::Quit) { self.button_id = 4; }
                    sfx.play_ex("menu_blip_negative", 1.0, 0.5);

                    self.current_menu = MenuType::Home;
                    self.close_on_x = true;
                },
                MenuType::SaveConfirm => {
                    self.current_menu = MenuType::SaveLoad(true);
                    self.close_on_x = true;
                    self.button_id = world.special_context.pending_save as i32;
                },
                MenuType::SaveLoad(save) => {
                    if !save {
                        self.current_menu = MenuType::MainMenu;
                        self.close_on_x = false;
                        self.button_id = 1;
                        sfx.play_ex("menu_blip_negative", 1.0, 0.5);
                    }
                }
                _ => ()
            }
        }

        if input.get_just_pressed(Keycode::Z) {
            match self.current_menu {
                MenuType::Home => {
                    match self.button_id {
                        0 => {
                            // Effects
                            self.current_menu = MenuType::Effects;
                            self.close_on_x = false;
                        },
                        1 => {
                            // Special
                            self.current_menu = MenuType::Special;
                            self.close_on_x = false;
                            self.button_id = 0;
                        },
                        2 => {
                            // Me
                            self.current_menu = MenuType::Me;
                            self.close_on_x = false;
                            self.button_id = 0;
                        },
                        4 => {
                            // Quit
                            self.current_menu = MenuType::Quit;
                            self.close_on_x = false;
                            self.button_id = 1;
                        }
                        _ => ()
                    }

                    match self.button_id {
                        0 | 1 | 2 | 4 => {
                            sfx.play("menu_blip_affirmative");
                        },
                        _ => {
                            sfx.play("menu_blip_error");
                        }
                    }
                },
                MenuType::Effects => {
                    if player.unlocked_effects.len() > 0 {
                        if player.dreaming {
                            if player.current_effect.is_some() && player.current_effect.as_ref().unwrap() == &player.unlocked_effects[self.button_id as usize] {
                                player.remove_effect();
                                sfx.play("effect_negate");
                            } else {
                                if player.current_effect.is_some() {
                                    player.remove_effect();
                                }
                                player.apply_effect(player.unlocked_effects[self.button_id as usize].clone());
                                sfx.play("effect");
                            }
                            self.current_menu = MenuType::Home;
                            self.menu_should_close = true;
                        } else {
                            sfx.play("menu_blip_error");
                        }
                    }
                },
                MenuType::Quit => {
                    match self.button_id {
                        0 => {
                            // Yes
                            //self.should_quit = true;
                            self.current_menu = MenuType::MainMenu;
                            self.button_id = 2;

                            self.menu_should_close = true;
                            self.close_on_x = false;
                            //world.paused = false;

                            world.queued_load = Some(QueuedLoad {
                                map: String::new(),
                                pos: WarpPos { x: IntProperty::Level(LevelPropertyType::DefaultX), y: IntProperty::Level(LevelPropertyType::DefaultY) }
                            });
                            world.transition = Some(Transition::new(TransitionType::FadeScreenshot, 32, 0, true, 2, false));
                        },
                        1 => {
                            // No
                            self.current_menu = MenuType::Home;
                            self.close_on_x = true;
                            self.button_id = 4;
                        },
                        _ => ()
                    }

                    sfx.play("menu_blip_affirmative");
                },
                MenuType::Me => {
                    sfx.play("menu_blip_error");
                },
                MenuType::MainMenu => {
                    match self.button_id {
                        0 => {
                            // New Game
                            world.special_context.new_game = true;
                            world.paused = false;
                            self.menu_should_close = true;
                            self.menu_screenshot = true;
                            sfx.play_ex("menu_blip_affirmative", 1.0, 0.25);
                        }
                        1 => {
                            // Continue
                            if save_info.files.is_empty() {
                                sfx.play_ex("menu_blip_error", 1.0, 0.25);
                            } else {
                                self.button_id = 0;
                                self.close_on_x = false;
                                self.current_menu = MenuType::SaveLoad(false);
                                sfx.play_ex("menu_blip_affirmative", 1.0, 0.25);
                            }
                        }
                        2 => {
                            // Quit
                            self.should_quit = true;
                        },
                        _ => ()
                    }
                },
                MenuType::SaveLoad(b) => {
                    if b {
                        // this shouldn't fail because button_id can only be negative in the scrolling functions
                        world.special_context.pending_save = (self.button_id + self.page_index * 3) as usize;
                        self.current_menu = MenuType::SaveConfirm;
                        self.close_on_x = false;
                        self.button_id = 0;
                    } else {
                        world.special_context.pending_load = Some((self.button_id + self.page_index * 3) as usize);
                        world.special_context.new_game = true;
                        world.paused = false;
                        self.menu_should_close = true;
                        self.menu_screenshot = true;
                        sfx.play_ex("menu_blip_affirmative", 1.0, 0.25);
                    }
                },
                MenuType::SaveConfirm => {
                    if self.button_id == 0 {
                        world.special_context.write_save_to_pending = true;
                        sfx.play_ex("magic0", 1.0, 0.25);
                    }

                    self.close_on_x = true;
                    self.current_menu = MenuType::SaveLoad(true);
                    self.button_id = world.special_context.pending_save as i32;
                },
                MenuType::Special => {
                    if self.button_id == 0 {
                        if player.dreaming {
                            player.waking_up = true;
                            player.waking_up_timer = player::WAKE_UP_TIMER_MAX;

                            self.menu_should_close = true;

                            // self.menu_should_close = true;
                            // sfx.play_ex("song1", 1.5, 0.5);

                            // 11, 6
                            // world.queued_load = Some(
                            //     crate::game::QueuedLoad { map: "res/maps/bedroom.tmx".to_string(), pos: WarpPos {
                            //         x: IntProperty::Level(LevelPropertyType::DefaultX),
                            //         y: IntProperty::Level(LevelPropertyType::DefaultY)
                            //     } }
                            // );
                            // world.queued_load = Some(
                            //     crate::game::QueuedLoad { map: "res/maps/bedroom.tmx".to_string(), pos: WarpPos {
                            //         x: IntProperty::Int(11),
                            //         y: IntProperty::Int(6)
                            //     } }
                            // );
                            // world.transition = Some(
                            //     Transition::new(TransitionType::GridCycle, 1, 1, true, 5, false)
                            // );
                            // world.global_flags.insert("start_in_bed".to_string(), 1);

                            // player.dreaming = false;
                            // player.remove_effect();
                        } else {
                            sfx.play("menu_blip_error");
                        }
                    }
                }
            }
        }

        match self.current_menu {
            MenuType::Home => {
                if input.get_just_pressed(Keycode::Up) { self.button_id -= 1; }
                if input.get_just_pressed(Keycode::Down) { self.button_id += 1; }
                if self.button_id >= BUTTONS_MAIN as i32 {
                    self.button_id = 0;
                }
                if self.button_id < 0 {
                    self.button_id = BUTTONS_MAIN as i32 - 1;
                }
            },
            MenuType::Effects => {
                if input.get_just_pressed(Keycode::Right) { self.button_id += 1; }
                if input.get_just_pressed(Keycode::Down) { self.button_id += 2; }
                if input.get_just_pressed(Keycode::Left) { self.button_id -= 1; }
                if input.get_just_pressed(Keycode::Up) { self.button_id -= 2; }
                if self.button_id >= player.unlocked_effects.len() as i32 {
                    self.button_id = 0;
                }
                if self.button_id < 0 {
                    self.button_id = player.unlocked_effects.len() as i32 - 1;
                }
            },
            MenuType::Quit | MenuType::SaveConfirm => {
                if input.get_just_pressed(Keycode::Up) { self.button_id -= 1; }
                if input.get_just_pressed(Keycode::Down) { self.button_id += 1; }
                if self.button_id > 1 {
                    self.button_id = 0;
                }
                if self.button_id < 0 {
                    self.button_id = 1;
                }
            },
            MenuType::MainMenu => {
                if input.get_just_pressed(Keycode::Up) { self.button_id -= 1; }
                if input.get_just_pressed(Keycode::Down) { self.button_id += 1; }
                if self.button_id >= BUTTONS_TITLE as i32 {
                    self.button_id = 0;
                }
                if self.button_id < 0 {
                    self.button_id = BUTTONS_TITLE as i32 - 1;
                }
            },
            MenuType::SaveLoad(b) => {
                if input.get_just_pressed(Keycode::Up) { self.button_id -= 1; }
                if input.get_just_pressed(Keycode::Down) { self.button_id += 1; }
                if input.get_just_pressed(Keycode::Right) { self.page_index += 1; }
                if input.get_just_pressed(Keycode::Left) { self.page_index -= 1; }

                //let button_max = save_info.files.len() as i32;
                //let button_max_load = ((save_info.files.len() - 1) % 3) as i32;
                let button_max_load = (save_info.files.len() as i32 - (3 * self.page_index)).min(3);
                let page_max_load = (save_info.files.len() as i32 - 1).max(0) / 3;
                //let button_max_save = button_max_load + 1;
                let button_max_save = (save_info.files.len() as i32 - (3 * self.page_index) + 1).min(3);
                // if self.page_index != 0 {
                //     button_max_save = (1 + button_max_save).min(3);
                // }
                let page_max_save = (save_info.files.len() / 3) as i32;
        
                if b { // Save
                    if self.button_id >= button_max_save {
                        self.button_id = 0;
                    }
                    if self.button_id < 0 {
                        self.button_id = (button_max_save - 1).max(0);
                    }
                    if self.page_index < 0 {
                        self.page_index = page_max_save;
                    }
                    if self.page_index > page_max_save {
                        self.page_index = 0;
                    }
                } else { // Load
                    if self.button_id >= button_max_load {
                        self.button_id = 0;
                    }
                    if self.button_id < 0 {
                        self.button_id = (button_max_load - 1).max(0);
                    }
                    if self.page_index < 0 {
                        self.page_index = page_max_load;
                    }
                    if self.page_index > page_max_load {
                        self.page_index = 0;
                    }
                }
            },
            MenuType::Special => {
                let button_max = 1;
                if input.get_just_pressed(Keycode::Up) { self.button_id -= 1; }
                if input.get_just_pressed(Keycode::Down) { self.button_id += 1; }
                if self.button_id >= button_max {
                    self.button_id = 0;
                }
                if self.button_id < 0 {
                    self.button_id = button_max - 1;
                }
            }
            _ => ()
        }

        self.selection_flash = (self.timer / 10) % 2 == 0;

        self.timer += 1;
    }
}

pub struct Ui<'a> {
    pub theme: MenuSet<'a>,
    pub clear: bool,
    pub open: bool,
    pub menu_state: MenuState,
    pub effect_get: Option<String>,
    pub effect_get_timer: u32,
    pub player_preview_texture: Texture<'a>,
}

impl<'a> Ui<'a> {
    pub fn new<T>(theme: &PathBuf, font: Option<&str>, creator: &'a TextureCreator<T>) -> Self {
        let tileset = Tileset::load_from_file(theme, creator);
        Self {
            theme: MenuSet::from_tileset(tileset, font, creator),
            clear: false,
            open: false,
            menu_state: MenuState::new(),
            effect_get: None,
            effect_get_timer: 0,
            player_preview_texture: Texture::from_file(&PathBuf::from("res/textures/misc/preview.png"), creator).expect("could not finish loading textures")
        }
    }

    /// Shows a menu<br>
    pub fn show_menu(&mut self, menu: MenuType) {
        self.menu_state.current_menu = menu;
        self.clear = true;
        self.open = true;
    }

    pub fn effect_get(&mut self, effect: &Effect) {
        self.effect_get = Some(effect.name().to_string());
        self.effect_get_timer = 128;
    }

    // pub fn init(&self, sfx: &mut SoundEffectBank) {
    //     sfx.load(&String::from("menu_blip_affirmative"), SFX_VOLUME, 1.0);
    //     sfx.load(&String::from("menu_blip_negative"), SFX_VOLUME, 1.0);
    //     sfx.load(&String::from("menu_blip_error"), SFX_VOLUME, 1.0);
    // }

    pub fn update(&mut self, input: &Input, player: &mut Player, world: &mut World, save_info: &SaveInfo, sink: &Sink, sfx: &mut SoundEffectBank) {
        if world.special_context.save_game {
            self.show_menu(MenuType::SaveLoad(true));
            self.menu_state.close_on_x = true;
            self.menu_state.button_id = 0;
            world.special_context.save_game = false;
        }
        
        if input.get_just_pressed(Keycode::X) && self.effect_get.is_none() {
            if self.open && self.menu_state.close_on_x {
                //sink.play();
                match self.menu_state.current_menu {
                    MenuType::SaveLoad(b) => {
                        if b {
                            if let Some(song) = &mut world.song {
                                
                                song.default_volume = 0.0;
                                song.volume = 0.0;
                                song.dirty = true;
                            }
                        }
                    }
                    _ => {
                        sink.set_volume(sink.volume() * 5.0);
                    }
                }
                self.open = false;
                self.clear = false;
                sfx.play_ex("menu_blip_negative", 1.0, 0.5);

            } else if !self.open && !player.moving && !player.disable_player_input && world.transition.is_none() {
                //sink.pause();
                self.menu_state.current_menu = MenuType::Home;
                sink.set_volume(sink.volume() / 5.0);
                self.open = true;  
                self.clear = true;
                self.menu_state.button_id = 0;
                self.menu_state.close_on_x = true;
                sfx.play("menu_blip_affirmative");
            }
        }

        if let Some(effect) = &world.special_context.effect_get {
            self.effect_get(effect);
        }

        if self.menu_state.menu_should_close && self.open {
            sink.set_volume(sink.volume() * 5.0);
            self.open = false;
            self.clear = false;
            self.menu_state.menu_should_close = false;
        }

        if self.effect_get_timer > 0 {
            self.effect_get_timer -= 1;
            if self.effect_get_timer == 0 {
                self.effect_get = None;
                world.paused = false;
                player.frozen = false;
                player.frozen_time = 0;
            }
        }

        if self.open {
            self.menu_state.update(input, player, world, save_info, sfx);
        }
    }

    pub fn draw<T: RenderTarget>(&self, player: &Player, canvas: &mut Canvas<T>, save_info: &SaveInfo, state: &RenderState) {
        if self.open || self.menu_state.menu_screenshot {
            match self.menu_state.current_menu {
                MenuType::Home => {
                    let effects_selected = self.menu_state.button_id == 0;
                    let special_selected = self.menu_state.button_id == 1;
                    let me_selected = self.menu_state.button_id == 2;
                    let unknown_selected = self.menu_state.button_id == 3;
                    let quit_selected = self.menu_state.button_id == 4;

                    self.theme.draw_frame_tiled(canvas, 0, 0, 5, 6);
                    let button_width = (16 * 5) - (4 + MENU_BUTTON_PADDING_HORIZ as i32) * 2;
                    let button_x = 4 + MENU_BUTTON_PADDING_HORIZ as i32;
                    let button_start_y = 4 + MENU_BUTTON_PADDING_VERT as i32;
                    let button_height = 14 + MENU_BUTTON_PADDING_VERT as i32;
                    self.theme.draw_button(canvas, button_x, button_start_y, button_width, "Effects", effects_selected, self.menu_state.selection_flash);
                    self.theme.draw_button(canvas, button_x, button_start_y + button_height, button_width, "Special", special_selected, self.menu_state.selection_flash);
                    self.theme.draw_button(canvas, button_x, button_start_y + button_height * 2, button_width, "Me", me_selected, self.menu_state.selection_flash);
                    self.theme.draw_button(canvas, button_x, button_start_y + button_height * 3, button_width, "...", unknown_selected, self.menu_state.selection_flash);
                    self.theme.draw_button(canvas, button_x, button_start_y + button_height * 4, button_width, "Quit", quit_selected, self.menu_state.selection_flash);
                },
                MenuType::Effects => {
                    self.theme.draw_frame_tiled(canvas, 0, 0, state.screen_extents.0 / 16, 2);
                    self.theme.draw_frame_tiled(canvas, 0, 2, state.screen_extents.0 / 16, (state.screen_extents.1 / 16) - 2);
                    if player.unlocked_effects.len() > 0 {
                        let description = player.unlocked_effects[self.menu_state.button_id as usize].description();
                        self.theme.font.draw_string(canvas, description, (11, 11));
                        let start_y = (2 * 16) + 8;
                        let start_x = 8;
                        let button_height = 14 + MENU_BUTTON_PADDING_VERT as i32;
                        let button_width = 200 - 8;
                        for i in 0..player.unlocked_effects.len() {
                            let name = player.unlocked_effects[i].name();
                            if player.dreaming {
                                self.theme.draw_button(canvas, start_x + (button_width * (i as i32 % 2)), start_y + (button_height * (i as i32 / 2)), button_width - 6, name, self.menu_state.button_id as usize == i, self.menu_state.selection_flash);
                            } else {
                                self.theme.draw_button_strikethrough(canvas, start_x + (button_width * (i as i32 % 2)), start_y + (button_height * (i as i32 / 2)), button_width - 6, name, self.menu_state.button_id as usize == i, self.menu_state.selection_flash);
                            }
                        }
                    }
                },
                MenuType::Quit => {
                    let yes_selected = self.menu_state.button_id == 0;
                    let no_selected = self.menu_state.button_id == 1;

                    self.theme.draw_frame_tiled(canvas, ((state.screen_extents.0 / 2) - (16 * 5)) / 16, 64 / 16, 10, 2);
                    let text_width = self.theme.font.string_width("Do you want to quit?");
                    self.theme.font.draw_string(canvas, "Do you want to quit?", ((state.screen_extents.0 as i32 / 2) - text_width as i32 / 2, 64 + 10));
                    self.theme.draw_frame_tiled(canvas, ((state.screen_extents.0 / 2) - (16 * 2)) / 16, 112 / 16, 4, 3);

                    let button_x = (((state.screen_extents.0 as i32 / 2) - (16 * 2)) / 16) * 16 + 4 + MENU_BUTTON_PADDING_HORIZ as i32;
                    let button_start_y = 112 + 6 + MENU_BUTTON_PADDING_VERT as i32;
                    let button_width = (16 * 4) - (4 + MENU_BUTTON_PADDING_HORIZ as i32) * 2;
                    self.theme.draw_button(canvas, button_x, button_start_y, button_width, "Yes", yes_selected, self.menu_state.selection_flash);
                    self.theme.draw_button(canvas, button_x, button_start_y + (14 + MENU_BUTTON_PADDING_VERT as i32), button_width, "No", no_selected, self.menu_state.selection_flash);
                },
                MenuType::SaveConfirm => {
                    let yes_selected = self.menu_state.button_id == 0;
                    let no_selected = self.menu_state.button_id == 1;

                    self.theme.draw_frame_tiled(canvas, ((state.screen_extents.0 / 2) - (16 * 6)) / 16, 64 / 16, 12, 2);
                    let text_width = self.theme.font.string_width("Overwrite this save file?");
                    self.theme.font.draw_string(canvas, "Overwrite this save file?", ((state.screen_extents.0 as i32 / 2) - text_width as i32 / 2, 64 + 10));
                    self.theme.draw_frame_tiled(canvas, ((state.screen_extents.0 / 2) - (16 * 2)) / 16, 112 / 16, 4, 3);

                    let button_x = (((state.screen_extents.0 as i32 / 2) - (16 * 2)) / 16) * 16 + 4 + MENU_BUTTON_PADDING_HORIZ as i32;
                    let button_start_y = 112 + 6 + MENU_BUTTON_PADDING_VERT as i32;
                    let button_width = (16 * 4) - (4 + MENU_BUTTON_PADDING_HORIZ as i32) * 2;
                    self.theme.draw_button(canvas, button_x, button_start_y, button_width, "Yes", yes_selected, self.menu_state.selection_flash);
                    self.theme.draw_button(canvas, button_x, button_start_y + (14 + MENU_BUTTON_PADDING_VERT as i32), button_width, "No", no_selected, self.menu_state.selection_flash);
                },
                MenuType::MainMenu => {
                    let centered_x = (state.screen_extents.0 / 2) - (self.theme.title.width / 2);
                    let y = MAIN_MENU_TITLE_Y;
                    canvas.copy(
                        &self.theme.title.texture, 
                        None, 
                        Rect::new(centered_x as i32, y as i32, self.theme.title.width, self.theme.title.height)
                    ).unwrap();

                    let centered_x = (state.screen_extents.0 / 2) - (MAIN_MENU_WIDTH * 8);
                    let y = MAIN_MENU_Y;
                    self.theme.draw_frame(canvas, centered_x, y, MAIN_MENU_WIDTH, MAIN_MENU_HEIGHT);

                    let new_game_selected = self.menu_state.button_id == 0;
                    let continue_selected = self.menu_state.button_id == 1;
                    let quit_selected = self.menu_state.button_id == 2;

                    let button_x = (centered_x + MENU_BUTTON_PADDING_HORIZ) as i32;
                    let button_y = (y + MENU_BUTTON_PADDING_VERT * 3) as i32;
                    let button_w = (MAIN_MENU_WIDTH as i32 * 16) - (MENU_BUTTON_PADDING_HORIZ as i32 * 2);
                    self.theme.draw_button(canvas, button_x, button_y, button_w, "New Game", new_game_selected, self.menu_state.selection_flash);
                    if !save_info.files.is_empty() {
                        self.theme.draw_button(canvas, button_x, button_y + (MENU_BUTTON_PADDING_VERT as i32 + 14), button_w, "Continue", continue_selected, self.menu_state.selection_flash);
                    } else {
                        self.theme.draw_button_strikethrough(canvas, button_x, button_y + (MENU_BUTTON_PADDING_VERT as i32 + 14), button_w, "Continue", continue_selected, self.menu_state.selection_flash);
                    }
                    self.theme.draw_button(canvas, button_x, button_y + (MENU_BUTTON_PADDING_VERT as i32 + 14) * 2, button_w, "Quit", quit_selected, self.menu_state.selection_flash);
                },
                MenuType::SaveLoad(b) => {
                    self.theme.draw_frame(canvas, 0, 0, state.screen_extents.0 / 16, 2);
                    self.theme.font.draw_string(canvas, if b { "Save Game" } else { "Load Game" }, (11, 11));
                    let mut y = 32;

                    let drawn_files = save_info.files.len() as i32 + if b { 1 } else { 0 };
                    let buttons_on_page = (drawn_files - (self.menu_state.page_index * 3)).min(3);
                    let selected_button = (self.menu_state.page_index * 3) + self.menu_state.button_id;

                    let page_left = self.menu_state.page_index > 0;
                    let page_right = self.menu_state.page_index < ((drawn_files - 1) / 3);

                    for i in 0..buttons_on_page {
                        let id = (i + (self.menu_state.page_index * 3)) as u32;
                        if b && id >= save_info.files.len() as u32 { // New file
                            let slot_message = String::from("Slot ") + &(save_info.files.len() + 1).to_string();
                            self.theme.draw_frame(canvas, 0, y, state.screen_extents.0 / 16, 4);
                            self.theme.draw_button(canvas, 14 + 8, y as i32 + 9, 48, &slot_message, selected_button == save_info.files.len() as i32, self.menu_state.selection_flash);
                            self.theme.font.draw_string(canvas, "New Save", (14 + 8, y as i32 + 9 + 16));
                        } else { // Overwrite
                            let entry = save_info.files.get(&id).unwrap();

                            self.theme.draw_frame(canvas, 0, y, state.screen_extents.0 / 16, 4);
                            let slot_message = String::from("Slot ") + &(id + 1).to_string();
                            let effects_message = entry.effects.to_string() + if entry.effects == 1 { " Effect" } else { " Effects" };
                            self.theme.draw_button(canvas, 14 + 4, y as i32 + 9, 48, &slot_message, selected_button == id as i32, self.menu_state.selection_flash);
                            self.theme.font.draw_string(canvas, "Katrin", (14 + 8, y as i32 + 9 + 16 + 1));
                            self.theme.font.draw_string(canvas, &effects_message, (14 + 8, y as i32 + 9 + 32));
                            canvas.copy(
                                &self.player_preview_texture.texture,
                                None,
                                Rect::new(
                                    100, y as i32 + 8, 48, 48
                                )
                            ).unwrap();
                            y += 64;
                        }
                    }

                    if page_left && self.menu_state.selection_flash {
                        self.theme.draw_element(canvas, 4, 32 + 64 + 24, MENU_ARROW_LEFT);
                    }
                    if page_right && self.menu_state.selection_flash {
                        self.theme.draw_element(canvas, state.screen_extents.0 as i32 - (4 + 16), 32 + 64 + 24, MENU_ARROW_RIGHT);
                    }
                },
                MenuType::Special => {
                    self.theme.draw_frame(canvas, 0, 0, state.screen_extents.0 / 16, 2);
                    self.theme.draw_frame(canvas, 0, 32, state.screen_extents.0 / 16, 6);
                    let buttons_x = 6;
                    let buttons_y = 32 + 6;
                    let buttons_width = state.screen_extents.0 - 16;
                    if player.dreaming {
                        self.theme.draw_button(canvas, 6, 32 + 6, buttons_width as i32, "Wake Up", self.menu_state.button_id == 0, self.menu_state.selection_flash);
                    } else {
                        self.theme.draw_button_strikethrough(canvas, buttons_x, buttons_y, buttons_width as i32, "Wake Up", self.menu_state.button_id == 0, self.menu_state.selection_flash);
                    }
                }
                _ => {
                    let width = self.theme.font.string_width("...");
                    self.theme.font.draw_string(canvas, "...", ((state.screen_extents.0 as i32 / 2) - (width as i32 / 2), (state.screen_extents.1 as i32 / 2) - (self.theme.font.char_height as i32 / 2)));
                }
            }
        }

        if let Some(str) = &self.effect_get {
            self.theme.clear_frame(canvas, ((state.screen_extents.0 / 2) - (16 * 4)) / 16, 150 / 16, 8, 2);
            self.theme.draw_frame_tiled(canvas, ((state.screen_extents.0 / 2) - (16 * 4)) / 16, 150 / 16, 8, 2);
            let text_width = self.theme.font.string_width(str);
            self.theme.font.draw_string(canvas, str, ((state.screen_extents.0 / 2) as i32 - text_width as i32 / 2, 156));
        }
    }
}

pub struct MenuSet<'a> {
    pub tileset: Tileset<'a>,
    pub font: Font<'a>,
    pub title: Texture<'a>
}

impl<'a> MenuSet<'a> {
    pub fn from_tileset<T>(tileset: Tileset<'a>, font: Option<&str>, creator: &'a TextureCreator<T>) -> Self {
        Self {
            tileset,
            title: Texture::from_file(&PathBuf::from(MAIN_MENU_TITLE), &creator).map_err(|e| format!("failed to load title texture: {}", e)).unwrap(),
            font: Font::load_from_file(
                &PathBuf::from(font.unwrap_or(DEFAULT_FONT)), 
                creator, 
                DEFAULT_FONT_WIDTH, 
                DEFAULT_FONT_HEIGHT, 
                None)
        }
    }

    pub fn clear_frame<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: u32, y: u32, w: u32, h: u32) {
        let draw_x = (x as i32 * 16) + 2;
        let draw_y = (y as i32 * 16) + 2;
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.set_blend_mode(sdl2::render::BlendMode::None);
        canvas.fill_rect(Rect::new(draw_x, draw_y, (w * 16) - 4, (h * 16) - 4)).unwrap();
    }

    /// Draw a frame at (x, y) with size (w, h) in tiles
    pub fn draw_frame<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: u32, y: u32, w: u32, h: u32) {
        let draw_x = x as i32;
        let draw_y = y as i32;
        self.tileset.draw_tile(canvas, MENU_FRAME_TOP_RIGHT, (draw_x, draw_y));
        self.tileset.draw_tile(canvas, MENU_FRAME_TOP_LEFT, (draw_x + ((w as i32 - 1) * 16), draw_y));
        self.tileset.draw_tile(canvas, MENU_FRAME_BOTTOM_LEFT, (draw_x, draw_y + ((h as i32 - 1) * 16)));
        self.tileset.draw_tile(canvas, MENU_FRAME_BOTTOM_RIGHT, (draw_x + ((w as i32 - 1) * 16), draw_y + ((h as i32 - 1) * 16)));
        for i in 1..(w - 1) {
            self.tileset.draw_tile(canvas, MENU_FRAME_TOP, (draw_x + (i as i32 * 16), draw_y));
            self.tileset.draw_tile(canvas, MENU_FRAME_BOTTOM, (draw_x + (i as i32 * 16), draw_y + ((h as i32 - 1) * 16)));
        }
        for i in 1..(h - 1) {
            self.tileset.draw_tile(canvas, MENU_FRAME_LEFT, (draw_x, draw_y + (i as i32 * 16)));
            self.tileset.draw_tile(canvas, MENU_FRAME_RIGHT, (draw_x + ((w as i32 - 1) * 16), draw_y + (i as i32 * 16)));
        }
    }

    /// Draw a frame aligned to the tile grid starting at (0, 0)
    pub fn draw_frame_tiled<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: u32, y: u32, w: u32, h: u32) {
        self.draw_frame(canvas, x * 16, y * 16, w, h);
    }

    pub fn draw_button<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: i32, y: i32, w: i32, text: &str, selected: bool, flash: bool) {
        if selected {
            if flash {
                let tiles_flash = w / 16;
                let start = (x + w / 2) - ((tiles_flash * 16) / 2);
                for tile_x in 0..tiles_flash {
                    self.tileset.draw_tile(canvas, MENU_SELECTION_HIGHLIGHT, (start + (tile_x * 16), y));
                }
            }

            self.tileset.draw_tile(canvas, MENU_SELECTION_BORDER_LEFT, (x, y));
            self.tileset.draw_tile(canvas, MENU_SELECTION_BORDER_RIGHT, (x + w - 16, y));
        }

        self.font.draw_string(canvas, text, (x + 4, y + 3));
    }

    pub fn draw_element<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: i32, y: i32, tile: u32) {
        self.tileset.draw_tile(canvas, tile, (x, y));
    }

    pub fn draw_button_strikethrough<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: i32, y: i32, w: i32, text: &str, selected: bool, flash: bool) {
        if selected {
            if flash {
                let tiles_flash = w / 16;
                let start = (x + w / 2) - ((tiles_flash * 16) / 2);
                for tile_x in 0..tiles_flash {
                    self.tileset.draw_tile(canvas, MENU_SELECTION_HIGHLIGHT, (start + (tile_x * 16), y));
                }
            }

            self.tileset.draw_tile(canvas, MENU_SELECTION_BORDER_LEFT, (x, y));
            self.tileset.draw_tile(canvas, MENU_SELECTION_BORDER_RIGHT, (x + w - 16, y));
        }

        self.font.draw_string_strikethrough(canvas, text, (x + 4, y + 3));
    }
}

pub struct Font<'a> {
    pub texture: Texture<'a>,
    pub chars: String,
    pub char_width: u32,
    pub char_height: u32,
    pub image_chars_width: u32,
    pub chars_map: HashMap<char, (u32, u32)>,
    pub char_spacing: (u32, u32)
}

impl<'a> Font<'a> {
    pub fn new(texture: Texture<'a>, char_width: u32, char_height: u32, chars: Option<&str>) -> Self {
        let width = texture.width / char_width;
        let height = texture.height / char_height;
        let chars_key = chars.unwrap_or(FONT_CHARS).to_string();
        let chars_vec = chars_key.chars().collect::<Vec<char>>();
        let mut map = HashMap::new();

        let mut i = 0;
        'outer: for y in 0..height {
            for x in 0..width {
                if i >= chars_vec.len() {
                    break 'outer;
                }
                map.insert(chars_vec[i], (x * char_width, y * char_height));
                i += 1;
            }
        }

        Self {
            texture,
            char_height,
            char_width,
            chars: chars_key,
            image_chars_width: width,
            chars_map: map,
            char_spacing: (DEFAULT_FONT_SPACING_HORIZ, DEFAULT_FONT_SPACING_VERT)
        }
    }

    pub fn new_mini(texture: Texture<'a>) -> Self {
        let width = texture.width / MINIFONT_FONT_WIDTH;
        let height = texture.height / MINIFONT_FONT_HEIGHT;
        let chars = MINIFONT_CHARS.to_string().chars().collect::<Vec<char>>();
        let mut map = HashMap::new();

        let mut i = 0;
        'outer: for y in 0..height {
            for x in 0..width {
                if i >= chars.len() {
                    break 'outer;
                }
                map.insert(chars[i], (x * MINIFONT_FONT_WIDTH, y * MINIFONT_FONT_HEIGHT));
                i += 1;
            }
        }

        Self {
            texture,
            char_height: MINIFONT_FONT_HEIGHT,
            char_width: MINIFONT_FONT_WIDTH,
            char_spacing: (MINIFONT_FONT_SPACING_HORIZ, MINIFONT_FONT_SPACING_VERT),
            chars: MINIFONT_CHARS.to_string(),
            chars_map: map,
            image_chars_width: MINIFONT_FONT_WIDTH
        }
    }

    pub fn load_from_file<T>(file: &PathBuf, creator: &'a TextureCreator<T>, char_width: u32, char_height: u32, chars: Option<&str>) -> Self {
        let texture =
            Texture::from_file(file, creator).map_err(|e| format!("failed to load font texture: {}", e)).unwrap();
        Self::new(texture, char_width, char_height, chars)
    }

    pub fn draw_char<T: RenderTarget>(&self, canvas: &mut Canvas<T>, char: char, pos: (i32, i32)) {
        if let Some(char_pos) = self.chars_map.get(&char) {
            canvas.copy(&self.texture.texture, 
                Rect::new(char_pos.0 as i32, char_pos.1 as i32, self.char_width, self.char_height), 
                Rect::new(pos.0, pos.1, self.char_width, self.char_height)
            ).unwrap();
        }
    }

    pub fn string_width(&self, string: &str) -> u32 {
        return string.len() as u32 * (self.char_width + self.char_spacing.0);
    }

    pub fn draw_string<T: RenderTarget,>(&self, canvas: &mut Canvas<T>, message: &str, pos: (i32, i32)) {
        let chars = message.chars().collect::<Vec<char>>();
        for i in 0..chars.len() {
            self.draw_char(canvas, chars[i], (pos.0 + ((self.char_width + self.char_spacing.0) * i as u32) as i32, pos.1));
        }
    }

    pub fn draw_string_strikethrough<T: RenderTarget>(&self, canvas: &mut Canvas<T>, message: &str, pos: (i32, i32)) {
        let chars = message.chars().collect::<Vec<char>>();
        for i in 0..chars.len() {
            self.draw_char(canvas, chars[i], (pos.0 + ((self.char_width + self.char_spacing.0) * i as u32) as i32, pos.1));
            self.draw_char(canvas, '§', (pos.0 + ((self.char_width + self.char_spacing.0) * i as u32) as i32, pos.1));
        }
    }

    pub fn draw_string_wrapped<T: RenderTarget>(&self, canvas: &mut Canvas<T>, string: &str, pos: (i32, i32), width: u32) {
        let mut x = pos.0;
        let mut y = pos.1;
        let chars = string.chars().collect::<Vec<char>>();
        let spacing_x = (self.char_width + self.char_spacing.0) as i32;

        for i in 0..chars.len() {
            self.draw_char(canvas, chars[i], (x, y));
            x += spacing_x;

            if (x + spacing_x as i32) - pos.0 > width as i32 {
                y += (self.char_height + self.char_spacing.1) as i32;
                x = pos.0;
            }
        }
    }
}