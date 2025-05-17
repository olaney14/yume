use std::{cell::RefCell, cmp::Ordering, collections::HashMap, path::PathBuf, rc::Rc};

use json::JsonValue;
use rand::Rng;
use rodio::Sink;
use sdl2::{render::{Canvas, RenderTarget, Texture, TextureCreator, TextureAccess}, rect::{Rect, Point}, pixels::{Color, PixelFormatEnum}};
use serde_derive::{Deserialize, Serialize};

use crate::{actions::Action, audio::{Song, SoundEffectBank}, effect::Effect, entity::{Entity, Trigger, VariableValue}, game::{self, BoolProperty, EntityPropertyType, Input, IntProperty, QueuedLoad, RenderState}, player::Player, screen_event::ScreenEvent, texture, tiles::{SpecialTile, Tile, Tilemap, Tileset}, transitions::{Transition, TransitionTextures}};

const RAINDROPS_LIFETIME: u32 = 10;
const RAINDROPS_PER_CYCLE: usize = 3;
const RAINDROP_FRAMES: usize = 4;

const SNOW_LIFETIME: u32 = 40;
const SNOW_PER_CYCLE: usize = 1;
const SNOW_FRAMES: usize = 5;

pub const OFFSCREEN_DISTANCE: u32 = 18;

#[derive(Clone)]
pub enum Interaction {
    Use(i32, i32),
    Bump(i32, i32),
    Walk(i32, i32), 
}

impl Interaction {
    pub fn get_pos(&self) -> (i32, i32) {
        match self {
            &Self::Use(x, y) | &Self::Bump(x, y) | &Self::Walk(x, y) => return (x, y)
        }
    }
}

pub struct QueuedEntityAction {
    pub delay: i32,
    pub entity_id: usize,
    pub action_id: usize,
    pub multiple_action_id: Option<usize>
}

#[derive(Clone)]
pub struct RandomState {
    pub level_random: f32,
    pub session_random: f32
}

impl RandomState {
    pub fn new() -> Self {
        Self {
            level_random: rand::thread_rng().gen_range(0.0..1.0),
            session_random: rand::thread_rng().gen_range(0.0..1.0)
        }
    }

    pub fn level(mut self) -> Self {
        self.level_random = rand::thread_rng().gen_range(0.0..1.0);
        self
    }
}

pub struct World<'a> {
    pub layers: Vec<Layer>,
    pub image_layers: Vec<ImageLayer<'a>>,
    pub tilesets: Vec<Tileset<'a>>,
    /// The lowest layer depth found in this world
    pub layer_min: i32,
    /// The highest layer depth found in this world
    pub layer_max: i32,
    pub width: u32,
    pub height: u32,
    pub background_color: sdl2::pixels::Color,
    pub clamp_camera: bool,
    pub clamp_camera_axes: Option<Axis>,

    /// On the next available frame, the map in the QueuedLoad will be loaded and the map transition will begin <br>
    /// The player is placed at the target position
    pub queued_load: Option<QueuedLoad>,
    pub queued_entity_actions: Vec<QueuedEntityAction>,

    /// Up, Down, Left, Right
    pub side_actions: [(bool, Option<Box<dyn Action>>); 4],
    pub paused: bool,
    pub interactions: Vec<Interaction>,

    /// This is some if a transition is currently happening
    pub transition: Option<Transition>,
    pub looping: bool,
    pub looping_axes: Option<Axis>,
    pub render_texture: Option<Texture<'a>>,
    pub song: Option<Song>,
    pub tint: Option<Color>,
    pub entities: Option<Vec<Entity>>,
    pub default_pos: Option<(i32, i32)>,
    pub name: String,
    pub special_context: SpecialContext,
    pub flags: HashMap<String, i32>,
    pub global_flags: HashMap<String, i32>,
    pub transitions: TransitionTextures<'a>,
    pub transition_context: TransitionContext<'a>,
    pub timer: u64,
    pub draw_player: bool,
    pub raindrops: RaindropsInfo,
    pub snow: SnowInfo,
    pub source_file: PathBuf,
    pub particle_textures: ParticleTextures<'a>,

    pub screen_events: HashMap<String, ScreenEvent<'a>>,
    pub running_screen_event: Option<String>,
    pub pre_event_song: Option<Song>,

    pub entity_draw_order: Vec<Vec<usize>>,
    pub player_draw_slot: Option<usize>,
    pub random: RandomState
}

#[derive(Serialize, Deserialize)]
pub enum Axis {
    All,
    Horizontal,
    Vertical
}

impl Axis {
    pub fn parse(from: &str) -> Option<Self> {
        match from.to_lowercase().as_ref() {
            "all" => return Some(Self::All),
            "horizontal" | "horiz" | "x" => return Some(Self::Horizontal),
            "vertical" | "vert" | "y" => return Some(Self::Vertical),
            _ => {
                eprintln!("Warning: Invalid axis type `{}`", from);
                return None;
            }
        }
    }
}

