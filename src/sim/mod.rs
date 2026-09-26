//! The simulation, with no engine in it (§21).
//!
//! Everything here is plain Rust on plain data. The Bevy view reads from it
//! and calls the player verbs; the headless runner just prints what happens.

pub mod action;
pub mod attribution;
pub mod bees;
pub mod bison;
pub mod calendar;
pub mod character;
pub mod chronicle;
pub mod civic;
pub mod debug;
pub mod economy;
pub mod events;
pub mod family;
pub mod farmwork;
pub mod geography;
pub mod ghosts;
pub mod history;
pub mod homestead;
pub mod institutions;
pub mod intrigue;
pub mod land;
pub mod law;
pub mod legacy;
pub mod life;
pub mod mail;
pub mod market;
pub mod mortality;
pub mod nations;
pub mod psyche;
pub mod railroad;
pub mod reconcile;
pub mod rng;
pub mod romance;
pub mod systems;
pub mod world;

pub use calendar::Day;
pub use events::{EventId, EventKind, FireCause, Suspect, WorldEvent};
pub use world::{Faction, NpcId, PLAYER, World};

#[cfg(test)]
mod tests;
