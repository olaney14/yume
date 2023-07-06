use std::{path::PathBuf, collections::HashMap};

use rodio::Sink;
use sdl2::{render::{RenderTarget, Canvas, TextureCreator}, rect::Rect, keyboard::Keycode};

use crate::{tiles::Tileset, texture::Texture, game::Input, effect::Effect, player::{Player, self}, audio::SoundEffectBank};

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
const MENU_BUTTON_PADDING_VERT: u32 = 2;
const MENU_BUTTON_PADDING_HORIZ: u32 = 2;

const FONT_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .!,-�?";
const DEFAULT_FONT: &str = "res/textures/ui/fonts/font.png";
const DEFAULT_FONT_WIDTH: u32 = 6;
const DEFAULT_FONT_HEIGHT: u32 = 10;
const DEFAULT_FONT_SPACING_HORIZ: u32 = 0;
const DEFAULT_FONT_SPACING_VERT: u32 = 1;

const FONT_VINES: &str = "res/textures/ui/fonts/vines.png";

const BUTTONS_MAIN: u32 = 5;

const SFX_VOLUME: f32 = 0.7;

pub enum MenuType {
    Home,
    Effects,
    Special,
    Me,
    Quit
}

pub struct MenuState {
    pub close_on_x: bool,
    pub current_menu: MenuType,
    pub button_id: i32,
    pub selection_flash: bool,
    pub timer: u32,
    pub should_quit: bool,
    pub menu_should_close: bool
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            close_on_x: true,
            current_menu: MenuType::Home,
            button_id: 0,
            selection_flash: true,
            timer: 0,
            should_quit: false,
            menu_should_close: false
        }
    }

    pub fn update(&mut self, input: &Input, player: &mut Player, sfx: &mut SoundEffectBank) {
        if input.get_just_pressed(Keycode::X) {
            match self.current_menu {
                MenuType::Effects | MenuType::Quit | MenuType::Special | MenuType::Me => {
                    if matches!(self.current_menu, MenuType::Effects) { self.button_id = 0; }
                    if matches!(self.current_menu, MenuType::Special) { self.button_id = 1; }
                    if matches!(self.current_menu, MenuType::Me) { self.button_id = 2; }
                    if matches!(self.current_menu, MenuType::Quit) { self.button_id = 4; }
                    sfx.play("menu_blip_negative");

                    self.current_menu = MenuType::Home;
                    self.close_on_x = true;
                },
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
                            // Quit :(
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
                        if player.current_effect.is_some() && player.current_effect.as_ref().unwrap() == &player.unlocked_effects[self.button_id as usize] {
                            player.remove_effect();
                        } else {
                            player.apply_effect(player.unlocked_effects[self.button_id as usize].clone());
                        }
                        self.current_menu = MenuType::Home;
                        self.menu_should_close = true;
                        sfx.play("effect");
                    }
                },
                MenuType::Special => {
                    sfx.play("menu_blip_error");
                },
                MenuType::Quit => {
                    match self.button_id {
                        0 => {
                            // Yes
                            self.should_quit = true;
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
            MenuType::Quit => {
                if input.get_just_pressed(Keycode::Up) { self.button_id -= 1; }
                if input.get_just_pressed(Keycode::Down) { self.button_id += 1; }
                if self.button_id > 1 {
                    self.button_id = 0;
                }
                if self.button_id < 0 {
                    self.button_id = 1;
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
    pub menu_state: MenuState
}

impl<'a> Ui<'a> {
    pub fn new<T>(theme: &PathBuf, font: Option<&str>, creator: &'a TextureCreator<T>) -> Self {
        let tileset = Tileset::load_from_file(theme, creator);
        Self {
            theme: MenuSet::from_tileset(tileset, font, creator),
            clear: false,
            open: false,
            menu_state: MenuState::new()
        }
    }

    pub fn init(&self, sfx: &mut SoundEffectBank) {
        sfx.load(&String::from("menu_blip_affirmative"), SFX_VOLUME, 1.0);
        sfx.load(&String::from("menu_blip_negative"), SFX_VOLUME, 1.0);
        sfx.load(&String::from("menu_blip_error"), SFX_VOLUME, 1.0);
    }

    pub fn update(&mut self, input: &Input, player: &mut Player, sink: &Sink, sfx: &mut SoundEffectBank) {
        if input.get_just_pressed(Keycode::X) {
            if self.open && self.menu_state.close_on_x {
                //sink.play();
                sink.set_volume(sink.volume() * 5.0);
                self.open = false;
                self.clear = false;
                sfx.play("menu_blip_negative");
            } else if !self.open && !player.moving {
                //sink.pause();
                sink.set_volume(sink.volume() / 5.0);
                self.open = true;  
                self.clear = true;
                self.menu_state.button_id = 0;
                self.menu_state.close_on_x = true;
                sfx.play("menu_blip_affirmative");
            }
        }

        if self.menu_state.menu_should_close && self.open {
            sink.set_volume(sink.volume() * 5.0);
            self.open = false;
            self.clear = false;
            self.menu_state.menu_should_close = false;
        }

        if self.open {
            self.menu_state.update(input, player, sfx);
        }
    }

    pub fn draw<T: RenderTarget>(&self, player: &Player, canvas: &mut Canvas<T>) {
        if self.open {
            match self.menu_state.current_menu {
                MenuType::Home => {
                    let effects_selected = self.menu_state.button_id == 0;
                    let special_selected = self.menu_state.button_id == 1;
                    let me_selected = self.menu_state.button_id == 2;
                    let unknown_selected = self.menu_state.button_id == 3;
                    let quit_selected = self.menu_state.button_id == 4;

                    self.theme.draw_frame(canvas, 0, 0, 5, 6);
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
                    self.theme.draw_frame(canvas, 0, 0, 25, 2);
                    self.theme.draw_frame(canvas, 0, 2, 25, 16);
                    if player.unlocked_effects.len() > 0 {
                        let description = player.unlocked_effects[self.menu_state.button_id as usize].description();
                        self.theme.font.draw_string(canvas, description, (8, 8));
                        let start_y = (2 * 16) + 8;
                        let start_x = 8;
                        let button_height = 14 + MENU_BUTTON_PADDING_VERT as i32;
                        let button_width = 200 - 8;
                        for i in 0..player.unlocked_effects.len() {
                            let name = player.unlocked_effects[i].name();
                            self.theme.draw_button(canvas, start_x + (button_width * (i as i32 % 2)), start_y + (button_height * (i as i32 / 2)), button_width - 6, name, self.menu_state.button_id as usize == i, self.menu_state.selection_flash);
                        }
                    }
                },
                MenuType::Quit => {
                    let yes_selected = self.menu_state.button_id == 0;
                    let no_selected = self.menu_state.button_id == 1;

                    self.theme.draw_frame(canvas, (200 - (16 * 5)) / 16, 100 / 16, 10, 2);
                    self.theme.font.draw_string(canvas, "Do you want to quit?", (200 - (16 * 4) - 4, 100 + 6));
                    self.theme.draw_frame(canvas, (200 - (16 * 2)) / 16, 150 / 16, 4, 3);

                    let button_x = ((200 - (16 * 2)) / 16) * 16 + 4 + MENU_BUTTON_PADDING_HORIZ as i32;
                    let button_start_y = 150 + MENU_BUTTON_PADDING_VERT as i32;
                    let button_width = (16 * 4) - (4 + MENU_BUTTON_PADDING_HORIZ as i32) * 2;
                    self.theme.draw_button(canvas, button_x, button_start_y, button_width, "Yes", yes_selected, self.menu_state.selection_flash);
                    self.theme.draw_button(canvas, button_x, button_start_y + (14 + MENU_BUTTON_PADDING_VERT as i32), button_width, "No", no_selected, self.menu_state.selection_flash);
                },
                _ => {
                    let width = self.theme.font.string_width("under construction...");
                    self.theme.font.draw_string(canvas, "under construction...", (200 - (width as i32 / 2), 150 - (self.theme.font.char_height as i32 / 2)));
                }
            }
        }
    }
}

pub struct MenuSet<'a> {
    pub tileset: Tileset<'a>,
    pub font: Font<'a>
}

impl<'a> MenuSet<'a> {
    pub fn from_tileset<T>(tileset: Tileset<'a>, font: Option<&str>, creator: &'a TextureCreator<T>) -> Self {
        Self {
            tileset,
            font: Font::load_from_file(
                &PathBuf::from(font.unwrap_or(DEFAULT_FONT)), 
                creator, 
                DEFAULT_FONT_WIDTH, 
                DEFAULT_FONT_HEIGHT, 
                None)
        }
    }

    pub fn draw_frame<T: RenderTarget>(&self, canvas: &mut Canvas<T>, x: u32, y: u32, w: u32, h: u32) {
        let draw_x = x as i32 * 16;
        let draw_y = y as i32 * 16;
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

    pub fn draw_string<T: RenderTarget>(&self, canvas: &mut Canvas<T>, string: &str, pos: (i32, i32)) {
        let chars = string.chars().collect::<Vec<char>>();
        for i in 0..chars.len() {
            self.draw_char(canvas, chars[i], (pos.0 + ((self.char_width + self.char_spacing.0) * i as u32) as i32, pos.1));
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