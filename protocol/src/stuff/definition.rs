use bevy::prelude::*;
use lightyear::core::tick::Tick;
use serde::{Deserialize, Serialize};

use crate::net::config::TIMESTEP;

#[derive(Component, Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Interactable {
    /// cooldown between interactions as a positive number representing seconds
    pub delay_secs: f64,
    /// tick of the last accepted interaction
    /// written by server and read by client
    pub last_interacted: Tick,
}

impl Interactable {
    pub fn ready(&self, now: Tick) -> bool {
        now - self.last_interacted >= (self.delay_secs / TIMESTEP) as i32
    }
}