impl<'a> World<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>, state: &RenderState) -> Self {
        Self {
            layers: Vec::new(),
            image_layers: Vec::new(),
            tilesets: Vec::new(),
            layer_max: 0, 
            layer_min: 0,
            width: 0,
            height: 0,
            background_color: sdl2::pixels::Color::RGBA(0, 0, 0, 255),
            clamp_camera: false,
            clamp_camera_axes: None,
            queued_load: None,
            side_actions: [(false, None), (false, None), (false, None), (false, None)],
            paused: false,
            interactions: Vec::new(),
            transition: None,
            looping: false,
            looping_axes: None,
            render_texture: None,
            song: None,
            tint: None,
            entities: Some(Vec::new()),
            default_pos: None,
            name: String::from("none"),
            queued_entity_actions: Vec::new(),
            special_context: SpecialContext::new(),
            flags: HashMap::new(),
            global_flags: HashMap::new(),
            transitions: TransitionTextures::new(creator).unwrap(),
            transition_context: TransitionContext::new(creator, state),
            timer: 0,
            draw_player: true,
            raindrops: RaindropsInfo::new(),
            snow: SnowInfo::new(),
            source_file: PathBuf::new(),
            particle_textures: ParticleTextures::new(),
            running_screen_event: None,
            screen_events: HashMap::new(),
            pre_event_song: None,
            entity_draw_order: Vec::new(),
            player_draw_slot: None,
            random: RandomState::new()
        }
    }

    /// this function creates a new world, without copying any settings
    /// but reusing loaded textures
    pub fn with_old<T>(old: &mut World<'a>, creator: &'a TextureCreator<T>) -> Self {
        let transitions = std::mem::replace(&mut old.transitions, TransitionTextures::empty(creator));

        Self {
            layers: Vec::new(),
            image_layers: Vec::new(),
            tilesets: Vec::new(),
            layer_max: 0, 
            layer_min: 0,
            width: 0,
            height: 0,
            background_color: sdl2::pixels::Color::RGBA(0, 0, 0, 255),
            clamp_camera: false,
            clamp_camera_axes: None,
            queued_load: None,
            side_actions: [(false, None), (false, None), (false, None), (false, None)],
            paused: false,
            interactions: Vec::new(),
            transition: None,
            looping: false,
            looping_axes: None,
            render_texture: None,
            song: None,
            tint: None,
            entities: Some(Vec::new()),
            default_pos: None,
            name: String::from("none"),
            queued_entity_actions: Vec::new(),
            special_context: SpecialContext::new(),
            flags: HashMap::new(),
            global_flags: HashMap::new(),
            transitions,
            transition_context: TransitionContext {
                screenshot: old.transition_context.screenshot.take(),
                take_screenshot: true
            },
            timer: 0,
            draw_player: true,
            raindrops: RaindropsInfo::new(),
            snow: SnowInfo::new(),
            source_file: PathBuf::new(),
            particle_textures: ParticleTextures::new(),
            running_screen_event: None,
            screen_events: HashMap::new(),
            pre_event_song: None,
            entity_draw_order: Vec::new(),
            player_draw_slot: None,
            random: old.random.clone().level()
        }
    }

    pub fn can_rain_on_tile(&self, x: u32, y: u32) -> bool {
        for layer in self.layers.iter() {
            if x < layer.map.width && y < layer.map.height {
                if let Some(special) = layer.map.get_special(x, y) {
                    match special {
                        SpecialTile::NoRain => {
                            return false;
                        },
                        _ => ()
                    }
                }
            }
        }

        true
    }

    pub fn get_special_in_layer(&self, height: i32, x: u32, y: u32) -> Vec<&SpecialTile> {
        let mut specials = Vec::new();
        
        for layer in &self.layers {
            if layer.height == height && x < layer.map.width && y < layer.map.height {
                if let Some(special) = layer.map.get_special(x, y) {
                    specials.push(special);
                }
            }
        }

        specials
    }

    pub fn player_bump(&mut self, x: i32, y: i32) {
        self.interactions.push(Interaction::Bump(x, y));
    }

    pub fn player_use(&mut self, x: i32, y: i32) {
        self.interactions.push(Interaction::Use(x, y));
    }

    pub fn player_walk(&mut self, x: i32, y: i32) {
        self.interactions.push(Interaction::Walk(x, y));
    }

    pub fn onload(&mut self, player: &Player, sink: &Sink, state: &RenderState) {
        if let Some(song) = &mut self.song {
            song.play(sink);
        } else {
            sink.set_volume(0.0);
        }
        for entity in self.entities.as_mut().unwrap().iter_mut() {
            for action in &mut entity.actions {
                if action.trigger.contains_trigger(&Trigger::OnLoad) {
                    action.run_on_next_loop = true;
                }
            }
        }

        self.find_entity_draw_order(player, state);
    }

    pub fn reset(&mut self) {
        for entity in self.entities.as_mut().unwrap().iter_mut() {
            if let Some(animator) = &mut entity.animator {
                animator.reset();
            }
        }
    }

    pub fn loop_horizontal(&self) -> bool {
        self.looping && matches!(self.looping_axes, Some(Axis::Horizontal | Axis::All) | None)
    }

    pub fn loop_vertical(&self) -> bool {
        self.looping && matches!(self.looping_axes, Some(Axis::All | Axis::Vertical) | None)
    }

    pub fn clamp_horizontal(&self) -> bool {
        self.clamp_camera && matches!(self.clamp_camera_axes, Some(Axis::All | Axis::Horizontal) | None)
    }

    pub fn clamp_vertical(&self) -> bool {
        self.clamp_camera && matches!(self.clamp_camera_axes, Some(Axis::All | Axis::Vertical) | None)
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.as_mut().unwrap().push(entity);
    }

    pub fn update(&mut self, player: &mut Player, sfx: &mut SoundEffectBank, sink: &Sink, input: &Input, state: &mut RenderState) {
        self.timer += 1;
        if let Some(transition) = &mut self.transition {
            if transition.holding {
                transition.hold_timer -= 1;
                if transition.hold_timer == transition.hold / 2 {
                    transition.progress = 100;
                }
                if transition.hold_timer <= 0 {
                    transition.holding = false;
                }
            } else {
                if transition.delay > 0 && transition.delay_timer == 0 {
                    transition.delay_timer = transition.delay
                }

                if transition.delay_timer > 0 {
                    transition.delay_timer -= 1;
                } 
                if transition.delay_timer <= 0 {
                    transition.progress += transition.direction * transition.speed;
                    self.paused = true;
                    if transition.fade_music {
                        if let Some(song) = &mut self.song {
                            song.volume = song.default_volume - (((transition.progress as f32) / 100.0) * song.default_volume);
                            song.dirty = true;
                        }
                    }
                    if transition.progress >= 100 {
                        transition.progress = 100;
                        transition.direction = -1;
                        if transition.hold > 0 {
                            transition.holding = true;
                            transition.progress = 99;
                        }
                    } else if transition.progress <= -1 {
                        self.paused = false;
                        self.transition = None;
                        self.draw_player = true;
                        if let Some(song) = &mut self.song {
                            song.volume = song.default_volume;
                            song.speed = song.default_speed;
                            song.dirty = true;
                        }
                    }
                }
            }
        }

        if let Some(song) = &mut self.song {
            if song.dirty {
                song.update(sink);
                song.dirty = false;
            }
        }

        while !self.special_context.play_sounds.is_empty() {
            if let Some((song, speed, volume)) = self.special_context.play_sounds.pop() {
                sfx.play_ex(song.as_str(), speed, volume);
            }
        }

        if let Some(effect) = &self.special_context.effect_get {
            sfx.play_ex("effect_get", 1.0, 0.5);
            player.frozen = true;
            player.give_effect(effect.clone());
            self.paused = true;
            self.special_context.effect_get = None;
        }

        if !self.paused {
            for image_layer in self.image_layers.iter_mut() {
                image_layer.update();
            }

            for entity in self.entities.as_mut().unwrap().iter_mut() {
                for action in &mut entity.actions {
                    if player.effect_just_changed && action.trigger.contains_trigger(&Trigger::EffectSwitch) {
                        action.run_on_next_loop = true;
                    }
                    if let Some(time) = action.trigger.get_tick() {
                        if self.timer % time as u64 == 0 {
                            action.run_on_next_loop = true;
                        }
                    }
                }
            }

            let mut act_entities = Vec::new();

            let mut entity_list = self.entities.take().unwrap();
            let mut placeholder = Some(Entity::new());
            for i in 0..entity_list.len() {
                let mut entity = std::mem::replace(entity_list.get_mut(i).unwrap(), placeholder.take().unwrap());
                entity.update(self, &player, &entity_list);
                placeholder = Some(std::mem::replace(entity_list.get_mut(i).unwrap(), entity));
            }
            self.entities = Some(entity_list);

            for inter in self.interactions.iter() {
                match inter {
                    Interaction::Bump(x, y) | Interaction::Use(x, y) => {
                        if *y >= self.height as i32 {
                            self.side_actions[1].0 = true;
                        }
                        if *y < 0 {
                            self.side_actions[0].0 = true;
                        }
                        if *x >= self.width as i32{
                            self.side_actions[3].0 = true;
                        }
                        if *x < 0 {
                            self.side_actions[2].0 = true;
                        }
                    },
                    _ => (),
                }

                let point = inter.get_pos();
                for (i, entity) in self.entities.as_mut().unwrap().iter_mut().enumerate() {
                    if Rect::new(entity.collider.x + entity.x, entity.collider.y + entity.y, entity.collider.width(), entity.collider.height()).contains_point(Point::new(point.0 * 16 + 8, point.1 * 16 + 8)) {
                        entity.interaction = Some(
                            (inter.clone(), player.facing.flipped())
                        );
                        for (j, action) in entity.actions.iter().enumerate() {
                            if action.trigger.fulfilled_interaction(inter, Some(player.facing.flipped())) {
                                act_entities.push((i, j));
                            }
                        }
                    }
                }
            }
            self.interactions.clear();

            // TODO: delayed actions for screen transitions (if needed)
            for i in 0..4 {
                if self.side_actions[i].0 && self.side_actions[i].1.is_some() {
                    let action = self.side_actions[i].1.take();
                    action.as_ref().unwrap().act(player, self);
                    self.side_actions[i].1 = action;
                    self.side_actions[i].0 = false;
                }
            }

            self.special_context.entity_context.entity_call = true;
            for (i, j) in act_entities.iter() {
                let mut entity = self.entities.as_mut().unwrap().remove(*i);
                self.special_context.action_id = *j;
                self.special_context.entity_id = *i;
                self.special_context.entity_context.id = *i as i32;
                self.special_context.entity_context.x = entity.x;
                self.special_context.entity_context.y = entity.y;
                self.special_context.entity_context.entity_variables = Some(entity.variables.clone());
                entity.actions.get(*j).unwrap().action.act(player, self);
                self.apply_set_entity_properties(&mut entity, player);
                self.entities.as_mut().unwrap().insert(*i, entity);
            }

            let mut action_opt = None;

            for i in 0..self.queued_entity_actions.len() {
                self.queued_entity_actions[i].delay -= 1;
                if self.queued_entity_actions[i].delay <= 0 {
                    //let queued_action = self.queued_entity_actions.remove(i);
                    action_opt = Some(i);
                }
            }

            //self.special_context.multiple_action_index = None;
            if let Some(delayed_action) = action_opt {
                let action = self.queued_entity_actions.remove(delayed_action);
                let mut entity = self.entities.as_mut().unwrap().remove(action.entity_id);
                self.special_context.entity_id = action.entity_id;
                self.special_context.action_id = action.action_id;
                self.special_context.multiple_action_index = action.multiple_action_id;
                self.special_context.delayed_run = true;
                self.special_context.entity_context.id = action.entity_id as i32;
                self.special_context.entity_context.x = entity.x;
                self.special_context.entity_context.y = entity.y;
                self.special_context.entity_context.entity_variables = Some(entity.variables.clone());
                entity.actions.get(action.action_id).unwrap().action.act(player, self);
                self.special_context.delayed_run = false;
                self.apply_set_entity_properties(&mut entity, player);
                self.entities.as_mut().unwrap().insert(action.entity_id, entity);
            }

            for i in 0..self.entities.as_ref().unwrap().len() {
                let mut entity = std::mem::replace(self.entities.as_mut().unwrap().get_mut(i).unwrap(), placeholder.take().unwrap());
                for (j, action) in entity.actions.iter_mut().enumerate() {
                    if action.run_on_next_loop {
                        self.special_context.entity_context.id = i as i32;
                        self.special_context.entity_context.x = entity.x;
                        self.special_context.entity_context.y = entity.y;
                        self.special_context.entity_id = i;
                        self.special_context.action_id = j;
                        self.special_context.entity_context.entity_variables = Some(entity.variables.clone());
                        action.action.act(player, self);
                    }
                    action.run_on_next_loop = false;
                }
                // TODO this might be a problem if some set actions depend on others
                self.apply_set_entity_properties(&mut entity, player);
                placeholder = Some(std::mem::replace(self.entities.as_mut().unwrap().get_mut(i).unwrap(), entity));
            }
            for deferred_action in std::mem::take(&mut self.special_context.deferred_entity_actions).into_iter() {
                if let Some(entity) = self.entities.as_mut().unwrap().get_mut(deferred_action.0) {
                    (deferred_action.1)(entity);
                } else {
                    eprintln!("Warning: tried to use a deferred action on a `None`");
                }
            }
            self.special_context.entity_context.entity_call = false;

            if let Some(id) = self.special_context.entity_removal_queue.pop() {
                self.entities.as_mut().unwrap().remove(id);
            }

            if let Some(event) = &self.running_screen_event {
                if let Some(event) = self.screen_events.get_mut(event) {
                    if !event.running {
                        player.frozen = event.freeze_player;
                        event.running = true;
                        event.visible = true;
                    }
                    
                    if !event.tick(sfx, input, state) {
                        event.reset();
                        self.running_screen_event = None;
                        player.frozen = false;
                        if self.pre_event_song.is_some() {
                            self.song = self.pre_event_song.take();
                            self.song.as_mut().unwrap().dirty = true;
                            self.song.as_mut().unwrap().speed = self.song.as_ref().unwrap().default_speed;
                            self.song.as_mut().unwrap().volume = self.song.as_ref().unwrap().default_volume;
                            self.song.as_mut().unwrap().reload(sink);
                        } else if event.has_changed_song {
                            self.song = None;
                            sink.clear();
                        }
                    }

                    if let Some(song) = event.set_song.take() {
                        self.pre_event_song = self.song.take();
                        self.song = Some(Song::new(PathBuf::from("res/audio/music/").join(format!("{}.ogg", song.0))));
                        self.song.as_mut().unwrap().volume = song.1 * self.pre_event_song.as_ref().map(|s| s.volume).unwrap_or(1.0);
                        self.song.as_mut().unwrap().speed = song.2;
                        self.song.as_mut().unwrap().default_volume = song.1;
                        self.song.as_mut().unwrap().default_speed = song.2;
                        self.song.as_mut().unwrap().dirty = true;
                        self.song.as_mut().unwrap().reload = true;
                        event.has_changed_song = true;
                    }

                    if let Some(volume) = event.set_volume.take() {
                        self.song.as_mut().unwrap().volume = self.song.as_ref().unwrap().default_volume * volume;
                        self.song.as_mut().unwrap().dirty = true;
                    }
                }
            }

            if self.special_context.new_session {
                self.random.session_random = rand::thread_rng().gen_range(0.0..1.0);
                self.special_context.new_session = false;
            }

            self.find_entity_draw_order(player, state);
        }
    }

    fn find_entity_draw_order(&mut self, player: &Player, state: &RenderState) {
        let mut entity_ids_by_layer = Vec::new();

        for layer in self.layer_min..=self.layer_max {
            let mut layer_ids = Vec::new();
            for (i, entity) in self.entities.as_ref().unwrap().iter().enumerate() {
                if entity.get_height() == layer {
                    layer_ids.push(i);
                }
            }

            entity_ids_by_layer.push(layer_ids);
        }

        self.entity_draw_order = entity_ids_by_layer.into_iter().map(|mut ids| { 
            ids.sort_by(|a, b| {
                let a_pos = self.entities.as_ref().unwrap().get(*a).unwrap().get_standing_tile();
                let b_pos = self.entities.as_ref().unwrap().get(*b).unwrap().get_standing_tile();

                if self.entities.as_ref().unwrap().get(*a).unwrap().walk_over {
                    return Ordering::Less;
                }

                if a_pos.1 < b_pos.1 {
                    return Ordering::Less
                } else if a_pos.1 > b_pos.1 {
                    return Ordering::Greater
                } else if a_pos.0 > b_pos.0 {
                    return Ordering::Less
                } else if a_pos.0 < b_pos.0 {
                    return Ordering::Greater
                }

                Ordering::Equal
            }); 
            ids
        }).collect();

        let mut draw_player = self.entity_draw_order.get((player.layer - self.layer_min) as usize).unwrap().len();
        // if draw_player > 0 {
        //     draw_player -= 1;
        // }

        for (i, entity_id) in self.entity_draw_order.get((player.layer - self.layer_min) as usize).unwrap().iter().enumerate() {
            let entity = self.entities.as_ref().unwrap().get(*entity_id).unwrap();
            let entity_pos = (entity.collision_x(), entity.collision_y());
            let player_pos = (player.x, player.y + 16);
            //let entity_pos = entity.get_standing_tile();
            //let player_pos = player.get_standing_tile();

            if entity.walk_over {
                continue;
            }

            if player_pos.1 < entity_pos.1 {
                draw_player = i;
                break;
            } else if entity_pos.1 == player_pos.1 && player_pos.0 <= entity_pos.0 {
                draw_player = i;
                break;
            }
        }

        self.player_draw_slot = Some(draw_player);
    }

    pub fn apply_set_entity_properties(&mut self, entity: &mut Entity, player: &Player) {
        let mut properties = vec![];
        // TODO: i think this flips the order and might be a problem later on
        while !self.special_context.entity_context.set_properties.is_empty() {
            properties.push(self.special_context.entity_context.set_properties.remove(0));
        }

        for (prop, val) in properties {
            match prop {
                EntityPropertyType::ID => { eprintln!("no") },
                EntityPropertyType::Draw => { entity.draw = BoolProperty::parse(&val).unwrap().get(Some(player), Some(self)).unwrap() },
                EntityPropertyType::X => { entity.x = IntProperty::parse(&val).unwrap().get(Some(player), Some(self)).unwrap() },
                EntityPropertyType::Y => { entity.y = IntProperty::parse(&val).unwrap().get(Some(player), Some(self)).unwrap() },
            }
        }
    }

    pub fn defer_entity_action(&mut self, action: Box<dyn Fn(&mut Entity)>) {
        if self.special_context.entity_context.entity_call {
            self.special_context.deferred_entity_actions.push((self.special_context.entity_context.id as usize, action));
        }
    }

    pub fn draw<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, player: &Player, state: &RenderState) {
        let mut player_drawn = false;

        for height in self.layer_min..=self.layer_max {

            for image_layer in self.image_layers.iter() {
                if image_layer.draw && image_layer.height == height {
                    image_layer.draw(canvas, state);
                }
            }

            for layer in self.layers.iter() {
                if layer.draw && layer.height == height {
                    //layer.map.draw(canvas, self.tilesets[layer.map.tileset_id], state);
                    self.draw_tile_layer(canvas, layer, false, state);
                } else if layer.height > height {
                    // because layers are sorted, breaking early is fine
                    break;
                }
            }

            // for (_, entity) in self.entities.as_ref().unwrap().iter().enumerate() {
            //     if entity.draw && entity.get_height(player.y) == height {
            //         self.draw_entity(canvas, entity, false, state);
            //     }

            //     if let Some(emitter) = &entity.particle_emitter {
            //         if emitter.height == height {
            //             emitter.draw(canvas, self, state);
            //         }
            //     }
            // }

            let entity_ids = self.entity_draw_order.get((height - self.layer_min) as usize);
            if let Some(entity_ids) = entity_ids {
                for (i, id) in entity_ids.iter().enumerate() {
                    if height == player.layer && i == self.player_draw_slot.unwrap() {
                        player_drawn = true;
                        player.draw(canvas, state);
                    }

                    let entity = self.entities.as_ref().unwrap().get(*id).unwrap();
    
                    if entity.draw {
                        self.draw_entity(canvas, entity, false, state);
                    }
    
                    if let Some(emitter) = &entity.particle_emitter {
                        emitter.draw(canvas, self, state);
                    }
                }
            }

            if player.layer == height && self.draw_player && !player_drawn {
                player.draw(canvas, state);
            }
        }

        if let Some(tint) = self.tint {
            canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
            canvas.set_draw_color(tint);
            canvas.fill_rect(None).unwrap();
        }

        self.post_draw(canvas, state);

        // if self.transition.is_some() {
        //     let mut transition = self.transition.take().unwrap();
        //     transition.draw(canvas, self);
        //     self.transition = Some(transition);
        // }
    }

    pub fn post_draw<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, state: &RenderState) {
        let mut rng = rand::thread_rng();

        if self.raindrops.enabled {
            for _ in 0..RAINDROPS_PER_CYCLE {
                let x = rng.gen_range(0..state.screen_extents.0) as i32 - state.offset.0;
                let y = rng.gen_range(0..state.screen_extents.1) as i32 - state.offset.1;

                //let special = self.get_special_in_layer(height, x, y)
                let tile = ((x / 16).rem_euclid(self.width as i32) as u32, (y / 16).rem_euclid(self.height as i32) as u32);
                if self.can_rain_on_tile(tile.0, tile.1) {
                    self.raindrops.raindrops.push(Raindrop {
                        lifetime: RAINDROPS_LIFETIME,
                        x, y
                    });
                }
            }

            for raindrop in self.raindrops.raindrops.iter_mut() {
                raindrop.lifetime -= 1;
                if raindrop.lifetime == 0 {
                    continue;
                }

                let frame = (((RAINDROPS_LIFETIME - raindrop.lifetime) as f32 / RAINDROPS_LIFETIME as f32) * RAINDROP_FRAMES as f32) as i32;
                canvas.copy(
                    &self.transitions.raindrop.texture,
                    Some(Rect::new(frame * 4, 0, 4, 4)),
                    Some(Rect::new(raindrop.x + state.offset.0, raindrop.y + state.offset.1, 4, 4))
                ).unwrap();
            }

            self.raindrops.raindrops.retain(|r| r.lifetime > 0);
        }

        if self.snow.enabled {
            for _ in 0..SNOW_PER_CYCLE {
                let x = rng.gen_range(0..state.screen_extents.0) as i32 - state.offset.0;
                let y = rng.gen_range(-80..state.screen_extents.1 as i32) - state.offset.1;

                self.snow.snow.push(Snow {
                    lifetime: SNOW_LIFETIME,
                    x, y
                });
            }

            for snow in self.snow.snow.iter_mut() {
                snow.lifetime -= 1;
                if snow.lifetime == 0 {
                    continue;
                }

                snow.y += 2;

                let osc = ((SNOW_LIFETIME - snow.lifetime) as f32 / (SNOW_LIFETIME as f32 / 10.0)).sin() * 2.0;
                snow.x += osc as i32;

                let frame = (((2.0 * SNOW_FRAMES as f32 / SNOW_LIFETIME as f32) * (snow.lifetime as f32 - SNOW_LIFETIME as f32 / 2.0).abs()) as i32).min(SNOW_FRAMES as i32 - 1);
                canvas.copy(&self.transitions.snow.texture, 
                    Some(Rect::new(frame * 3, 0, 3, 3)), 
                    Some(Rect::new(snow.x + state.offset.0, snow.y + state.offset.1, 3, 3))
                ).unwrap();
            }

            self.snow.snow.retain(|r| r.lifetime > 0);
        }

        if let Some(screen_event) = &self.running_screen_event {
            if let Some(event) = self.screen_events.get(screen_event) {
                event.draw(canvas, state);
            }
        }
    }

    pub fn draw_looping<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, player: &Player, state: &RenderState) {
        let mut player_drawn = false;

        for height in self.layer_min..=self.layer_max {
            for image_layer in self.image_layers.iter() {
                if image_layer.draw && image_layer.height == height {
                    image_layer.draw(canvas, state);
                }
            }

            for layer in self.layers.iter() {
                if layer.draw && layer.height == height {
                    //layer.map.draw(canvas, self.tilesets[layer.map.tileset_id], state);
                    self.draw_tile_layer(canvas, layer, true, state);
                } else if layer.height > height {
                    // because layers are sorted, breaking early is fine
                    break;
                }
            }

            // for entity in self.entities.as_ref().unwrap().iter() {
            //     if entity.draw && entity.get_height(player.y) == height {
            //         self.draw_entity(canvas, entity, true, state);
            //     }
                
            //     if let Some(emitter) = &entity.particle_emitter {
            //         if emitter.height == height {
            //             emitter.draw(canvas, self, state);
            //         }
            //     }
            // }

            let entity_ids = self.entity_draw_order.get((height - self.layer_min) as usize);
            if let Some(entity_ids) = entity_ids {
                for (i, id) in entity_ids.iter().enumerate() {
                    if height == player.layer && i == self.player_draw_slot.unwrap() {
                        player_drawn = true;
                        player.draw(canvas, state);
                    }

                    let entity = self.entities.as_ref().unwrap().get(*id).unwrap();
    
                    if entity.draw {
                        self.draw_entity(canvas, entity, true, state);
                    }
    
                    if let Some(emitter) = &entity.particle_emitter {
                        emitter.draw(canvas, self, state);
                    }
                }
            }

            if player.layer == height && self.draw_player && !player_drawn {
                player.draw(canvas, state);
            }
        }

        if let Some(tint) = self.tint {
            canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
            canvas.set_draw_color(tint);
            canvas.fill_rect(None).unwrap();
        }

        self.post_draw(canvas, state);
    }

    pub fn draw_transitions<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, player: &Player, state: &RenderState) {
        if self.transition.is_some() {
            let mut transition = self.transition.take().unwrap();
            transition.draw(canvas, self, player, state);
            self.transition = Some(transition);
        }
    }

    pub fn draw_whole_tile_layer<T: RenderTarget>(&self, canvas: &mut Canvas<T>, layer: &Layer, state: &RenderState) {
        let width = self.width;

        for y in 0..self.height {
            for x in 0..self.width {
                let tile = layer.map.tiles[(y * width + x) as usize];
                if tile.tileset >= 0 && tile.id >= 0 {
                    self.tilesets[tile.tileset as usize].draw_tile(canvas, tile.id as u32, (x as i32 * 16 + state.offset.0, y as i32 * 16 + state.offset.1));
                }
            }
        }
    }

    pub fn draw_tile_layer<T: RenderTarget>(&self, canvas: &mut Canvas<T>, layer: &Layer, looping: bool, state: &RenderState) {
        let orig_x = -state.offset.0 / 16;
        let orig_y = -state.offset.1 / 16;
        if looping {
            match self.looping_axes {
                Some(Axis::All) | None => {
                    self.draw_tile_layer_section_looping(canvas, layer, 
                        (orig_x - 1, orig_y - 1), 
                        (orig_x + state.screen_extents.0 as i32 / 16 + 1, orig_y + state.screen_extents.1 as i32 / 16 + 2), 
                    state);
                },
                Some(Axis::Horizontal) => {
                    self.draw_tile_layer_section_looping_horiz(canvas, layer, 
                        (orig_x - 1, orig_y - 1), 
                        (orig_x + state.screen_extents.0 as i32 / 16 + 1, orig_y + state.screen_extents.1 as i32 / 16 + 2), 
                    state);
                },
                Some(Axis::Vertical) => {
                    self.draw_tile_layer_section_looping_vert(canvas, layer, 
                        (orig_x - 1, orig_y - 1), 
                        (orig_x + state.screen_extents.0 as i32 / 16 + 1, orig_y + state.screen_extents.1 as i32 / 16 + 2), 
                    state);
                }
            }

        } else {
            self.draw_tile_layer_section(canvas, layer, 
                (orig_x, orig_y), 
                (orig_x + state.screen_extents.0 as i32 / 16 + 1, orig_y + state.screen_extents.1 as i32 / 16 + 2), 
            state);
        }
    }

    pub fn draw_tile_layer_section<T: RenderTarget>(&self, canvas: &mut Canvas<T>, layer: &Layer, 
        start: (i32, i32), end: (i32, i32), state: &RenderState) {
        let start_y = start.1.max(0);
        let start_x = start.0.max(0);
        let end_x = end.0.min(self.width as i32);
        let end_y = end.1.min(self.height as i32);
        for y in start_y..end_y {
            for x in start_x..end_x {
                let tile = layer.map.tiles[(y * self.width as i32 + x) as usize];
                if tile.tileset >= 0 && tile.id >= 0 {
                    self.tilesets[tile.tileset as usize].draw_tile(canvas, tile.id as u32, 
                        (x as i32 * 16 + state.offset.0, y as i32 * 16 + state.offset.1)
                    );
                }
            }
        }
    }

    pub fn draw_tile_layer_section_looping<T: RenderTarget>(&self, canvas: &mut Canvas<T>, layer: &Layer, 
        start: (i32, i32), end: (i32, i32), state: &RenderState) {
        for y in start.1..end.1 {
            for x in start.0..end.0 {
                let draw_coord = ( x.rem_euclid(self.width as i32), y.rem_euclid(self.height as i32) );
                let tile = layer.map.tiles[(draw_coord.1 * self.width as i32 + draw_coord.0) as usize];
                if tile.tileset >= 0 && tile.id >= 0 {
                    self.tilesets[tile.tileset as usize].draw_tile(canvas, tile.id as u32, 
                        (x as i32 * 16 + state.offset.0, y as i32 * 16 + state.offset.1)
                    );
                }
            }
        }
    }

    /// Looping will only happen on the horizontal axis
    pub fn draw_tile_layer_section_looping_horiz<T: RenderTarget>(&self, canvas: &mut Canvas<T>, layer: &Layer,
        start: (i32, i32), end: (i32, i32), state: &RenderState) {
        let start_y = start.1.max(0);
        let end_y = end.1.min(self.height as i32);

        for y in start_y..end_y {
            for x in start.0..end.0 {
                let draw_coord = (x.rem_euclid(self.width as i32), y);
                let tile = layer.map.tiles[(draw_coord.1 * self.width as i32 + draw_coord.0) as usize];
                if tile.tileset >= 0 && tile.id >= 0 {
                    self.tilesets[tile.tileset as usize].draw_tile(canvas, tile.id as u32, 
                        (x as i32 * 16 + state.offset.0, y as i32 * 16 + state.offset.1)
                    );
                }
            }
        }
    }

    /// Looping will only happen on the vertical axis
    pub fn draw_tile_layer_section_looping_vert<T: RenderTarget>(&self, canvas: &mut Canvas<T>, layer: &Layer,
        start: (i32, i32), end: (i32, i32), state: &RenderState) {
        let start_x = start.0.max(0);
        let end_x = end.0.min(self.width as i32);

        for y in start.1..end.1 {
            for x in start_x..end_x {
                let draw_coord = (x, y.rem_euclid(self.height as i32));
                let tile = layer.map.tiles[(draw_coord.1 * self.width as i32 + draw_coord.0) as usize];
                if tile.tileset >= 0 && tile.id >= 0 {
                    self.tilesets[tile.tileset as usize].draw_tile(canvas, tile.id as u32, 
                        (x as i32 * 16 + state.offset.0, y as i32 * 16 + state.offset.1)
                    );
                }
            }
        }
    }

    pub fn draw_entity<T: RenderTarget>(&self, canvas: &mut Canvas<T>, entity: &Entity, looping: bool, state: &RenderState) {
        if looping {
            let mut draw_positions;
            match self.looping_axes {
                Some(Axis::All) | None => {
                    let draw_pos = (entity.x + state.offset.0, entity.y + state.offset.1);
                    let draw_pos_rem = ((entity.x + state.offset.0).rem_euclid(self.width as i32 * 16), (entity.y + state.offset.1).rem_euclid(self.height as i32 * 16));
                    let draw_pos_far_rem = (
                        (entity.x + entity.collider.w + state.offset.0).rem_euclid(self.width as i32 * 16) - entity.collider.w,
                        (entity.y + entity.collider.h + state.offset.1).rem_euclid(self.height as i32 * 16) - entity.collider.h
                    );
                    draw_positions = vec![draw_pos, draw_pos_rem, draw_pos_far_rem];
                },
                Some(Axis::Vertical) => {
                    let draw_pos = (entity.x + state.offset.0, entity.y + state.offset.1);
                    let draw_pos_rem = (entity.x + state.offset.0, (entity.y + state.offset.1).rem_euclid(self.height as i32 * 16));
                    let draw_pos_far_rem = (
                        entity.x + state.offset.0,
                        (entity.y + entity.collider.h + state.offset.1).rem_euclid(self.height as i32 * 16) - entity.collider.h
                    );
                    draw_positions = vec![draw_pos, draw_pos_rem, draw_pos_far_rem];
                },
                Some(Axis::Horizontal) => {
                    let draw_pos = (entity.x + state.offset.0, entity.y + state.offset.1);
                    let draw_pos_rem = ((entity.x + state.offset.0).rem_euclid(self.width as i32 * 16), entity.y + state.offset.1);
                    let draw_pos_far_rem = (
                        (entity.x + entity.collider.w + state.offset.0).rem_euclid(self.width as i32 * 16) - entity.collider.w,
                        entity.y + state.offset.1
                    );
                    draw_positions = vec![draw_pos, draw_pos_rem, draw_pos_far_rem];
                }
            }

            draw_positions.sort();
            draw_positions.dedup();
            for position in draw_positions.into_iter() {
                if let Some(animator) = &entity.animator {
                    self.tilesets[animator.tileset as usize].draw_tile_sized(canvas, animator.frame, position);
                } else {
                    self.tilesets[entity.tileset as usize].draw_tile_sized(canvas, entity.id, position);
                }

                // if let Some(particles) = &entity.particle_emitter {
                //     particles.draw(canvas, &self, state);
                // }
            }
        } else {
            if let Some(animator) = &entity.animator {
                self.tilesets[animator.tileset as usize].draw_tile_sized(canvas, animator.frame, (entity.x + state.offset.0, entity.y + state.offset.1));
            } else {
                self.tilesets[entity.tileset as usize].draw_tile_sized(canvas, entity.id, (entity.x + state.offset.0, entity.y + state.offset.1));
            }

            // if let Some(particles) = &entity.particle_emitter {
            //     particles.draw(canvas, &self, state);
            // }
        }
    }

    pub fn add_layer(&mut self, layer: Layer) {
        if self.width < layer.map.width {
            self.width = layer.map.width;
        }
        if self.height < layer.map.height {
            self.height = layer.map.height;
        }
        if layer.height > self.layer_max { self.layer_max = layer.height; }
        if layer.height < self.layer_min { self.layer_min = layer.height; }
        self.layers.push(layer);
        self.layers.sort_by(|a, b| a.height.partial_cmp(&b.height).unwrap());
    }

    pub fn get_mut_layer_by_name(&mut self, name: &str) -> Option<&mut Layer> {
        return self.layers.iter_mut().find(|layer| layer.name == name)
    }

    pub fn try_set_tile(&mut self, layer: &str, tileset: &str, tile: i32, x: u32, y: u32) -> Result<(), ()> {
        let try_tileset = self.get_tileset_by_name(tileset);
        //let index = (y * self.width + x) as usize;
        //let width = self.width;
        if let Some(tileset) = try_tileset {
            if let Some(layer) = self.get_mut_layer_by_name(layer) {
                layer.map.set_tile(x, y, Tile::new(tile, tileset)).unwrap();
            }
        }
        
        Ok(())
    }

    pub fn get_tileset_by_name(&self, name: &str) -> Option<i32> {
        for (i, tileset) in self.tilesets.iter().enumerate() {
            if let Some(tileset_name) = &tileset.name {
                if tileset_name == name {
                    return Some(i.try_into().unwrap());
                }
            }
        }

        None
    }

    fn get_tilemap_collision_at_tile(&self, x: u32, y: u32, height: i32) -> bool {
        for layer in self.layers.iter().filter(|l| l.height == height) {
            if layer.map.get_collision(x, y) {
                return true;
            }
        }
        return false;
    }

    fn get_entity_collision_at_tile(&self, x: u32, y: u32, height: i32) -> bool {
        for entity in self.entities.as_ref().unwrap().iter().filter(|e| e.height == height) {
            if entity.get_collision(Rect::new(x as i32 * 16, y as i32 * 16, 16, 16)) {
                return true;
            }
        }
        return false;
    }

    /// Get a collision on a certain layer with the player
    pub fn get_collision_at_tile(&self, x: u32, y: u32, height: i32) -> bool {
        if self.get_tilemap_collision_at_tile(x, y, height) { return true; }
        if self.get_entity_collision_at_tile(x, y, height) { return true; }

        return false;
    }

    pub fn get_unbounded_collision_at_tile(&self, x: i32, y: i32, height: i32) -> bool {
        if x >= 0 && y >= 0 {
            if self.get_tilemap_collision_at_tile(x as u32, y as u32, height) { return true; }
            if self.get_entity_collision_at_tile(x as u32, y as u32, height) { return true; }
        }

        return false;
    }

    // pub fn collide_entity_at_tile_with_list(&self, x: u32, y: u32, player_opt: Option<&Player>, height: i32, entity_list: &Vec<Entity>) -> bool {
    //     if self.get_tilemap_collision_at_tile(x, y, height) { return true; }
    //     for entity in entity_list.iter().filter(|e| e.height == height) {
    //         if entity.get_collision(Rect::new(x as i32 * 16, y as i32 * 16, 16, 16)) {
    //             return true;
    //         }
    //     }
    //     if let Some(player) = player_opt {
    //         if Rect::new(x as i32 * 16, y as i32 * 16, 16, 16).has_intersection(Rect::new(player.x, player.y + 16, 16, 16)) { return true; }
    //     }

    //     return false;
    // }

    pub fn get_unbounded_collision_at_tile_with_list(&self, x: i32, y: i32, player_opt: Option<&Player>, height: i32, entity_list: &Vec<Entity>) -> bool {
        if x >= 0 && y >= 0 {
            if self.get_tilemap_collision_at_tile(x as u32, y as u32, height) { return true; }
            for entity in entity_list.iter().filter(|e| e.height == height) {
                // TODO: THIS MIGHT BE A HUGE PROBLEM!!!!!!!!!!!!!!!!!!
                if entity.get_collision(Rect::new(x as i32 * 16, y as i32 * 16, 16, 16)) {
                    return true;
                }
            }
            if let Some(player) = player_opt {
                if Rect::new(x as i32 * 16, y as i32 * 16, 16, 16).has_intersection(Rect::new(player.x, player.y + 16, 16, 16)) { return true; }
            }
        }

        return false;
    }

    pub fn collide_entity_at_tile(&self, x: u32, y: u32, player: &Player, height: i32) -> bool {
        if self.get_tilemap_collision_at_tile(x, y, height) { return true; }
        if self.get_entity_collision_at_tile(x, y, height) { return true; }
        if Rect::new(x as i32 * 16, y as i32 * 16, 16, 16).has_intersection(Rect::new(player.x, player.y + 16, 16, 16)) { return true; }
        return false;
    }

    pub fn collide_rect(&self, rect: Rect, height: i32) -> bool {
        for layer in self.layers.iter().filter(|l| l.height == height) {
            if layer.map.get_collision_with_rect(rect) {
                return true;
            }
        }

        for entity in self.entities.as_ref().unwrap().iter().filter(|e| e.height == height) {
            if entity.get_collision(rect) {
                return true;
            }
        }

        return false;
    }

    pub fn collide_entity_at_tile_with_list(&self, x: u32, y: u32, player_opt: Option<&Player>, height: i32, entity_list: &Vec<Entity>) -> bool {
        if self.get_tilemap_collision_at_tile(x, y, height) { return true; }
        for entity in entity_list.iter().filter(|e| e.height == height) {
            if entity.get_collision(Rect::new(x as i32 * 16, y as i32 * 16, 16, 16)) {
                return true;
            }
        }
        if let Some(player) = player_opt {
            if Rect::new(x as i32 * 16, y as i32 * 16, 16, 16).has_intersection(Rect::new(player.x, player.y + 16, 16, 16)) { return true; }
        }

        return false;
    }

    pub fn collide_entity(&self, rect: Rect, player: &Player, height: i32, entity_list: &Vec<Entity>) -> bool {
        for layer in self.layers.iter().filter(|l| l.height == height) {
            if layer.map.get_collision_with_rect(rect) {
                return true;
            }
        }

        for entity in entity_list.iter().filter(|e| e.height == height) {
            if entity.get_collision(rect) {
                return true;
            }
        }

        if rect.has_intersection(Rect::new(player.x, player.y + 16, 16, 16)) {
            return true;
        }

        return false;
    }
}

