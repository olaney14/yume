use std::{thread::{JoinHandle, self}, path::PathBuf, time::{Instant, Duration}, collections::{HashMap, LinkedList}};

use rfd::FileDialog;
use sdl2::{keyboard::Keycode, render::{Canvas, RenderTarget}};

use crate::{audio::SoundEffectBank, effect, game::{Input, IntProperty, LevelPropertyType, RenderState, WarpPos}, player::Player, transitions::{Transition, TransitionType}, ui::{Font, Ui}, world::World};

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum ProfileTargetType {
    HandleEvents,
    UIUpdate,
    PlayerUpdate,
    WorldUpdate,
    InputUpdate,
    ClampCamera,
    WorldDraw,
    UIDraw,
    Frame,
    Loop
}

pub struct ProfileTarget {
    pub start: Option<Instant>,
    pub end: Option<Instant>
    //pub duration: Option<Duration>
}

impl ProfileTarget {
    fn new() -> Self {
        Self {
            start: None, end: None
        }
    }
}

const FRAME_AVG_SAMPLE: usize = 100;
const SPIKE_LIMIT: u32 = 10;

pub struct ProfileInfo {
    stages: HashMap<ProfileTargetType, ProfileTarget>,
    past_frames: LinkedList<Duration>
}

impl ProfileInfo {
    pub fn new() -> Self {
        let mut stages = HashMap::new();
        stages.insert(ProfileTargetType::HandleEvents, ProfileTarget::new());
        stages.insert(ProfileTargetType::UIUpdate, ProfileTarget::new());
        stages.insert(ProfileTargetType::PlayerUpdate, ProfileTarget::new());
        stages.insert(ProfileTargetType::WorldUpdate, ProfileTarget::new());
        stages.insert(ProfileTargetType::InputUpdate, ProfileTarget::new());
        stages.insert(ProfileTargetType::ClampCamera, ProfileTarget::new());
        stages.insert(ProfileTargetType::WorldDraw, ProfileTarget::new());
        stages.insert(ProfileTargetType::UIDraw, ProfileTarget::new());
        stages.insert(ProfileTargetType::Frame, ProfileTarget::new());
        stages.insert(ProfileTargetType::Loop, ProfileTarget::new());
        Self {
            stages, past_frames: LinkedList::new()
        }
    }
    
    #[inline]
    pub fn begin_stage(&mut self, stage: ProfileTargetType) {
        if self.stages.contains_key(&stage) {
            self.stages.get_mut(&stage).unwrap().start = Some(Instant::now());
        }
    }

    #[inline]
    pub fn end_stage(&mut self, stage: ProfileTargetType) {
        if self.stages.contains_key(&stage) {
            self.stages.get_mut(&stage).unwrap().end = Some(Instant::now());
        }
    }

    #[inline]
    pub fn get_stage_timing(&self, stage: &ProfileTargetType) -> Option<Duration> {
        if self.stages.contains_key(stage) {
            let stage = self.stages.get(&stage).unwrap();
            if stage.end.is_some() && stage.start.is_some() {
                return Some(*stage.end.as_ref().unwrap() - *stage.start.as_ref().unwrap())
            }
        }
        return None
    }
}

pub struct Debug<'a> {
    pub load_handle: Option<JoinHandle<Option<PathBuf>>>,
    pub profiler: ProfileInfo,
    pub enable_profiling: bool,
    pub enable_debug_overlay: bool,
    pub mini_font: Font<'a>
}

fn f3_combo(input: &Input, key: Keycode) -> bool {
    input.get_pressed(Keycode::F3) && input.get_just_pressed(key)
    || input.get_pressed(Keycode::LAlt) && input.get_just_pressed(key)
}

