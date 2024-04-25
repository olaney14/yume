use std::{collections::HashMap, rc::Rc, cell::RefCell};

use sdl2::rect::Rect;

use crate::{game::{Direction, IntProperty, FloatProperty, BoolProperty, StringProperty}, world::{Interaction, World}, ai::{Ai, Animator, AnimationFrameData}, player::{self, Player}, actions::Action};

pub struct TriggeredAction {
    pub trigger: Trigger,
    pub action: Box<dyn Action>,
    pub run_on_next_loop: bool
}

#[derive(PartialEq)]
pub enum Trigger {
    Use,
    Walk,
    Bump,
    AnyInteraction,
    OnLoad,
    Tick(u32),
    EffectSwitch,
    Sided(Direction, Box<Trigger>),
    Or(Vec<Trigger>)
}

impl Trigger {
    pub fn fulfilled_interaction(&self, interaction: &Interaction, side: Option<Direction>) -> bool {
        match self {
            Self::AnyInteraction => return true,
            Self::Bump => return matches!(interaction, Interaction::Bump(..)),
            Self::Walk => return matches!(interaction, Interaction::Walk(..)),
            Self::Use => return matches!(interaction, Interaction::Use(..)),
            Self::Sided(dir, trigger) => {
                return side.is_some() && side.unwrap() == *dir && trigger.fulfilled_interaction(interaction, side);
            },
            Self::Or(triggers) => {
                return triggers.iter().map(|t| t.fulfilled_interaction(interaction, side)).any(|b| b);
            }
            _ => false
        }
    }

    pub fn contains_trigger(&self, trigger: &Trigger) -> bool {
        if self == trigger { return true; }
        match self {
            Self::AnyInteraction => matches!(trigger, Self::Walk | Self::Use | Self::Bump),
            Self::Or(triggers) => {
                for inner_trigger in triggers.iter() {
                    if inner_trigger.contains_trigger(trigger) {
                        return true;
                    }
                }

                return false;
            },
            Self::Sided(_, inner_trigger) => {
                return inner_trigger.contains_trigger(trigger);
            },
            _ => false
        }
    }

    pub fn get_tick(&self) -> Option<u32> {
        if let Trigger::Tick(time) = self {
            return Some(*time);
        }

        if let Trigger::Or(triggers) = self {
            for inner_trigger in triggers.iter() {
                if let Some(time) = inner_trigger.get_tick() {
                    return Some(time);
                }
            }
        }

        return None;
    }
}

fn parse_trigger_type(source: &str) -> Option<Trigger> {
    match source {
        "use" => Some(Trigger::Use),
        "walk" => Some(Trigger::Walk),
        "bump" => Some(Trigger::Bump),
        "interact" => Some(Trigger::AnyInteraction),
        "onload" => Some(Trigger::OnLoad),
        "switch" => Some(Trigger::EffectSwitch),
        _ => None,
    }
}

pub fn parse_trigger(source: &mut json::JsonValue) -> Option<Trigger> {
    let mut base = None;

    if source.is_string() {
        return parse_trigger_type(source.as_str().unwrap());
    }

    if source["type"].is_string() {
        base = parse_trigger_type(source["type"].as_str().unwrap());

        if base.is_none() {
            if source["type"].as_str().unwrap() == "tick" {
                let freq = source["freq"].as_u32().unwrap_or(1).max(1);
                return Some(Trigger::Tick(freq));
            }
        }
    } else if source["type"].is_array() {
        let mut triggers = Vec::new();
        let mut trigger = source["type"].pop();

        while !trigger.is_null() {
            triggers.push(parse_trigger(&mut trigger));
            trigger = source["type"].pop();
        }

        base = Some(Trigger::Or(triggers.into_iter().filter_map(|x| x).collect()));
    }

    if source["side"].is_string() && base.is_some() {
        let dir = source["side"].as_str().unwrap().parse::<Direction>();
        if let Ok(direction) = dir {
            return Some(Trigger::Sided(direction, Box::new(base.unwrap())));
        }
    }
    
    return base;
}

pub struct EntityMovementInfo {
    pub moving: bool,
    pub move_timer: i32,
    pub speed: u32,
    pub direction: Direction,
}

#[derive(Clone)]
pub enum VariableValue {
    Int(IntProperty),
    LitInt(i32),
    Float(FloatProperty),
    LitFloat(f32),
    Bool(BoolProperty),
    LitBool(bool),
    String(StringProperty),
    LitString(String)
}