const PARTICLE_IMAGES_PATH: &str = "res/textures/particle/";

pub struct ParticleTextures<'a> {
    pub textures: HashMap<String, texture::Texture<'a>>
}

impl<'a> ParticleTextures<'a> {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new()
        }
    }

    pub fn get_texture(&self, id: &String) -> Option<&texture::Texture> {
        self.textures.get(id)
    }

    pub fn add_texture<T>(&mut self, name: &String, creator: &'a TextureCreator<T>) {
        self.textures.insert(
            name.clone(), 
            texture::Texture::from_file(&PathBuf::from(PARTICLE_IMAGES_PATH).join(name), creator).expect(&format!("failed to load particle texture {}", name))
        );
    }
}

pub struct ImageLayer<'a> {
    pub image: texture::Texture<'a>,
    pub x: i32,
    pub y: i32,
    pub looping_x: bool,
    pub looping_y: bool,
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub height: i32,
    pub draw: bool,
    pub delay_x: u32,
    pub delay_y: u32,
    pub timer_x: i32,
    pub timer_y: i32,
    pub parallax_x: i32,
    pub parallax_y: i32,
    /// True - divide, False - multiply
    pub parallax_mode: bool,
    pub name: String
}

impl<'a> ImageLayer<'a> {
    pub fn new(image: texture::Texture<'a>) -> Self {
        Self {
            image,
            looping_x: false,
            looping_y: false,
            scroll_x: 0,
            scroll_y: 0,
            x: 0,
            y: 0,
            height: 0,
            draw: true,
            delay_x: 0,
            delay_y: 0,
            timer_x: 0,
            timer_y: 0,
            parallax_mode: true,
            parallax_x: 1,
            parallax_y: 1,
            name: "Image Layer".to_string()
        }
    }

