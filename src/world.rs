use std::{sync::Arc, path::PathBuf, collections::HashMap};

use rodio::{Sink, OutputStreamHandle};
use sdl2::{render::{Canvas, RenderTarget, Texture, TextureCreator, TextureAccess}, rect::{Rect, Point}, pixels::{Color, PixelFormatEnum}, EventSubsystem};

use crate::{tiles::{Tilemap, Tileset, Tile}, player::{Player, self}, game::{RenderState, QueuedLoad, Action, Transition, self, TransitionTextures}, audio::Song, entity::{Entity, Trigger}, texture, world};

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
    pub action_id: usize
}

// pub enum Flag {
//     Int(i32),
//     String(String)
// }

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
    pub queued_load: Option<QueuedLoad>,
    pub queued_entity_actions: Vec<QueuedEntityAction>,

    /// Up, Down, Left, Right
    pub side_actions: [(bool, Option<Box<dyn Action>>); 4],
    pub paused: bool,
    pub interaction: Option<Interaction>,
    pub transition: Option<Transition>,
    pub looping: bool,
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
}

impl<'a> World<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>) -> Self {
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
            queued_load: None,
            side_actions: [(false, None), (false, None), (false, None), (false, None)],
            paused: false,
            interaction: None,
            transition: None,
            looping: false,
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
            transition_context: TransitionContext::new(creator)
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
            queued_load: None,
            side_actions: [(false, None), (false, None), (false, None), (false, None)],
            paused: false,
            interaction: None,
            transition: None,
            looping: false,
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
            }
        }
    }

    pub fn player_bump(&mut self, x: i32, y: i32) {
        self.interaction = Some(Interaction::Bump(x, y));
    }

    pub fn player_use(&mut self, x: i32, y: i32) {
        self.interaction = Some(Interaction::Use(x, y));
    }

    pub fn player_walk(&mut self, x: i32, y: i32) {
        self.interaction = Some(Interaction::Walk(x, y));
    }

    pub fn onload(&mut self, sink: &Sink) {
        if let Some(song) = &mut self.song {
            song.play(sink);
        } else {
            sink.set_volume(0.0);
        }
        for entity in self.entities.as_mut().unwrap().iter_mut() {
            for action in &mut entity.actions {
                if matches!(action.trigger, Trigger::OnLoad) {
                    action.run_on_next_loop = true;
                }
            }
        }
    }

    pub fn reset(&mut self) {
        for entity in self.entities.as_mut().unwrap().iter_mut() {
            if let Some(animator) = &mut entity.animator {
                animator.reset();
            }
        }
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.as_mut().unwrap().push(entity);
    }

    pub fn update(&mut self, player: &mut Player, sink: &Sink) {
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
                    if let Some(song) = &mut self.song {
                        song.volume = song.default_volume;
                        song.speed = song.default_speed;
                        song.dirty = true;
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

        if !self.paused {
            for image_layer in self.image_layers.iter_mut() {
                image_layer.update();
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

            if let Some(inter) = &self.interaction {
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
            self.interaction = None;

            // TODO: delayed actions for screen transitions (if needed)
            for i in 0..4 {
                if self.side_actions[i].0 && self.side_actions[i].1.is_some() {
                    let action = self.side_actions[i].1.take();
                    action.as_ref().unwrap().act(player, self);
                    self.side_actions[i].1 = action;
                    self.side_actions[i].0 = false;
                }
            }

            for (i, j) in act_entities.iter() {
                self.special_context.action_id = *j;
                self.special_context.entity_id = *i;
                let entity = self.entities.as_mut().unwrap().remove(*i);
                entity.actions.get(*j).unwrap().action.act(player, self);
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

            if let Some(delayed_action) = action_opt {
                let action = self.queued_entity_actions.remove(delayed_action);
                let entity = self.entities.as_mut().unwrap().remove(action.entity_id);
                self.special_context.entity_id = action.entity_id;
                self.special_context.action_id = action.action_id;
                self.special_context.delayed_run = true;
                entity.actions.get(action.action_id).unwrap().action.act(player, self);
                self.special_context.delayed_run = false;
                self.entities.as_mut().unwrap().insert(action.entity_id, entity);
            }

            for i in 0..self.entities.as_ref().unwrap().len() {
                let mut entity = std::mem::replace(self.entities.as_mut().unwrap().get_mut(i).unwrap(), placeholder.take().unwrap());
                for action in entity.actions.iter_mut() {
                    if action.run_on_next_loop {
                        action.action.act(player, self);
                    }
                    action.run_on_next_loop = false;
                }
                placeholder = Some(std::mem::replace(self.entities.as_mut().unwrap().get_mut(i).unwrap(), entity));
            }
        }
    }

    pub fn draw<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, player: &Player, state: &RenderState) {
        for height in self.layer_min..=self.layer_max {

            for image_layer in self.image_layers.iter() {
                if image_layer.draw && image_layer.height == height {
                    image_layer.draw(canvas, state);
                }
            }

            for layer in self.layers.iter() {
                if layer.draw && layer.height == height {
                    //layer.map.draw(canvas, self.tilesets[layer.map.tileset_id], state);
                    self.draw_tile_layer(canvas, layer, state);
                } else if layer.height > height {
                    // because layers are sorted, breaking early is fine
                    break;
                }
            }

            for entity in self.entities.as_ref().unwrap().iter() {
                if entity.draw && entity.get_height(player.y) == height {
                    self.draw_entity(canvas, entity, state);
                }
            }

            if player.layer == height {
                player.draw(canvas, state);
            }
        }

        if let Some(tint) = self.tint {
            canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
            canvas.set_draw_color(tint);
            canvas.fill_rect(None).unwrap();
        }

        if self.transition.is_some() {
            let mut transition = self.transition.take().unwrap();
            transition.draw(canvas, self);
            self.transition = Some(transition);
        }
    }

    pub fn draw_looping<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, player: &Player, state: &RenderState) {
        assert!(self.render_texture.is_some(), "world needs to have a render texture to do looping draws");
        let mut render_texture = self.render_texture.take();

        // TODO: Image layer height support for looping draws

        for image_layer in self.image_layers.iter() {
            image_layer.draw(canvas, state);
        }

        canvas.with_texture_canvas(render_texture.as_mut().unwrap(), |tex_canvas| {
            tex_canvas.set_draw_color(Color::RGBA(255, 255, 255, 0));
            tex_canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
            tex_canvas.clear();
            // HAHAHAHHAHAAAAAAAAA
            let world_state = RenderState::new((self.width * 16, self.height * 16));
            for height in self.layer_min..=self.layer_max {

                for layer in self.layers.iter() {
                    if layer.draw && layer.height == height {
                        //layer.map.draw(canvas, self.tilesets[layer.map.tileset_id], state);
                        self.draw_tile_layer(tex_canvas, layer, &world_state);
                    } else if layer.height > height {
                        // because layers are sorted, breaking early is fine
                        break;
                    }
                }
                for entity in self.entities.as_ref().unwrap().iter() {
                    if entity.draw && entity.get_height(player.y) == height {
                        self.draw_entity(tex_canvas, entity, &world_state);
                    }
                }
                if player.layer == height {
                    player.draw_looping(tex_canvas, &world_state);
                }
            }
        }).unwrap();
        self.render_texture = render_texture;

        let mut dest = Rect::new(0, 0, state.screen_extents.0, state.screen_extents.1);
        let mut source = Rect::new(player.x + 8 - (state.screen_extents.0 as i32 / 2), player.y + 16 - (state.screen_extents.1 as i32 / 2), state.screen_extents.0, state.screen_extents.1);

        let mut left_loop = false;
        let mut right_loop = false;
        let mut up_loop = false;
        let mut down_loop = false;
        let width_px = self.width as i32 * 16;
        let height_px = self.height as i32 * 16;

        if state.offset.0 > 0 {
            dest.x = state.offset.0;
            source.x = 0;
            left_loop = true;
        }

        if state.offset.1 > 0 {
            dest.y = state.offset.1;
            source.y = 0;
            up_loop = true;
        }

        if state.offset.0 * -1 + state.screen_extents.0 as i32 > self.width as i32 * 16 {
            source.w = width_px + state.offset.0;
            dest.w = source.w;
            right_loop = true;
        }

        if -state.offset.1 + state.screen_extents.1 as i32 > self.height as i32 * 16 {
            source.h = height_px + state.offset.1;
            dest.h = source.h;
            down_loop = true;
        }

        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);

        if left_loop {
            let sub_rect = Rect::new(width_px - state.offset.0 as i32, source.y.max(0), state.offset.0 as u32, source.height());
            let dest_rect = Rect::new(0, dest.y, state.offset.0 as u32, dest.height());
            canvas.copy(self.render_texture.as_ref().unwrap(), Some(sub_rect), Some(dest_rect)).unwrap();
        }

        if right_loop {
            let sub_rect = Rect::new(0, source.y, (width_px + state.offset.0) as u32 + 16, source.height());
            let dest_rect = Rect::new(width_px + state.offset.0, dest.y, sub_rect.width(), dest.height());
            canvas.copy(self.render_texture.as_ref().unwrap(), Some(sub_rect), Some(dest_rect)).unwrap();
        }

        if up_loop {
            let sub_rect = Rect::new(source.x.max(0), height_px - state.offset.1, source.width(), state.offset.1 as u32);
            let dest_rect = Rect::new(dest.x, 0, dest.width(), state.offset.1 as u32);
            canvas.copy(self.render_texture.as_ref().unwrap(), Some(sub_rect), Some(dest_rect)).unwrap(); 
        }

        if down_loop {
            let sub_rect = Rect::new(source.x, 0, source.width(), (height_px + state.offset.1) as u32 + 16);
            let dest_rect = Rect::new(dest.x, height_px + state.offset.1, dest.width(), sub_rect.height());
            canvas.copy(self.render_texture.as_ref().unwrap(), Some(sub_rect), Some(dest_rect)).unwrap(); 
        }

        // Top-left corner
        if up_loop && left_loop {
            canvas.copy(
                self.render_texture.as_ref().unwrap(),
                Some(Rect::new(width_px - state.offset.0 as i32, height_px - state.offset.1 as i32, state.offset.0 as u32, state.offset.1 as u32)),
                Some(Rect::new(0, 0, state.offset.0 as u32, state.offset.1 as u32))
            ).unwrap();
        }

        // Top-right corner
        if up_loop && right_loop {
            canvas.copy(
                self.render_texture.as_ref().unwrap(),
                Some(Rect::new(0, height_px - state.offset.1 as i32, (width_px + state.offset.0) as u32 + 16, source.height())),
                Some(Rect::new(width_px + state.offset.0, 0, (width_px + state.offset.0) as u32 + 16, state.offset.1 as u32))
            ).unwrap();
        }

        // Bottom-left corner
        if down_loop && left_loop {
            canvas.copy(
                self.render_texture.as_ref().unwrap(),
                Some(Rect::new(width_px - state.offset.0 as i32, 0, state.offset.0 as u32, (height_px + state.offset.1) as u32 + 16)),
                Some(Rect::new(0, height_px + state.offset.1, state.offset.0 as u32, (height_px + state.offset.1) as u32 + 16))
            ).unwrap();
        }

        // Bottom-right corner
        if down_loop && right_loop {
            canvas.copy(
                self.render_texture.as_ref().unwrap(),
                Some(Rect::new(0, 0, (width_px + state.offset.0) as u32 + 16, (height_px + state.offset.1) as u32 + 16)),
                Some(Rect::new(width_px + state.offset.0, height_px + state.offset.1, (width_px + state.offset.0) as u32 + 16, (height_px + state.offset.1) as u32 + 16)),
            ).unwrap();
        }

        canvas.copy(self.render_texture.as_ref().unwrap(), Some(source), Some(dest)).unwrap();

        if player.draw_over || player.y < 0 {
            player.draw(canvas, state);
        }

        if let Some(tint) = self.tint {
            canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
            canvas.set_draw_color(tint);
            canvas.fill_rect(None).unwrap();
        }

        if self.transition.is_some() {
            let mut transition = self.transition.take().unwrap();
            transition.draw(canvas, self);
            self.transition = Some(transition);
        }
    }

    //pub fn draw<'a, T: RenderTarget>(&self, canvas: &mut Canvas<T>, tileset: Tileset<'a>, state: &RenderState) {
    pub fn draw_tile_layer<T: RenderTarget>(&self, canvas: &mut Canvas<T>, layer: &Layer, state: &RenderState) {
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

    pub fn draw_entity<T: RenderTarget>(&self, canvas: &mut Canvas<T>, entity: &Entity, state: &RenderState) {
        // canvas.copy(&self.texture.texture, Rect::new(tile_x as i32 * 16, tile_y as i32 * 16, 16, 16), Rect::new(pos.0, pos.1, 16, 16)).unwrap();
        if let Some(animator) = &entity.animator {
            self.tilesets[animator.tileset as usize].draw_tile_sized(canvas, animator.frame, (entity.x + state.offset.0, entity.y + state.offset.1));
        } else {
            self.tilesets[entity.tileset as usize].draw_tile_sized(canvas, entity.id, (entity.x + state.offset.0, entity.y + state.offset.1));
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
        let index = (y * self.width + x) as usize;
        let width = self.width;
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
    pub parallax_mode: bool
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
            parallax_y: 1
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
        //let repeat_x = game::ceil((-left) + state.screen_extents.0 as i32, w_i32) / w_i32;
        //let repeat_y = game::ceil((-top) + state.screen_extents.1 as i32, h_i32) / h_i32;
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

pub struct SpecialContext {
    /// if a delayed action is ready
    pub delayed_run: bool,

    /// index of the action being called
    pub action_id: usize,

    /// index of the entity that contains an action
    pub entity_id: usize
}

impl SpecialContext {
    pub fn new() -> Self {
        Self {
            delayed_run: false,
            action_id: 0,
            entity_id: 0
        }
    }
}

pub struct TransitionContext<'a> {
    pub screenshot: Option<sdl2::render::Texture<'a>>,
    pub take_screenshot: bool
}

impl<'a> TransitionContext<'a> {
    pub fn new<T>(creator: &'a TextureCreator<T>) -> Self {
        // world.render_texture = Some(creator.create_texture(Some(PixelFormatEnum::RGBA8888), TextureAccess::Target, world.width * 16, world.height * 16).expect("failed to create render texture for looping level"));
        // world.render_texture.as_mut().unwrap().set_blend_mode(sdl2::render::BlendMode::Blend);

        Self {
            screenshot: Some(creator.create_texture(Some(PixelFormatEnum::RGBA8888), TextureAccess::Target,400, 300).expect("failed to create render texture for transitions")),
            take_screenshot: false
        }
    }
}