use std::{path::PathBuf, sync::Arc, collections::HashMap};

use rodio::OutputStreamHandle;
use sdl2::{render::{TextureCreator, RenderTarget, Canvas}, rect::Rect, keyboard::Keycode};

use crate::{texture::Texture, game::{Direction, Input, RenderState}, world::World, effect::Effect};

pub const SWITCH_EFFECT_ANIMATION_SPEED: u32 = 2;

pub struct Player<'a> {
    pub x: i32,
    pub y: i32,
    pub texture: Texture<'a>,
    pub effects_texture: Texture<'a>,
    pub facing: Direction,
    pub moving: bool,
    pub speed: u32,
    pub move_timer: i32,
    pub animation_info: AnimationInfo,
    pub last_direction: Option<Direction>,
    pub layer: i32,
    pub draw_over: bool,
    pub occupied_tile: (u32, u32),
    pub frozen: bool,
    pub unlocked_effects: Vec<Effect>,
    pub current_effect: Option<Effect>,
    pub frozen_time: u32,
    pub effect_textures: HashMap<Effect, Texture<'a>>
}

pub struct AnimationInfo {
    pub frame_row: u32,
    pub frame: u32,
    pub frame_direction: i32,
    pub animation_speed: u32,
    pub animation_timer: i32,
    pub effect_switch_animation: u32,
    pub effect_switch_animation_timer: u32,
}

impl AnimationInfo {
    pub fn new() -> Self {
        Self {
            frame_row: 1, frame: 1, frame_direction: 1, animation_speed: 7, animation_timer: 3,
            effect_switch_animation: 0, effect_switch_animation_timer: 0
        }
    }

    pub fn animate_effects(&mut self) {
        if self.effect_switch_animation > 0 && self.effect_switch_animation_timer > 0 {
            self.effect_switch_animation_timer -= 1;
            if self.effect_switch_animation_timer == 0 {
                self.effect_switch_animation -= 1;
                if self.effect_switch_animation > 0 {
                    self.effect_switch_animation_timer = SWITCH_EFFECT_ANIMATION_SPEED;
                }
            }
        }
    }

    pub fn animate_walk(&mut self) {
        if self.animation_timer <= 0 {
            // TODO: frame origin and frame max
            if 1_i32.abs_diff(self.frame as i32) == 1 {
                self.frame_direction *= -1;
            }

            self.frame = (self.frame as i32 + self.frame_direction).try_into().expect("bad animation frame");
            self.animation_timer = self.animation_speed as i32;
        } else {
            self.animation_timer -= 1;
        }
    }

    pub fn stop(&mut self) {
        // TODO: frame origin
        self.frame = 1;
    }

    pub fn get_frame_pos(&self) -> (u32, u32) {
        (self.frame * 16, self.frame_row * 32)
    }
}

pub const MOVE_TIMER_MAX: i32 = 16;

