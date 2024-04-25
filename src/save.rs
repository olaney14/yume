use std::{collections::BTreeMap, error::Error, fs::File, path::{Path, PathBuf}};

use sdl2::render::TextureCreator;
use serde_derive::{Serialize, Deserialize};

use crate::{player::{Player, Statistics}, effect::Effect};

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
            effect: effect.parsable().to_string()
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

    pub fn save(&self, id: u32, name: &PathBuf, saves: &mut SaveInfo) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(name)?;
        serde_cbor::to_writer(&mut file, &self)?;

        saves.update(id, SaveSlot::new(name, self.player.unlocked_effects.len()));

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct SaveSlot {
    pub file: String,
    pub effects: usize,
}

impl SaveSlot {
    pub fn new(path: &PathBuf, effects: usize) -> Self {
        Self { effects, file: path.to_str().expect("invalid save file name").to_string() }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SaveInfo {
    pub files: BTreeMap<u32, SaveSlot>,
    //pub files_ordered: BTreeMap<usize, SaveSlot>
}

impl SaveInfo {
    pub fn update(&mut self, new_id: u32, new_save: SaveSlot) {
        self.files.insert(new_id, new_save);
        self.write().expect("failed to update save info");
    }

    pub fn read() -> Result<Self, Box<dyn Error>> {
        let file = File::open("saves/.saves")?;
        let read: Result<SaveInfo, serde_cbor::Error> = serde_cbor::from_reader(&file);
        if let Ok(save) = read {
            return Ok(save);
        } else {
            return Err(Box::new(read.err().unwrap()));
        }
    }

    pub fn create_new() -> Result<Self, Box<dyn Error>> {
        let save_data = SaveInfo {
            files: BTreeMap::new(),
            //files_ordered: BTreeMap::new()
        };
        let mut write = File::create("saves/.saves")?;
        serde_cbor::to_writer(&mut write, &save_data)?;
        Ok(save_data)
    }

    pub fn read_or_create_new() -> Result<Self, Box<dyn Error>> {
        let saves_path = Path::new("saves/.saves");
        if saves_path.exists() {
            return SaveInfo::read();
        } else {
            return SaveInfo::create_new();
        }
    }

    pub fn write(&self) -> Result<(), Box<dyn Error>> {
        let mut file = File::create("saves/.saves")?;
        serde_cbor::to_writer(&mut file, self)?;

        Ok(())
    }
}