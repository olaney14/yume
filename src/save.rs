use rodio::static_buffer::StaticSamplesBuffer;
use sdl2::render::TextureCreator;
use serde_derive::{Serialize, Deserialize};

use crate::{player::{Player, Statistics}, effect::Effect, world::{self, World}};

#[derive(Serialize, Deserialize)]
pub struct SerializablePlayer {
    pub unlocked_effects: Vec<SerializableEffect>,
    pub money: u32,
    pub stats: Statistics
}

impl SerializablePlayer {
    pub fn from_player(player: &Player) -> Self {
        let mut unlocked_effects = Vec::new();
        for effect in player.unlocked_effects.iter() {
            unlocked_effects.push(SerializableEffect::from_effect(effect));
        }
        Self {
            unlocked_effects,
            money: player.money,
            stats: player.stats.clone()
        }
    }

    pub fn to_player<'a, T>(&self, creator: &'a TextureCreator<T>) -> Player<'a> {
        let mut player = Player::new(creator);
        for effect in self.unlocked_effects.iter() {
            player.unlocked_effects.push(effect.to_effect());
        }
        player
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializableEffect {
    pub effect: String
}

impl SerializableEffect {
    pub fn from_effect(effect: &Effect) -> Self {
        Self {
            effect: effect.name().to_string()
        }
    }

    pub fn to_effect(&self) -> Effect {
        Effect::parse(&self.effect).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub player: SerializablePlayer
}

impl SaveData {
    pub fn create(player: &Player) -> Self {
        Self {
            player: SerializablePlayer::from_player(player)
        }
    } 

    pub fn get_player<'a, T>(&self, creator: &'a TextureCreator<T>) -> Player<'a> {
        self.player.to_player(creator)
    }
}