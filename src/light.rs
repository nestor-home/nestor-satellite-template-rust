//! The satellite's body: what it does with the core's intents.
//!
//! This one is a status light, printed. Replace the prints with your hardware: an LED ring, a
//! screen, motors. Every method of `Body` is optional; see the others (`expression`, `beat`,
//! `heartbeat`) in the SDK.
//!
//! Calls come from the network task and must return at once: keep the state here and drive the
//! hardware from a task or thread of your own.

use nestor_satellite::{Behavior, Body};

/// A status light.
pub struct Light;

impl Body for Light {
    fn behavior(&self, behavior: Behavior) {
        // TODO(hardware): show the assistant's attitude (idle, listening, thinking, speaking) until the next.
        println!("light: {behavior}");
    }

    fn identify(&self) {
        // TODO(hardware): blink for a few seconds, so a person tells this device apart from others.
        // It is called while the device waits to be paired, and on `intent.identify` once paired.
        println!("light: blink");
    }

    fn connected(&self, up: bool) {
        // TODO(hardware): a dim light when the link to the core is down, for instance.
        println!("light: {}", if up { "connected" } else { "disconnected" });
    }
}