impl VariableValue {
    pub fn is_int(&self) -> bool {
        matches!(self, Self::Int(..) | Self::LitInt(..))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(..) | Self::LitFloat(..))
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(..) | Self::LitBool(..))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(..) | Self::LitString(..))
    }

    pub fn as_i32(&self, world: Option<&World>, player: Option<&Player>) -> Option<i32> {
        match self {
            Self::Int(prop) => {
                return prop.get(player, world);
            },
            Self::LitInt(i) => return Some(*i),
            _ => return None
        }
    }

    pub fn as_f32(&self, world: Option<&World>, player: Option<&Player>) -> Option<f32> {
        match self {
            Self::Float(prop) => {
                return prop.get(player, world);
            },
            Self::LitFloat(f) => return Some(*f),
            _ => return None
        }
    }

    pub fn as_bool(&self, world: Option<&World>, player: Option<&Player>) -> Option<bool> {
        match self {
            Self::Bool(prop) => {
                return prop.get(player, world);
            },
            Self::LitBool(b) => return Some(*b),
            _ => return None
        }
    }

    pub fn as_string(&self, world: Option<&World>, player: Option<&Player>) -> Option<String> {
        match self {
            Self::String(prop) => {
                return prop.get(player, world);
            },
            Self::LitString(s) => return Some(s.clone()),
            _ => return None
        }
    }
}

pub struct Entity {
    pub id: u32,
    pub tileset: u32,
    pub height: i32,
    pub walk_behind: bool,
    pub actions: Vec<TriggeredAction>,
    pub solid: bool,
    pub draw: bool,
    pub collider: Rect,
    pub x: i32,
    pub y: i32,
    pub ai: Option<Box::<dyn Ai>>,
    pub animator: Option<Animator>,
    pub movement: Option<EntityMovementInfo>,
    pub interaction: Option<(Interaction, Direction)>,
    pub variables: Rc<RefCell<HashMap<String, VariableValue>>>,
    //pub script: Option<>
}

// TODO looping movement for entities
// TODO continuous movement for entities
impl Entity {
    pub fn new() -> Self {
        Self {
            id: 0,
            tileset: 0,
            height: 0,
            walk_behind: false,
            actions: Vec::new(),
            solid: false,
            draw: false,
            collider: Rect::new(0, 0, 0, 0),
            ai: None,
            animator: None,
            movement: None,
            x: 0,
            y: 0,
            interaction: None,
            variables: Rc::new(RefCell::new(HashMap::new()))
        }
    }

    pub fn get_collision(&self, other: Rect) -> bool {
        Rect::new(self.x + self.collider.x, self.y + self.collider.y, self.collider.width(), self.collider.height()).has_intersection(other) && self.solid
    }

    pub fn get_height(&self, player_y: i32) -> i32 {
        if player_y < self.y && self.walk_behind {
            return self.height + 1;
        }

        return self.height;
    }

    pub fn set_variable(&mut self, name: String, value: VariableValue) {
        self.variables.borrow_mut().insert(name, value);
    }

    // pub fn get_variable(&mut self, name: &str) -> Option<&VariableValue> {
    //     self.variables.borrow().get(name)
    // }

    pub fn walk(&mut self, direction: Direction, world: &World, player: &Player, entity_list: &Vec<Entity>) -> bool {
        if let Some(movement) = &self.movement {
            if movement.moving {
                return false;
            }
        }

        if let Some(animator) = &mut self.animator {
            if let AnimationFrameData::Directional(data) = &mut animator.frame_data {
                if data.direction != direction {
                    data.direction = direction;
                    animator.frame = match direction {
                        Direction::Down => data.down,
                        Direction::Left => data.left,
                        Direction::Right => data.right,
                        Direction::Up => data.up
                    } * data.frames_per_direction + 1
                }
            }
        }

        if self.can_move_in_direction(direction, world, player, entity_list) {
            if self.movement.is_none() {
                self.movement = Some(EntityMovementInfo {
                    move_timer: player::MOVE_TIMER_MAX,
                    moving: false,
                    speed: 1,
                    direction
                });
            }
            let mut movement = self.movement.take().unwrap();
            movement.moving = true;
            movement.move_timer = player::MOVE_TIMER_MAX;
            movement.direction = direction;
            self.movement = Some(movement);
            return true;
        } else {
            // taken straight from Player::move_player()
            let pos = self.get_standing_tile();
            let target_pos = (pos.0 as i32 + direction.x(), pos.1 as i32 + direction.y());
            
            if world.looping &&
            (target_pos.0 < 0 || target_pos.1 < 0 || target_pos.0 >= world.width as i32 || target_pos.1 >= world.height as i32) {
                let mut moved = false;

                if world.loop_horizontal() && target_pos.0 < 0 && !world.get_unbounded_collision_at_tile_with_list(world.width as i32 - 1, (self.y / 16) + 1, Some(player), self.height, entity_list) { // left
                    self.x = world.width as i32 * 16 - self.collider.x;
                    moved = true;
                } else if world.loop_horizontal() && target_pos.0 >= world.width as i32 && !world.get_unbounded_collision_at_tile_with_list(0, (self.y / 16) + 1, Some(player), self.height, entity_list) { // right
                    self.x = -16 - self.collider.x;
                    moved = true;
                } else if world.loop_vertical() && target_pos.1 < 0 && !world.get_unbounded_collision_at_tile_with_list(self.x / 16, world.height as i32 - 1, Some(player), self.height, entity_list) { // up
                    self.y = world.height as i32 * 16 - self.collider.y;
                    moved = true;
                } else if world.loop_vertical() && target_pos.1 >= world.height as i32 && !world.get_unbounded_collision_at_tile_with_list(self.x / 16, 0, Some(player), self.height, entity_list) { // down 
                    self.y = -16 - self.collider.y;
                    moved = true;
                }

                if moved {
                    let mut movement = self.movement.take().unwrap();
                    movement.moving = true;
                    movement.move_timer = player::MOVE_TIMER_MAX;
                    movement.direction = direction;
                    self.movement = Some(movement);
                    return true;
                }
            }
        }

        return false;
    }

