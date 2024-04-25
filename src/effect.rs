use crate::player::Player;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    Glasses,
    Speed,
    Fire
}

impl Effect {
    pub fn parse(source: &str) -> Option<Self> {
        match source {
            "glasses" | "Glasses" => Some(Self::Glasses),
            "shoes" | "Running shoes" => Some(Self::Speed),
            "fire" | "Fire" => Some(Self::Fire),
            _ => None
        }
    }

    pub fn parsable(&self) -> &str {
        match self {
            Self::Fire => "fire",
            Self::Speed => "shoes",
            Self::Glasses => "glasses"
        }
    }

    pub fn description(&self) -> &str {
        use Effect::*;
        match self {
            Glasses => "Put on glasses",
            Speed => "Put on running shoes",
            Fire => "Catch on fire"
        }
    }

    pub fn name(&self) -> &str {
        use Effect::*;
        match self {
            Glasses => "Glasses",
            Speed => "Running shoes",
            Fire => "Fire"
        }
    }

    // in theory we could have 4294967295 effects
    pub fn order(&self) -> u32 {
        use Effect::*;
        match self {
            Glasses => 0,
            Speed => 1,
            Fire => 2
        }
    }

    pub fn apply(&self, player: &mut Player) {
        use Effect::*;
        match self {
            Speed => {
                player.speed *= 2;
                player.animation_info.animation_speed = 4;
            },
            _ => ()
        }
    }

    // TODO: What to do if speed is 1?
    pub fn remove(&self, player: &mut Player) {
        use Effect::*;
        match self {
            Speed => {
                player.speed /= 2;
                player.animation_info.animation_speed = 7;
            },
            _ => ()
        }
    }
}