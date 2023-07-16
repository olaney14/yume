use crate::player::Player;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    Glasses,
    Speed
}

impl Effect {
    pub fn parse(source: &str) -> Option<Self> {
        match source {
            "glasses" => Some(Self::Glasses),
            "shoes" => Some(Self::Speed),
            _ => None
        }
    }

    pub fn description(&self) -> &str {
        use Effect::*;
        match self {
            Glasses => "Put on glasses",
            Speed => "Put on running shoes"
        }
    }

    pub fn name(&self) -> &str {
        use Effect::*;
        match self {
            Glasses => "Glasses",
            Speed => "Running shoes"
        }
    }

    // in theory we could have 4294967295 effects
    pub fn order(&self) -> u32 {
        use Effect::*;
        match self {
            Glasses => 0,
            Speed => 1
        }
    }

    pub fn apply(&self, player: &mut Player) {
        use Effect::*;
        match self {
            Speed => {
                player.speed *= 2;
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
            },
            _ => ()
        }
    }
}