impl<'a> Player<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>) -> Self {
        let mut player = Self {
            x: 0, y: 0,
            effects_texture: Texture::from_file(&PathBuf::from("res/textures/misc/effects.png"), creator).expect("failed to load effects texture"),
            texture: Texture::from_file(&PathBuf::from("res/textures/player/player.png"), creator).expect("failed to load player texture"),
            facing: Direction::Down,
            moving: false,
            speed: 1,
            move_timer: 0,
            animation_info: AnimationInfo::new(),
            last_direction: None,
            layer: 0,
            draw_over: false,
            occupied_tile: (0, 1),
            frozen: false,
            unlocked_effects: Vec::new(),
            current_effect: None,
            frozen_time: 0,
            effect_textures: HashMap::new()
        };

        player.load_effect_textures(creator);

        player
    }

    fn load_effect_textures<T>(&mut self, creator: &'a TextureCreator<T>) {
        self.effect_textures.insert(Effect::Glasses, Texture::from_file(&PathBuf::from("res/textures/player/glasses.png"), creator).unwrap());
    }

    pub fn set_x(&mut self, x: i32) {
        self.x = x;
        self.occupied_tile.0 = (self.x / 16).max(0) as u32;
    }

    pub fn set_y(&mut self, y: i32) {
        self.y = y;
        self.occupied_tile.1 = (self.y / 16).max(0) as u32 + 1;
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.set_x(x);
        self.set_y(y);
    }

    pub fn move_player(&mut self, direction: Direction, world: &mut World, force: bool, just_pressed: bool) {
        if !self.moving || force {
            if self.can_move_in_direction(direction, world) && !self.frozen {
                self.moving = true;
                self.move_timer = MOVE_TIMER_MAX;
                self.occupied_tile.0 = (self.occupied_tile.0 as i32 + direction.x()) as u32;
                self.occupied_tile.1 = (self.occupied_tile.1 as i32 + direction.y()) as u32;

                if !force {
                    self.animation_info.frame = 1;
                }
            } else {
                let pos = self.get_standing_tile();
                let target_pos = (pos.0 as i32 + direction.x(), pos.1 as i32 + direction.y());
                if world.looping &&
                (target_pos.0 < 0 || target_pos.1 < 0 || target_pos.0 >= world.width as i32 || target_pos.1 >= world.height as i32) {
                    if target_pos.0 < 0 { // left
                        self.x = world.width as i32 * 16;
                        self.occupied_tile.0 = world.width - 1;
                        self.occupied_tile.1 = (self.occupied_tile.1 as i32 + direction.y()) as u32;
                    } else if target_pos.0 >= world.width as i32 { // right
                        self.x = -16;
                        self.occupied_tile.0 = 0;
                        self.occupied_tile.1 = (self.occupied_tile.1 as i32 + direction.y()) as u32;
                    } else if target_pos.1 < 0 { // up
                        self.y = world.height as i32 * 16 - 16;
                        self.occupied_tile.0 = (self.occupied_tile.0 as i32 + direction.x()) as u32;
                        self.occupied_tile.1 = world.height - 1;
                    } else if target_pos.1 >= world.height as i32 { // down
                        self.y = -32;
                        self.occupied_tile.0 = (self.occupied_tile.0 as i32 + direction.x()) as u32;
                        self.occupied_tile.1 = 0;
                    }
                    self.moving = true;
                    self.move_timer = MOVE_TIMER_MAX;
                    self.draw_over = true;
                } else {
                    self.animation_info.frame = 1;
                    let player_pos = self.get_standing_tile();
                    if just_pressed || force {
                        world.player_bump(player_pos.0 as i32 + direction.x(), player_pos.1 as i32 + direction.y());
                    }
                }
            }

            if !self.frozen {
                self.facing = direction;
                self.animation_info.frame_row = match direction {
                    Direction::Down => 1,
                    Direction::Left => 2,
                    Direction::Right => 0,
                    Direction::Up => 3
                };
            }
        }
    }

    pub fn movement_check(&mut self, input: &Input, world: &mut World, force: bool) -> bool {
        use Keycode::*;

        let directions_pressed: Vec<Direction> = [Up, Down, Left, Right]
            .iter()
            .filter(|key| input.get_pressed(**key))
            .map(Direction::from_key)
            .filter(Option::is_some)
            .map(|x| x.unwrap())
            .collect();

        if directions_pressed.len() > 1 {
            assert!(self.last_direction.is_some());
            let last_pressed = directions_pressed.iter()
                .find(|dir| **dir == self.last_direction.unwrap());
            if let Some(last) = last_pressed {
                self.move_player(*last, world, force, input.get_just_pressed(last.to_key().unwrap_or(Keycode::PrintScreen)));
                return true;
            }
        } else if directions_pressed.len() == 1 {
            let direction = directions_pressed.first().unwrap();
            self.move_player(*direction, world, force, input.get_just_pressed(direction.to_key().unwrap_or(Keycode::PrintScreen)));
            return true;
        }

        return false;
    }

    pub fn can_move_in_direction(&mut self, direction: Direction, world: &World) -> bool {
        let pos = self.get_standing_tile();
        let target_pos = (pos.0 as i32 + direction.x(), pos.1 as i32 + direction.y());
        if target_pos.0 < 0 || target_pos.1 < 0 || target_pos.0 >= world.width as i32 || target_pos.1 >= world.height as i32 {
            return false;
        }
        return !world.get_collision_at_tile(target_pos.0 as u32, target_pos.1 as u32, self.layer);
    }

    pub fn apply_effect(&mut self, effect: Effect) {
        effect.apply(self);
        self.current_effect = Some(effect);
        self.frozen_time = 32;
        self.animation_info.effect_switch_animation = 8;
        self.animation_info.effect_switch_animation_timer = SWITCH_EFFECT_ANIMATION_SPEED;
    }

    pub fn remove_effect(&mut self) {
        if self.current_effect.is_some() {
            let mut effect = self.current_effect.take().unwrap();
            effect.remove(self);
            self.frozen_time = 32;
            self.animation_info.effect_switch_animation = 8;
            self.animation_info.effect_switch_animation_timer = SWITCH_EFFECT_ANIMATION_SPEED;
        }
    }

    pub fn update(&mut self, input: &Input, world: &mut World) {
        {
            use Keycode::*;
            for key in [Up, Down, Left, Right, W, A, S, D].into_iter() {
                if input.get_just_pressed(key) {
                    self.last_direction = Direction::from_key(&key);
                    break;
                }
            }
        }

        if self.frozen_time > 0 {
            self.frozen = true;
            self.frozen_time -= 1;
            if self.frozen_time == 0 {
                self.frozen = false;
            }
        }

        self.animation_info.animate_effects();

        if self.moving {
            self.x += self.facing.x() * self.speed as i32;
            self.y += self.facing.y() * self.speed as i32;
            self.move_timer -= self.speed as i32;
            self.animation_info.animate_walk();
            if self.move_timer <= 0 {
                self.x = (self.x as f32 / 16.0).round() as i32 * 16;
                self.y =  (self.y as f32 / 16.0).round() as i32 * 16;
                self.moving = false;
                self.draw_over = false;
                if !self.movement_check(input, world, true) {
                    self.animation_info.stop();
                }
            }
        } else {
            self.movement_check(input, world, false);
            if input.get_just_pressed(Keycode::Z) && world.interaction.is_none() {
                let pos = self.get_standing_tile();
                world.interaction = Some(crate::world::Interaction::Use(pos.0 as i32 + self.facing.x(), pos.1 as i32 + self.facing.y()));
            }
        }
    }

    pub fn get_standing_tile(&self) -> (u32, u32) {
        (
            (self.x / 16).max(0) as u32,
            ((self.y / 16) + 1).max(0) as u32
        )
    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, state: &RenderState) {
        let source = self.animation_info.get_frame_pos();
        let x;
        let y;
        
        if state.clamp.0 {
            x = self.x + state.offset.0;
        } else {
            x = (state.screen_extents.0 as i32 / 2) - 8;
        }

        if state.clamp.1 {
            y = self.y + state.offset.1;
        } else {
            y = (state.screen_extents.1 as i32 / 2) - 16;
        }

        if self.current_effect.is_some() {
            if let Some(texture) = self.effect_textures.get(self.current_effect.as_ref().unwrap()) {
                canvas.copy(&texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
            } else {
                canvas.copy(&self.texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
            }
        } else {
            canvas.copy(&self.texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
        }

        if self.animation_info.effect_switch_animation > 0 {
            let frame = 8 - self.animation_info.effect_switch_animation;
            canvas.copy(
                &self.effects_texture.texture, 
                Rect::new(48 * frame as i32, 0, 48, 48), 
                Rect::new(x - 24 + 8, y - 24 + 16, 48, 48)
            ).unwrap();
        }
    }

    pub fn draw_looping<T: RenderTarget>(&self, canvas: &mut Canvas<T>, _state: &RenderState) {
        let source = self.animation_info.get_frame_pos();
        canvas.copy(&self.texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(self.x, self.y, 16, 32)).unwrap();
    }
}