    pub fn load_from_file<T>(file: &PathBuf, creator: &'a TextureCreator<T>) -> Self {
        Self::new(texture::Texture::from_file(file, creator).expect("failed to load image layer"))
    }

    pub fn draw<T: RenderTarget>(&self, canvas: &mut Canvas<T>, state: &RenderState) {
        let modified_offset = (
            if self.parallax_mode { state.offset.0 / self.parallax_x } else { state.offset.0 * self.parallax_x },
            if self.parallax_mode { state.offset.1 / self.parallax_y } else { state.offset.1 * self.parallax_y }
        );

        let w_i32 = self.image.width as i32;
        let h_i32 = self.image.height as i32;
        let left = game::offset_floor(-modified_offset.0, w_i32, self.x);
        let top = game::offset_floor(-modified_offset.1, h_i32, self.y);
        let repeat_x = (state.screen_extents.0 as i32 / w_i32) + 2;
        let repeat_y = (state.screen_extents.1 as i32 / h_i32) + 2;

        for y in -1..repeat_y {
            for x in -1..repeat_x {
                canvas.copy( 
                    &self.image.texture, 
                    Rect::new(0, 0, self.image.width, self.image.height), 
                    Rect::new(left + modified_offset.0 + (x * w_i32), top + modified_offset.1 + (y * h_i32), self.image.width, self.image.height)
                ).unwrap();
            }
        }  
    }