impl<'a> Debug<'a> {
    pub fn update(&mut self, input: &Input, world: &mut World, player: &mut Player, sfx: &mut SoundEffectBank) {
        
        // F3 + M - Load map
        if f3_combo(input, Keycode::M) {
            world.paused = true;
            self.load_handle = Some(thread::spawn(|| {
                FileDialog::new()
                    .add_filter("map", &["tmx"])
                    .set_directory("res/maps/")
                    .pick_file()
            }));
            player.dreaming = true;
        }

        // F3 + D - warp to dev map
        if f3_combo(input, Keycode::D) {
            world.queued_load = Some(
                crate::game::QueuedLoad { map: "res/maps/dev.tmx".to_string(), pos: WarpPos {
                    x: IntProperty::Level(LevelPropertyType::DefaultX),
                    y: IntProperty::Level(LevelPropertyType::DefaultY)
                } }
            );
            world.transition = Some(
                Transition::new(TransitionType::FadeScreenshot, 1, 1, true, 5, false)
            );
            player.dreaming = true;
        }

        // F3 + I - show debug info
        if f3_combo(input, Keycode::I) {
            self.enable_debug_overlay = !self.enable_debug_overlay;
            sfx.play("click-21156");
        }

        // F3 + P - show profiling info
        if f3_combo(input, Keycode::P) {
            self.enable_profiling = !self.enable_profiling;
            sfx.play("click-21156");
        }

        // F3 + S - teleport one space forward
        if f3_combo(input, Keycode::S) && !player.moving {
            player.set_pos(player.x + player.facing.x() * 16, player.y + player.facing.y() * 16);
        }

        // F3 + F - print all flags
        if f3_combo(input, Keycode::F) {
            println!("===Global Flags===");
            for (i, v) in world.global_flags.iter() {
                println!("{}: {}", i, v);
            }

            println!("===Local Flags===");
            for (i, v) in world.flags.iter() {
                println!("{}: {}", i, v);
            }
            sfx.play("click-21156");
        }

        // F3 + R - reload map from file
        if f3_combo(input, Keycode::R) {
            world.special_context.reload_on_warp = true;
            world.queued_load = Some(
                crate::game::QueuedLoad { map: world.source_file.as_os_str().to_string_lossy().to_string(),
                    pos: WarpPos {
                        x: IntProperty::Int(player.x / 16),
                        y: IntProperty::Int(player.y / 16)
                    }
                }
            );

            world.transition = Some(
                Transition::new(TransitionType::Fade, 4, 1, true, 5, false)
            );
        }

        // // F3 + O - optimize map files
        // if f3_combo(input, Keycode::O) {
        //     match optimize::optimize_all(&PathBuf::from("res/maps/"), creator) {
        //         Err(e) => {
        //             eprintln!("Error in map optimization: {}", e);
        //         }
        //         Ok(()) => {
        //             println!("Map optimization complete");
        //         }
        //     }
        //     sfx.play("click-21156");
        // }

        // F3 + E - Give all items
        if f3_combo(input, Keycode::E) {
            player.give_effect(effect::Effect::Bat);
            player.give_effect(effect::Effect::Fire);
            player.give_effect(effect::Effect::Glasses);
            player.give_effect(effect::Effect::Security);
            player.give_effect(effect::Effect::Speed);
            sfx.play("click-21156");
        }

        if self.load_handle.is_some() {
            if self.load_handle.as_ref().unwrap().is_finished() {
                let handle = self.load_handle.take().unwrap();
                if let Ok(path_opt) = handle.join() {
                    if let Some(path) = path_opt {
                        world.queued_load = Some(
                            crate::game::QueuedLoad { map: path.to_str().unwrap().to_string(), pos: 
                                WarpPos {
                                    x: IntProperty::Level(LevelPropertyType::DefaultX),
                                    y: IntProperty::Level(LevelPropertyType::DefaultY)
                                }
                            });
                        world.transition = Some(Transition::new(TransitionType::Fade, 8, 0, true, 0, false));
                    }
                }
                world.paused = false;
            }
        }
    }

    pub fn draw<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, ui: &Ui, player: &Player, state: &RenderState) {
        if self.enable_profiling {
            self.profiler.past_frames.push_front(self.profiler.get_stage_timing(&ProfileTargetType::Frame).unwrap_or(Duration::ZERO));
            if self.profiler.past_frames.len() >= FRAME_AVG_SAMPLE {
                self.profiler.past_frames.pop_back();
            }
            
            let avg: u128 = self.profiler.past_frames.iter().map(|f| f.as_nanos()).reduce(|a, e| a + e).unwrap() / self.profiler.past_frames.len() as u128;
            let avg_dur = Duration::from_nanos(avg.try_into().unwrap());
            if self.profiler.get_stage_timing(&ProfileTargetType::Frame).unwrap_or(Duration::ZERO).as_nanos() > avg * SPIKE_LIMIT as u128 {
                println!("SPIKE: {:?} at avg {:?}", self.profiler.get_stage_timing(&ProfileTargetType::Frame).unwrap_or(Duration::ZERO), Duration::from_nanos(avg as u64));
            }

            ui.theme.clear_frame(canvas, 8,/*(state.screen_extents.0 - 172) / 16 */ 0, 12, 16);
            //ui.theme.clear_frame(canvas, (200 - (16 * 4)) / 16, 150 / 16, 8, 2);
            ui.theme.draw_frame(canvas, state.screen_extents.0 - 172, 0, 12, 16);
            let text_x = state.screen_extents.0 as i32 - 172 + 6;
            let mut y = 4;
            for stage in self.profiler.stages.keys() {
                let timing = self.profiler.get_stage_timing(stage);
                if let Some(timing) = timing {
                    ui.theme.font.draw_string(
                        canvas, 
                        format!("{:?}: {:?}", stage, timing).as_str(), 
                        (text_x, y)
                    );
                }
                y += 12;
            }
            ui.theme.font.draw_string(
                canvas, 
                format!("avg: {:?}", avg_dur).as_str(), 
                (text_x, y)
            );
        }

        if self.enable_debug_overlay {
            ui.theme.clear_frame(canvas, (state.screen_extents.0 - 140) / 16, 0, 9, 15);
            ui.theme.draw_frame(canvas, state.screen_extents.0 - 140, 0, 9, 15);
            let text_x = state.screen_extents.0 as i32 - 140 + 6;
            let y = 4;
            let standing_tile = player.get_standing_tile();
            self.mini_font.draw_string(canvas, format!("Tile: ({}, {})", standing_tile.0, standing_tile.1).as_str(), (text_x, y));

            ui.theme.font.draw_string(canvas, "the quick brown fox jumped over the lazy dog", (10, state.screen_extents.1 as i32 - 50));
            ui.theme.font.draw_string(canvas, "The Quick Brown Fox Jumped Over The Lazy Dog", (10, state.screen_extents.1 as i32 - 35));
            ui.theme.font.draw_string(canvas, "THE QUICK BROWN FOX JUMPED OVER THE LAZY DOG", (10, state.screen_extents.1 as i32 - 20));
        }
    }
}