    pub fn init_movement(&mut self) {
        if self.movement.is_none() {
            self.movement = Some(EntityMovementInfo {
                move_timer: player::MOVE_TIMER_MAX,
                moving: false,
                speed: 1,
                direction: Direction::Down
            });
        }
    }

    pub fn update(&mut self, world: &mut World, player: &Player, entity_list: &Vec<Entity>) {
        if self.ai.is_some() {
            let mut ai = self.ai.take().unwrap();
            ai.act(self, world, player, entity_list);
            self.ai = Some(ai);
        }

        let on_move = if let Some(animator) = &self.animator { animator.on_move } else { false };
        let manual = if let Some(animator) = &self.animator { animator.manual } else { false };

        if !(on_move || manual) {
            if let Some(animator) = &mut self.animator {
                animator.step();
            }
        }

        if let Some(movement) = &mut self.movement {
            if movement.moving {
                self.x += movement.direction.x() * movement.speed as i32;
                self.y += movement.direction.y() * movement.speed as i32;
                movement.move_timer -= movement.speed as i32;


                if movement.move_timer <= 0 {
                    self.x = (self.x as f32 / 16.0).round() as i32 * 16;
                    self.y = (self.y as f32 / 16.0).round() as i32 * 16;
                    movement.move_timer = player::MOVE_TIMER_MAX;
                    movement.moving = false;
                }
            }

            if movement.moving && on_move {
                if let Some(animator) = &mut self.animator {
                    animator.step();
                }
            }
        }
    }

    pub fn collision_y(&self) -> i32 {
        self.y + self.collider.y
    }

    pub fn collision_x(&self) -> i32 {
        self.x + self.collider.x
    }

    pub fn would_bump_player(&self, direction: Direction, player: &Player) -> bool {
        let mut target_rect = self.collider;
        target_rect.x += self.x + direction.x() * 16;
        target_rect.y += self.y + direction.y() * 16;
        if target_rect.has_intersection(Rect::new(player.x, player.y + 16, 16, 16)) {
            return true;
        }

        return false;
    }

    // taken from Player
    pub fn can_move_in_direction(&self, direction: Direction, world: &World, player: &Player, entity_list: &Vec<Entity>) -> bool {
        let pos = self.get_standing_tile();
        let target_tile = (
            (pos.0 as i32 + direction.x()).max(0) as u32,
            (pos.1 as i32 + direction.y()).max(0) as u32,
        );
        let mut target_rect = self.collider;
        target_rect.x += self.x + direction.x() * 16;
        target_rect.y += self.y + direction.y() * 16;
        if target_rect.x < 0 || target_rect.y < 0 || target_rect.x + target_rect.w > world.width as i32 * 16 || target_rect.y + target_rect.h > world.height as i32 * 16 {
            return false;
        }

        // TODO maybe you should fix this . you are very naughty
        // if target_tile == player.occupied_tile {
        //     return false;
        // }

        return !world.collide_entity(target_rect, player, self.height, entity_list);
    }

    /// TODO: Account for collider offset
    pub fn get_standing_tile(&self) -> (u32, u32) {
        (
            ((self.x / 16) + self.collider.x / 16).max(0) as u32,
            ((self.y / 16) + self.collider.y / 16).max(0) as u32
        )
    }
}