    pub fn update(&mut self) {
        if self.delay_x > 0 {
            self.timer_x -= 1;
            if self.timer_x <= 0 {
                self.timer_x = self.delay_x as i32;
                self.x += self.scroll_x;
            }
        } else {
            self.x += self.scroll_x;
        }

        if self.delay_y > 0 {
            self.timer_y -= 1;
            if self.timer_y <= 0 {
                self.timer_y = self.delay_y as i32;
                self.y += self.scroll_y;
            }
        } else {
            self.y += self.scroll_y;
        }

        if self.x >= self.image.width as i32 {
            self.x %= self.image.width as i32;
        }
        if self.x < 0 {
            self.x %= self.image.width as i32;
            self.x += self.image.width as i32;
        }
        if self.y >= self.image.height as i32   {
            self.y %= self.image.height as i32;
        }
        if self.y < 0 {
            self.y %= self.image.height as i32;
            self.y += self.image.height as i32;
        }
    }
}

pub struct Layer {
    pub height: i32,
    pub map: Tilemap,
    pub draw: bool,
    pub collide: bool,
    pub name: String,
}

impl Layer {
    pub fn new(map: Tilemap) -> Self {
        Self {
            map,
            height: 0,
            draw: true,
            collide: true,
            name: String::new()
        }
    }
}

