use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Debug, PartialEq)]
pub struct Interactable;
