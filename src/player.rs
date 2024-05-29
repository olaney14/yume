use std::{path::PathBuf, collections::HashMap};

use sdl2::{render::{TextureCreator, RenderTarget, Canvas}, rect::Rect, keyboard::Keycode};
use serde_derive::{Serialize, Deserialize};

use crate::{audio::SoundEffectBank, effect::Effect, game::{Direction, Input, RenderState}, texture::Texture, tiles::SpecialTile, world::World};

pub const SWITCH_EFFECT_ANIMATION_SPEED: u32 = 2;

pub struct Player<'a> {
    pub x: i32,
    pub y: i32,
    pub texture: Texture<'a>,
    pub effects_texture: Texture<'a>,
    pub facing: Direction,
    pub diag_move: i32,
    pub moving: bool,
    pub speed: u32,
    pub move_delay: u32,
    pub move_timer: i32,
    pub move_delay_timer: i32,
    pub animation_info: AnimationInfo,
    pub animation_override_controller: AnimationOverrideController,
    pub last_direction: Option<Direction>,
    pub layer: i32,
    pub draw_over: bool,
    pub occupied_tile: (u32, u32),
    pub frozen: bool,
    pub unlocked_effects: Vec<Effect>,
    pub current_effect: Option<Effect>,
    pub frozen_time: u32,
    pub disable_player_input_time: u32,
    pub effect_textures: HashMap<Effect, Texture<'a>>,
    pub extra_textures: ExtraTextures<'a>,
    pub effect_just_changed: bool,
    pub money: u32,
    pub stats: Statistics,
    pub save_slot: u32,
    pub dreaming: bool,
    pub disable_player_input: bool,
    pub last_effect: Option<Effect>,
    pub reset_layer_on_stop: Option<i32>,
    pub exit_bed_direction: Option<Direction>,
    pub no_snap_on_stop: bool,
    pub check_walkable_on_next_frame: bool,
    pub speed_mod: i32,
    pub on_ladder: bool
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Statistics {
    pub steps: u64,
    pub times_slept: u32
}

pub struct AnimationOverrideController {
    pub active: bool,
    pub texture: PlayerTextureSheet,
    pub frame_pos: (u32, u32),
    pub sit_animation: bool,
    pub lay_down_animation: bool,
    pub draw_offset: (i32, i32)
}

impl AnimationOverrideController {
    pub fn do_sit(&mut self) {
        self.sit_animation = true;
        self.frame_pos = (0, 0);
        self.texture = PlayerTextureSheet::Other;
    }

    pub fn do_lay_down(&mut self) {
        self.lay_down_animation = true;
        self.frame_pos = (16, 32);
        self.texture = PlayerTextureSheet::Player;
    }

    pub fn new() -> Self {
        Self {
            active: false,
            frame_pos: (0, 0),
            sit_animation: false,
            lay_down_animation: false,
            texture: PlayerTextureSheet::Player,
            draw_offset: (0, 0)
        }
    }
}

pub struct AnimationInfo {
    pub frame_row: u32,
    pub frame: u32,
    pub frame_direction: i32,
    pub animation_speed: u32,
    pub animation_timer: i32,
    pub effect_switch_animation: u32,
    pub effect_switch_animation_timer: u32,
    pub do_step: bool,
}

impl AnimationInfo {
    pub fn new() -> Self {
        Self {
            frame_row: 1, frame: 1, frame_direction: 1, animation_speed: 7, animation_timer: 3,
            effect_switch_animation: 0, effect_switch_animation_timer: 0,
            do_step: false
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

            if self.frame == 1 {
                self.do_step = true;
            }

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

    pub fn get_ladder_frame_pos(&self) -> (u32, u32) {
        (self.frame * 16, 3 * 32)
    }
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            steps: 0,
            times_slept: 0
        }
    }
}

pub const MOVE_TIMER_MAX: i32 = 16;

pub enum PlayerTextureSheet {
    Player,
    Effect,
    Fire,
    Other
}

pub struct ExtraTextures<'a> {
    pub fire: Texture<'a>,
    pub other: Texture<'a>,
    pub fire_frame: u32,
    pub fire_timer: u32
}