/// misc logic 
pub struct SpecialContext {
    /// if a delayed action is ready
    pub delayed_run: bool,

    /// index of the action being called
    pub action_id: usize,

    /// index of the entity that contains an action
    pub entity_id: usize,

    /// all sounds in this vector will be played on the next update
    /// sound, speed, volume
    pub play_sounds: Vec<(String, f32, f32)>,

    /// Gives the player an effect on the next available frame
    pub effect_get: Option<Effect>,

    /// set by the ui, used by main to make a new game
    pub new_game: bool,

    /// when true, opens the save menu next frame
    pub save_game: bool,

    pub pending_save: usize,
    pub write_save_to_pending: bool,

    pub pending_load: Option<usize>,

    pub entity_context: EntityContext,

    pub deferred_entity_actions: Vec<(usize, Box<dyn Fn(&mut Entity)>)>,

    pub entity_removal_queue: Vec<usize>,

    pub multiple_action_index: Option<usize>,

    /// if the map visited on the next map is the same map, actually reload it from file instead of just keeping it
    pub reload_on_warp: bool,
    pub new_session: bool,

    pub open_music_menu: bool
}

struct Raindrop {
    lifetime: u32,
    x: i32,
    y: i32
}

pub struct RaindropsInfo {
    raindrops: Vec<Raindrop>,
    pub enabled: bool
}

