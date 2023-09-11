use std::{thread::{JoinHandle, self}, path::PathBuf};

use rfd::FileDialog;
use sdl2::keyboard::Keycode;

use crate::{world::World, game::{Input, Transition, TransitionType, WarpPos, IntProperty, LevelPropertyType}};

pub struct Debug {
    pub load_handle: Option<JoinHandle<Option<PathBuf>>>
}

impl Debug {
    pub fn update(&mut self, input: &Input, world: &mut World) {
        
        // F3 + M - Load map
        if input.get_pressed(Keycode::F3) && input.get_just_pressed(Keycode::M) {
            world.paused = true;
            self.load_handle = Some(thread::spawn(|| {
                FileDialog::new()
                    .add_filter("map", &["tmx"])
                    .set_directory("res/maps/")
                    .pick_file()
            }));
        }

        // F3 + D - warp to dev map
        if input.get_pressed(Keycode::F3) && input.get_just_pressed(Keycode::D) {
            world.queued_load = Some(
                crate::game::QueuedLoad { map: "res/maps/dev.tmx".to_string(), pos: WarpPos {
                    x: IntProperty::Level(LevelPropertyType::DefaultX),
                    y: IntProperty::Level(LevelPropertyType::DefaultY)
                } }
            );
            world.transition = Some(
                Transition::new(TransitionType::Lines(1), 2, true, 5)
            );
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
                        world.transition = Some(Transition::new(TransitionType::Fade, 8, true, 0));
                    }
                }
                world.paused = false;
            }
        }
    }
}