impl<'a> ExtraTextures<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>) -> Self {
        let fire = Texture::from_file(&PathBuf::from("res/textures/player/fire_sheet.png"), creator).expect("could not load \"res/textures/player/fire_sheet.png\"");
        let other = Texture::from_file(&PathBuf::from("res/textures/player/other.png"), creator).expect("could not load \"res/textures/player/other.png\"");
        Self { 
            fire, fire_frame: 0, fire_timer: 5,
            other
        }
    }

    pub fn animate(&mut self) {
        self.fire_timer -= 1;
        if self.fire_timer == 0 {
            self.fire_timer = 5;
            self.fire_frame += 1;
            if self.fire_frame > 2 {
                self.fire_frame = 0;
            }
        }
    }

    pub fn get_frame_pos_back(&self) -> (u32, u32) {
        return (self.fire_frame * 32, 0);
    }

    pub fn get_frame_pos_front(&self) -> (u32, u32) {
        return (self.fire_frame * 32, 48);
    }
}

// enum MovementIgnoreParams {
//     None,
//     IgnoreWorld,
//     IgnoreFrozen,
//     IgnoreAll
// }

impl<'a> Player<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>) -> Self {
        let mut player = Self {
            x: 0, y: 0,
            effects_texture: Texture::from_file(&PathBuf::from("res/textures/misc/effects.png"), creator).expect("failed to load effects texture"),
            texture: Texture::from_file(&PathBuf::from("res/textures/player/player.png"), creator).expect("failed to load player texture"),
            facing: Direction::Down,
            moving: false,
            speed: 1,
            move_delay: 0,
            move_timer: 0,
            animation_info: AnimationInfo::new(),
            animation_override_controller: AnimationOverrideController::new(),
            last_direction: None,
            layer: 0,
            draw_over: false,
            occupied_tile: (0, 1),
            frozen: false,
            unlocked_effects: Vec::new(),
            current_effect: None,
            frozen_time: 0,
            effect_textures: HashMap::new(),
            extra_textures: ExtraTextures::new(creator),
            diag_move: 0,
            effect_just_changed: false,
            stats: Statistics::new(),
            money: 0,
            save_slot: 0,
            dreaming: false,
            disable_player_input: false,
            last_effect: None,
            reset_layer_on_stop: None,
            disable_player_input_time: 0,
            exit_bed_direction: None,
            no_snap_on_stop: false,
            check_walkable_on_next_frame: false,
            speed_mod: 0,
            move_delay_timer: 0,
            on_ladder: false
        };

        player.load_effect_textures(creator);

        player
    }

    fn load_effect_textures<T>(&mut self, creator: &'a TextureCreator<T>) {
        self.effect_textures.insert(Effect::Glasses, Texture::from_file(&PathBuf::from("res/textures/player/glasses.png"), creator).unwrap());
        self.effect_textures.insert(Effect::Speed, Texture::from_file(&PathBuf::from("res/textures/player/running_shoes.png"), creator).unwrap());
        self.effect_textures.insert(Effect::Fire, Texture::from_file(&PathBuf::from("res/textures/player/fire.png"), creator).unwrap());
    }

    pub fn stash_last_effect(&mut self) {
        self.last_effect = self.current_effect.clone();
    }

    pub fn enable_last_effect(&mut self) -> bool {
        match &self.last_effect {
            Some(e) => {
                self.apply_effect(e.clone());
                return true;
            },
            None => self.remove_effect(),
        }
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

    pub fn get_override_texture(&self) -> Option<&Texture> {
        if !self.animation_override_controller.active {
            return None;
        }
        match self.animation_override_controller.texture {
            PlayerTextureSheet::Effect => Some(&self.effects_texture),
            PlayerTextureSheet::Fire => Some(&self.extra_textures.fire),
            PlayerTextureSheet::Other => Some(&self.extra_textures.other),
            PlayerTextureSheet::Player => Some(&self.texture)
        }
    }

    pub fn do_sit(&mut self, world: &mut World) {
        self.disable_player_input = true;
        self.stash_last_effect();
        if self.remove_effect() {
            world.special_context.play_sounds.push(("effect_negate".to_string(), 1.0, 1.0));
        }
        self.disable_player_input_time = 0;
        self.animation_override_controller.do_sit();
        //self.move_player(Direction::Up, world, true, true, MovementIgnoreParams::IgnoreAll, sfx);
        self.force_move_player(Direction::Up, world);
        //self.draw_over = true;
        self.layer += 1;
    }

    pub fn do_lay_down(&mut self, world: &mut World) {
        self.disable_player_input = true;
        self.stash_last_effect();
        if self.remove_effect() {
            world.special_context.play_sounds.push(("effect_negate".to_string(), 1.0, 1.0));
        }
        self.disable_player_input_time = 0;
        self.animation_override_controller.do_lay_down();
        self.force_move_player_custom(self.facing, world, MOVE_TIMER_MAX + 8);
        self.exit_bed_direction = Some(self.facing.flipped());
        self.no_snap_on_stop = true;
    }

    /// Can move the player in sub-tile increments, you should enable Player::no_snap_on_stop
    pub fn force_move_player_custom(&mut self, direction: Direction, world: &mut World, distance: i32) {
        self.force_move_player(direction, world);
        self.move_timer = distance;
    }

    pub fn force_move_player(&mut self, direction: Direction, world: &mut World) {
        self.moving = true;
        self.move_timer = MOVE_TIMER_MAX;
        self.occupied_tile.0 = (self.occupied_tile.0 as i32 + direction.x()) as u32;
        self.occupied_tile.1 = (self.occupied_tile.1 as i32 + direction.y()) as u32;
        world.special_context.play_sounds.push((
            "step".to_string(),
            1.0, 0.25
        ));

        self.facing = direction;
        self.animation_info.frame_row = match direction {
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Right => 0,
            Direction::Up => 3
        };
    }

    pub fn move_player(&mut self, direction: Direction, world: &mut World, force: bool, just_pressed: bool, sfx: &mut SoundEffectBank) {
        if !self.moving || force {
            if self.on_stairs(world) {
                let diag = self.check_stair_diag(direction, world);
                if diag != 0 {
                    let pos = self.get_standing_tile();
                    let target = (pos.0 as i32 + direction.x(), pos.1 as i32 + diag);
                    if !(target.0 < 0 || target.1 < 0 || target.0 >= world.width as i32 || target.1 >= world.height as i32) && !world.get_collision_at_tile(target.0 as u32, target.1 as u32, self.layer) {
                        self.moving = true;
                        self.move_timer = MOVE_TIMER_MAX;
                        self.occupied_tile.0 = (self.occupied_tile.0 as i32 + direction.x()) as u32;
                        self.occupied_tile.1 = (self.occupied_tile.0 as i32 + diag) as u32;
                        sfx.play_ex("step", 1.0, 0.25);

                        if !force {
                            self.animation_info.frame = 1;
                        }
                        self.diag_move = diag;
    
                        self.facing = direction;
                        self.animation_info.frame_row = match direction {
                            Direction::Down => 1,
                            Direction::Left => 2,
                            Direction::Right => 0,
                            Direction::Up => 3
                        };
                        return;
                    }
                }
            }

            if self.can_move_in_direction(direction, world) && !self.frozen {
                self.moving = true;
                //dbg!("normal move");
                self.move_timer = MOVE_TIMER_MAX;
                self.occupied_tile.0 = (self.occupied_tile.0 as i32 + direction.x()) as u32;
                self.occupied_tile.1 = (self.occupied_tile.1 as i32 + direction.y()) as u32;
                let pos = self.get_standing_tile();

                let (sound, volume) = self.get_step_sound(world, ((pos.0 as i32 + direction.x()) as u32, (pos.1 as i32 + direction.y()) as u32));
                sfx.play_ex(&sound, 1.0, volume);

                if !force {
                    self.animation_info.frame = 1;
                }
            } else {
                let pos = self.get_standing_tile();
                let target_pos = (pos.0 as i32 + direction.x(), pos.1 as i32 + direction.y());

                if world.looping &&
                (target_pos.0 < 0 || target_pos.1 < 0 || target_pos.0 >= world.width as i32 || target_pos.1 >= world.height as i32) {
                    let mut moved = false;

                    if world.loop_horizontal() && target_pos.0 < 0 && !world.get_unbounded_collision_at_tile(world.width as i32 - 1, (self.y / 16) + 1, self.layer) { // left
                        self.x = world.width as i32 * 16;
                        self.occupied_tile.0 = world.width - 1;
                        self.occupied_tile.1 = (self.occupied_tile.1 as i32 + direction.y()) as u32;

                        // correction for looping images
                        // i have no idea how or why this works
                        for image_layer in world.image_layers.iter_mut() {
                            image_layer.x -= if image_layer.parallax_mode { 
                                (4 * image_layer.image.width as i32 - (world.width as i32 * 16)) / image_layer.parallax_x
                            } else {
                                (4 * image_layer.image.width as i32 - (world.width as i32 * 16)) * image_layer.parallax_x
                            }
                        }
                        moved = true;
                    } else if world.loop_horizontal() && target_pos.0 >= world.width as i32 && !world.get_unbounded_collision_at_tile(0, (self.y / 16) + 1, self.layer) { // right
                        self.x = -16;
                        self.occupied_tile.0 = 0;
                        self.occupied_tile.1 = (self.occupied_tile.1 as i32 + direction.y()) as u32;
                        for image_layer in world.image_layers.iter_mut() {
                            image_layer.x += if image_layer.parallax_mode { 
                                (4 * image_layer.image.width as i32 - (world.width as i32 * 16)) / image_layer.parallax_x
                            } else {
                                (4 * image_layer.image.width as i32 - (world.width as i32 * 16)) * image_layer.parallax_x
                            }
                        }
                        moved = true;
                    } else if world.loop_vertical() && target_pos.1 < 0 && !world.get_unbounded_collision_at_tile(self.x / 16, world.height as i32 - 1, self.layer) { // up
                        self.y = world.height as i32 * 16 - 16;
                        self.occupied_tile.0 = (self.occupied_tile.0 as i32 + direction.x()) as u32;
                        self.occupied_tile.1 = world.height - 1;
                        for image_layer in world.image_layers.iter_mut() {
                            image_layer.y -= if image_layer.parallax_mode {
                                (4 * image_layer.image.height as i32 - (world.height as i32 * 16)) / image_layer.parallax_y
                            } else {
                                (4 * image_layer.image.height as i32 - (world.height as i32 * 16)) * image_layer.parallax_y
                            }
                        }
                        moved = true;
                    } else if world.loop_vertical() && target_pos.1 >= world.height as i32 && !world.get_unbounded_collision_at_tile(self.x / 16, 0, self.layer) { // down
                        self.y = -32;
                        self.occupied_tile.0 = (self.occupied_tile.0 as i32 + direction.x()) as u32;
                        self.occupied_tile.1 = 0;
                        for image_layer in world.image_layers.iter_mut() {
                            image_layer.y += if image_layer.parallax_mode {
                                (4 * image_layer.image.height as i32 - (world.height as i32 * 16)) / image_layer.parallax_y
                            } else {
                                (4 * image_layer.image.height as i32 - (world.height as i32 * 16)) * image_layer.parallax_y
                            }
                        }
                        moved = true;
                    }

                    if moved {
                        self.moving = true;
                        self.move_timer = MOVE_TIMER_MAX;
                        self.draw_over = true;
                        let new_pos = self.get_standing_tile();
                        let (sound, volume) = self.get_step_sound(world, ((new_pos.0 as i32 + direction.x()) as u32, (new_pos.1 as i32 + direction.y()) as u32));
                        sfx.play_ex(&sound, 1.0, volume);
                    } else {
                        self.animation_info.frame = 1;
                        let player_pos = self.get_standing_tile();
                        if just_pressed || force {
                            world.player_bump(player_pos.0 as i32 + direction.x(), player_pos.1 as i32 + direction.y());
                        }
                    }
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

    pub fn movement_check(&mut self, input: &Input, world: &mut World, force: bool, sfx: &mut SoundEffectBank) -> bool {
        use Keycode::*;
        if self.disable_player_input {
            return false;
        }

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
                self.move_player(*last, world, force, input.get_just_pressed(last.to_key().unwrap_or(Keycode::PrintScreen)), sfx);
                return true;
            }
        } else if directions_pressed.len() == 1 {
            let direction = directions_pressed.first().unwrap();
            self.move_player(*direction, world, force, input.get_just_pressed(direction.to_key().unwrap_or(Keycode::PrintScreen)), sfx);
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

    pub fn check_stair_diag(&mut self, direction: Direction, world: &World) -> i32 {
        match direction {
            Direction::Down | Direction::Up => return 0,
            _ => ()
        }

        let (mut tile_x, tile_y) = self.get_standing_tile();
        tile_x = match direction {
            Direction::Left => tile_x - 1,
            Direction::Right => tile_x + 1,
            _ => unreachable!()
        };

        let up = world.get_special_in_layer(self.layer, tile_x, tile_y - 1);
        let down = world.get_special_in_layer(self.layer, tile_x, tile_y + 1);

        // prioritize up over down

        for special in up {
            if matches!(special, SpecialTile::Stairs) {
                return -1;
            }
        }

        for special in down {
            if matches!(special, SpecialTile::Stairs) {
                return 1;
            }
        }

        0
    }

    pub fn apply_effect(&mut self, effect: Effect) {
        effect.apply(self);
        self.current_effect = Some(effect);
        self.disable_player_input_time = 16;
        self.animation_info.effect_switch_animation = 8;
        self.animation_info.effect_switch_animation_timer = SWITCH_EFFECT_ANIMATION_SPEED;
        self.effect_just_changed = true;
    }

    pub fn remove_effect(&mut self) -> bool {
        if self.current_effect.is_some() {
            let effect = self.current_effect.take().unwrap();
            effect.remove(self);
            self.disable_player_input_time = 16;
            self.animation_info.effect_switch_animation = 8;
            self.animation_info.effect_switch_animation_timer = SWITCH_EFFECT_ANIMATION_SPEED;
            self.effect_just_changed = true;
            return true;
        }
        false
    }

    pub fn give_effect(&mut self, effect: Effect) {
        if !self.has_effect(&effect) {
            self.unlocked_effects.push(effect);
        }
    }

    pub fn has_effect(&self, effect: &Effect) -> bool {
        return self.unlocked_effects.contains(effect);
    }
    
    pub fn on_level_transition(&mut self) {
        if self.animation_override_controller.lay_down_animation {
            self.animation_override_controller.lay_down_animation = false;
            self.animation_override_controller.active = false;
            //self.facing = Direction::Down;
            self.look_in_direction(Direction::Down);
            self.disable_player_input = false;
        }
    }
    
    /// Speed, delay
    pub fn speed(&self) -> (u32, u32) {
        if self.speed_mod == 0 {
            return (self.speed, self.move_delay);
        } else {
            let mut speed = self.speed;
            if self.speed_mod < 0 {
                let mut delay = self.move_delay;
                for i in 0..self.speed_mod.abs() {
                    speed /= 2;
                    if speed == 0 {
                        delay = 1.max(delay * 2);
                    }
                }
                if delay > 0 {
                    speed = 1;
                }
                return (speed, delay);
            } else {
                let mut delay = self.move_delay;
                for i in 0..self.speed_mod {
                    speed *= 2;
                    if self.speed == 0 { 
                        delay /= 2;
                        if delay == 0 && speed == 0 {
                            speed = 1;
                        }
                    }
                }
                if delay > 0 {
                    speed = 1;
                }
                return (speed, delay);
            }
        }
    }

    pub fn look_in_direction(&mut self, direction: Direction) {
        self.facing = direction;
        self.animation_info.frame_row = match direction {
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Right => 0,
            Direction::Up => 3
        }
    }

    pub fn update(&mut self, input: &Input, world: &mut World, sfx: &mut SoundEffectBank) {
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

        if self.disable_player_input_time > 0 {
            self.disable_player_input = true;
            self.disable_player_input_time -= 1;
            if self.disable_player_input_time == 0 {
                self.disable_player_input = false;
            }
        }

        self.extra_textures.animate();
        self.animation_info.animate_effects();

        if self.animation_override_controller.sit_animation || self.animation_override_controller.lay_down_animation {
            if !self.moving && !self.animation_override_controller.active {
                self.animation_override_controller.active = true;
            }
        }

        if self.moving {
            if self.move_delay_timer > 0 {
                self.move_delay_timer -= 1;
                // TODO: returning here may be a problem if anything is done in this function after the move check
                return;
            }
            let (speed, delay) = self.speed();
            self.x += self.facing.x() * speed as i32;
            self.y += self.facing.y() * speed as i32;
            self.y += self.diag_move * speed as i32;
            self.move_delay_timer = delay as i32;
            self.move_timer -= speed as i32;
            self.animation_info.animate_walk();
            if self.check_walkable_on_next_frame {
                if !self.can_move_in_direction(self.facing, &world) {
                    self.move_timer = 0;
                    self.moving = false;
                    self.x = (self.x as f32 / 16.0).round() as i32 * 16;
                    self.y = (self.y as f32 / 16.0).round() as i32 * 16;
                    self.moving = false;
                    self.move_timer = 0;
                    self.draw_over = false;
                    self.diag_move = 0;
                }
                self.check_walkable_on_next_frame = false;
            }

            // if self.animation_info.do_step {
            //     sfx.play_ex(&self.get_step_sound(world), 1.0, 0.5);
            //     self.animation_info.do_step = false;
            // }

            if self.frozen {
                self.x = (self.x as f32 / 16.0).round() as i32 * 16;
                self.y = (self.y as f32 / 16.0).round() as i32 * 16;
                self.moving = false;
                self.move_timer = 0;
                self.draw_over = false;
                self.diag_move = 0;
            } else if self.move_timer <= 0 {
                if !self.no_snap_on_stop {
                    self.x = (self.x as f32 / 16.0).round() as i32 * 16;
                    self.y = (self.y as f32 / 16.0).round() as i32 * 16;
                }

                self.moving = false;
                self.move_timer = 0;
                self.draw_over = false;
                self.diag_move = 0;
                let tile = (self.x / 16, (self.y / 16) + 1);
                world.player_walk(tile.0, tile.1);
                self.speed_mod = 0;
                let mut touched_ladder = false;
                for special in world.get_special_in_layer(self.layer, tile.0 as u32, tile.1 as u32).into_iter() {
                    if let SpecialTile::SpeedMod(speed_mod) = special {
                        self.speed_mod = *speed_mod;
                    }

                    if let SpecialTile::Ladder = special {
                        touched_ladder = true;
                    }
                }

                if touched_ladder {
                    if !self.on_ladder {
                        self.on_ladder = true;
                        self.stash_last_effect();
                        if self.remove_effect() {
                            world.special_context.play_sounds.push(("effect_negate".to_string(), 1.0, 1.0));
                        }
                    }
                } else {
                    if self.on_ladder {
                        if self.enable_last_effect() {
                            sfx.play("effect");
                        }
                        self.on_ladder = false;
                    }
                }

                if let Some(reset_layer) = self.reset_layer_on_stop {
                    self.layer = reset_layer;
                }
                self.reset_layer_on_stop = None;
                if !self.movement_check(input, world, true, sfx) {
                    self.animation_info.stop();
                }
            }
        } else {
            self.movement_check(input, world, false, sfx);
            if input.get_just_pressed(Keycode::Z) {
                let pos = self.get_standing_tile();
                world.interactions.push(crate::world::Interaction::Use(pos.0 as i32 + self.facing.x(), pos.1 as i32 + self.facing.y()));
                if self.animation_override_controller.sit_animation {
                    self.disable_player_input = false;
                    self.animation_override_controller.sit_animation = false;
                    self.animation_override_controller.active = false;
                    self.force_move_player(Direction::Down, world);
                    if self.enable_last_effect() {
                        sfx.play("effect");
                    }
                    self.reset_layer_on_stop = Some(self.layer - 1);
                } else if self.animation_override_controller.lay_down_animation {
                    self.disable_player_input = false;
                    self.animation_override_controller.lay_down_animation = false;
                    self.animation_override_controller.active = false;
                    self.force_move_player(self.exit_bed_direction.unwrap_or(Direction::Left), world);
                    if self.enable_last_effect() {
                        sfx.play("effect");
                    }
                }
            }
        } 
    }

    pub fn get_standing_tile(&self) -> (u32, u32) {
        (
            (self.x / 16).max(0) as u32,
            ((self.y / 16) + 1).max(0) as u32
        )
    }

    pub fn on_stairs(&self, world: &World) -> bool {
        let tile = self.get_standing_tile();
        for special in world.get_special_in_layer(self.layer, tile.0, tile.1) {
            if matches!(special, SpecialTile::Stairs) {
                return true;
            }
        }

        return false;
    }

    pub fn get_step_sound(&self, world: &World, pos: (u32, u32)) -> (String, f32) {
        for special in world.get_special_in_layer(self.layer, pos.0, pos.1) {
            if let SpecialTile::Step(sound, volume) = special {
                return (sound.clone(), *volume);
            }
        }

        return (String::from("step"), 0.25);
    }

    fn pre_draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, pos: (i32, i32), _state: &RenderState) {
        if self.current_effect.is_some() {
            let fire = matches!(self.current_effect.as_ref().unwrap(), Effect::Fire);

            if fire {
                let src = self.extra_textures.get_frame_pos_back();
                canvas.copy(&self.extra_textures.fire.texture, Rect::new(src.0 as i32, src.1 as i32, 32, 48), Rect::new(pos.0 - 8, pos.1 - 8, 32, 48)).unwrap();
            }
        }
    }

    fn post_draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, pos: (i32, i32), _state: &RenderState) {
        if self.current_effect.is_some() {
            let fire = matches!(self.current_effect.as_ref().unwrap(), Effect::Fire);

            if fire {
                let src = self.extra_textures.get_frame_pos_front();
                canvas.copy(&self.extra_textures.fire.texture, Rect::new(src.0 as i32, src.1 as i32, 32, 48), Rect::new(pos.0 - 8, pos.1 - 8, 32, 48)).unwrap();
            }
        }
    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, state: &RenderState) {
        let source = if self.on_ladder {
            self.animation_info.get_ladder_frame_pos()
        } else {
            self.animation_info.get_frame_pos()
        };
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

        self.pre_draw(canvas, (x, y), state);
        if !self.animation_override_controller.active {
            if self.current_effect.is_some() {
                if let Some(texture) = self.effect_textures.get(self.current_effect.as_ref().unwrap()) {
                    canvas.copy(&texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
                } else {
                    canvas.copy(&self.texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
                }
                
            } else {
                canvas.copy(&self.texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
            }
        } else {
            let override_source = self.animation_override_controller.frame_pos;
            if let Some(texture) = self.get_override_texture() {
                canvas.copy(&texture.texture, Rect::new(override_source.0 as i32, override_source.1 as i32, 16, 32), Rect::new(x, y, 16, 32)).unwrap();
            }
        }
        self.post_draw(canvas, (x, y), state);

        if self.animation_info.effect_switch_animation > 0 {
            let frame = 8 - self.animation_info.effect_switch_animation;
            canvas.copy(
                &self.effects_texture.texture, 
                Rect::new(48 * frame as i32, 0, 48, 48), 
                Rect::new(x - 24 + 8, y - 24 + 16, 48, 48)
            ).unwrap();
        }
    }

    // pub fn draw_looping<T: RenderTarget>(&self, canvas: &mut Canvas<T>, _state: &RenderState) {
    //     let source = self.animation_info.get_frame_pos();
    //     self.pre_draw(canvas, (self.x, self.y), _state);
    //     if self.current_effect.is_some() {
    //         if let Some(texture) = self.effect_textures.get(self.current_effect.as_ref().unwrap()) {
    //             canvas.copy(&texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(self.x, self.y, 16, 32)).unwrap();
    //         } else {
    //             canvas.copy(&self.texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(self.x, self.y, 16, 32)).unwrap();
    //         }
    //     } else {
    //         canvas.copy(&self.texture.texture, Rect::new(source.0 as i32, source.1 as i32, 16, 32), Rect::new(self.x, self.y, 16, 32)).unwrap();
    //     }
    //     self.post_draw(canvas, (self.x, self.y), _state);

    //     if self.animation_info.effect_switch_animation > 0 {
    //         let frame = 8 - self.animation_info.effect_switch_animation;
    //         canvas.copy(&self.effects_texture.texture, 
    //             Rect::new(48 * frame as i32, 0, 48, 48),
    //             Rect::new(self.x - 24 + 8, self.y - 24 + 16, 48, 48) 
    //         ).unwrap();
    //     }
    // }
}