impl RaindropsInfo {
    pub fn new() -> Self {
        Self {
            raindrops: Vec::new(),
            enabled: false
        }
    }
}

struct Snow {
    lifetime: u32,
    x: i32,
    y: i32
}

pub struct SnowInfo {
    snow: Vec<Snow>,
    pub enabled: bool
}

impl SnowInfo {
    pub fn new() -> Self {
        Self {
            snow: Vec::new(),
            enabled: false
        }
    }
}

impl SpecialContext {
    pub fn new() -> Self {
        Self {
            delayed_run: false,
            action_id: 0,
            entity_id: 0,
            play_sounds: Vec::new(),
            effect_get: None,
            new_game: false,
            save_game: false,
            pending_save: 0,
            write_save_to_pending: false,
            pending_load: None,
            entity_context: EntityContext::new(),
            deferred_entity_actions: Vec::new(),
            entity_removal_queue: Vec::new(),
            multiple_action_index: None,
            reload_on_warp: false,
            new_session: false,
            open_music_menu: false
        }
    }
}

pub struct EntityContext {
    pub entity_call: bool,
    pub id: i32,
    pub x: i32,
    pub y: i32,
    pub entity_variables: Option<Rc<RefCell<HashMap<String, VariableValue>>>>,
    pub set_properties: Vec<(EntityPropertyType, JsonValue)>
}

impl EntityContext {
    pub fn new() -> Self {
        Self {
            entity_call: false,
            id: 0,
            x: 0,
            y: 0,
            entity_variables: None,
            set_properties: vec![]
        }
    }
}

pub struct TransitionContext<'a> {
    pub screenshot: Option<sdl2::render::Texture<'a>>,
    pub take_screenshot: bool
}

impl<'a> TransitionContext<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>, state: &RenderState) -> Self {
        // world.render_texture = Some(creator.create_texture(Some(PixelFormatEnum::RGBA8888), TextureAccess::Target, world.width * 16, world.height * 16).expect("failed to create render texture for looping level"));
        // world.render_texture.as_mut().unwrap().set_blend_mode(sdl2::render::BlendMode::Blend);

        Self {
            screenshot: Some(creator.create_texture(Some(PixelFormatEnum::RGBA8888), TextureAccess::Target, state.screen_extents.0, state.screen_extents.1).expect("failed to create render texture for transitions")),
            take_screenshot: false
        }
    }
}