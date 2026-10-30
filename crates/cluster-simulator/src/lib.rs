//! Deterministic read-only simulated telemetry. No interfaces to real ECUs.

use cluster_core::{DataQuality, Gear, LampState, Reading, TelemetrySource, VehicleSnapshot, WarningLights};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario { Drive, ColdStart, EngineFault, SensorFailure, LinkLost }

impl Scenario {
    pub const ALL: [Self; 5] = [Self::Drive, Self::ColdStart, Self::EngineFault, Self::SensorFailure, Self::LinkLost];
    pub fn label(self) -> &'static str {
        match self {
            Self::Drive => "Conduite normale", Self::ColdStart => "Démarrage à froid",
            Self::EngineFault => "Défaut moteur", Self::SensorFailure => "Capteurs indisponibles",
            Self::LinkLost => "Connexion perdue",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lamp { Engine, Oil, Battery, Abs, Airbag, Brake, Seatbelt, Left, Right, Beam }

#[derive(Debug, Clone)]
pub enum Command {
    Accelerator(f64), Brake(f64), ShiftUp, ShiftDown, ToggleEngine,
    SetScenario(Scenario), ToggleLamp(Lamp), Pause, ResetTrip, ResetAll,
}

#[derive(Debug, Clone)]
pub struct Simulator {
    pub speed_kmh: f64,
    pub rpm: f64,
    pub coolant_c: f64,
    pub fuel_pct: f64,
    pub voltage: f64,
    pub accelerator_pct: f64,
    pub brake_pct: f64,
    pub gear: Gear,
    pub engine_running: bool,
    pub paused: bool,
    pub scenario: Scenario,
    pub trip_km: f64,
    pub odometer_km: f64,
    pub elapsed_s: f64,
    lamps: [bool; 10],
}

impl Default for Simulator {
    fn default() -> Self {
        Self {
            speed_kmh: 42.0, rpm: 2200.0, coolant_c: 82.0,
            fuel_pct: 68.0, voltage: 14.1, accelerator_pct: 26.0, brake_pct: 0.0,
            gear: Gear::Third, engine_running: true, paused: false,
            scenario: Scenario::Drive, trip_km: 0.0, odometer_km: 182_450.0,
            elapsed_s: 0.0, lamps: [false; 10],
        }
    }
}

impl Simulator {
    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Accelerator(v) => self.accelerator_pct = v.clamp(0.0, 100.0),
            Command::Brake(v) => self.brake_pct = v.clamp(0.0, 100.0),
            Command::ShiftUp => self.gear = self.gear.higher(),
            Command::ShiftDown => self.gear = self.gear.lower(),
            Command::ToggleEngine => { self.engine_running = !self.engine_running; self.voltage = if self.engine_running { 14.1 } else { 12.3 }; },
            Command::SetScenario(s) => {
                self.scenario = s;
                if s == Scenario::ColdStart { self.coolant_c = 22.0; self.speed_kmh = 0.0; self.gear = Gear::Neutral; }
            }
            Command::ToggleLamp(lamp) => {
                let index = match lamp {
                    Lamp::Engine => 0, Lamp::Oil => 1, Lamp::Battery => 2,
                    Lamp::Abs => 3, Lamp::Airbag => 4, Lamp::Brake => 5,
                    Lamp::Seatbelt => 6, Lamp::Left => 7, Lamp::Right => 8, Lamp::Beam => 9,
                };
                self.lamps[index] = !self.lamps[index];
            }
            Command::Pause => self.paused = !self.paused,
            Command::ResetTrip => self.trip_km = 0.0,
            Command::ResetAll => *self = Self::default(),
        }
    }

    pub fn lamp_active(&self, lamp: Lamp) -> bool {
        match lamp {
            Lamp::Engine => self.lamps[0] || self.scenario == Scenario::EngineFault,
            Lamp::Oil => self.lamps[1], Lamp::Battery => self.lamps[2],
            Lamp::Abs => self.lamps[3], Lamp::Airbag => self.lamps[4],
            Lamp::Brake => self.lamps[5], Lamp::Seatbelt => self.lamps[6],
            Lamp::Left => self.lamps[7], Lamp::Right => self.lamps[8], Lamp::Beam => self.lamps[9],
        }
    }

    pub fn step(&mut self, delta: Duration) {
        if self.paused { return; }
        let dt = delta.as_secs_f64().clamp(0.0, 0.2);
        if dt <= 0.0 { return; }
        self.elapsed_s += dt;
        // Intentionally approximate physics for animation tests, NOT calibration.
        let throttle = if self.engine_running && self.gear != Gear::Neutral {
            self.accelerator_pct / 100.0
        } else { 0.0 };
        let drag = 0.18 + self.speed_kmh * 0.008;
        let acceleration = throttle * 4.5 - drag - self.brake_pct / 100.0 * 10.0;
        let old_speed = self.speed_kmh;
        self.speed_kmh = (self.speed_kmh + acceleration * dt * 3.6).clamp(0.0, 210.0);
        let target_rpm = if !self.engine_running { 0.0 }
            else if self.gear == Gear::Neutral { 760.0 + self.accelerator_pct * 35.0 }
            else { (self.speed_kmh * self.gear.rpm_per_kmh()).clamp(760.0, 6900.0) };
        self.rpm += (target_rpm - self.rpm) * (dt * 6.0).clamp(0.0, 1.0);
        self.rpm = self.rpm.clamp(0.0, 7000.0);
        let cruise_temp = if self.engine_running { 90.0 } else { 30.0 };
        self.coolant_c += (cruise_temp - self.coolant_c) * dt * 0.016;
        self.voltage += ((if self.engine_running { 14.1 } else { 12.3 }) - self.voltage) * dt;
        let travelled = ((old_speed + self.speed_kmh) / 2.0) * dt / 3600.0;
        self.trip_km += travelled;
        self.odometer_km += travelled;
        if self.engine_running { self.fuel_pct = (self.fuel_pct - dt * (0.0004 + throttle * 0.001)).max(0.0); }
    }

    fn lamp_states(&self) -> WarningLights {
        let states = self.lamps.map(LampState::from_bool);
        WarningLights {
            engine: if self.scenario == Scenario::EngineFault { LampState::On } else { states[0] },
            oil_pressure: states[1], battery: states[2], abs: states[3],
            airbag: states[4], brake: states[5], seatbelt: states[6],
            left_indicator: states[7], right_indicator: states[8], main_beam: states[9],
        }
    }
}

impl TelemetrySource for Simulator {
    fn snapshot(&self) -> VehicleSnapshot {
        let mut reading = VehicleSnapshot {
            speed_kmh: Reading::simulated(self.speed_kmh),
            rpm: Reading::simulated(self.rpm),
            coolant_c: Reading::simulated(self.coolant_c),
            fuel_pct: Reading::simulated(self.fuel_pct),
            battery_v: Reading::simulated(self.voltage),
            trip_km: Reading::simulated(self.trip_km),
            simulated_odometer_km: Reading::simulated(self.odometer_km),
            gear: self.gear,
            engine_running: self.engine_running,
            lights: self.lamp_states(),
            elapsed_simulation_s: self.elapsed_s,
        };
        if self.scenario == Scenario::SensorFailure {
            reading.speed_kmh = Reading::unknown();
            reading.rpm = Reading::unknown();
            reading.coolant_c = Reading::unknown();
        } else if self.scenario == Scenario::LinkLost {
            reading.speed_kmh.quality = DataQuality::Stale;
            reading.rpm.quality = DataQuality::Stale;
            reading.coolant_c.quality = DataQuality::Stale;
            reading.fuel_pct.quality = DataQuality::Stale;
            reading.battery_v.quality = DataQuality::Stale;
            reading.lights = WarningLights::unknown();
        }
        reading
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brake_reduces_speed() {
        let mut sim = Simulator::default();
        sim.apply(Command::Accelerator(0.0));
        sim.apply(Command::Brake(100.0));
        let before = sim.speed_kmh;
        sim.step(Duration::from_millis(100));
        assert!(sim.speed_kmh < before);
    }

    #[test]
    fn driving_increases_trip_and_odometer() {
        let mut sim = Simulator::default();
        sim.step(Duration::from_millis(100));
        assert!(sim.trip_km > 0.0);
        assert!(sim.odometer_km > 182_450.0);
    }

    #[test]
    fn pause_freezes_snapshot() {
        let mut sim = Simulator::default();
        sim.apply(Command::Pause);
        let before = sim.snapshot();
        sim.step(Duration::from_millis(100));
        assert_eq!(sim.snapshot(), before);
    }

    #[test]
    fn dropout_never_reuses_last_speed_as_valid() {
        let mut sim = Simulator::default();
        sim.apply(Command::SetScenario(Scenario::SensorFailure));
        assert!(sim.snapshot().speed_kmh.usable().is_none());
        sim.apply(Command::SetScenario(Scenario::LinkLost));
        assert!(sim.snapshot().speed_kmh.usable().is_none());
        assert_eq!(sim.snapshot().lights.engine, LampState::Unknown);
    }

    #[test]
    fn simulator_does_not_exceed_bounds_even_with_bad_commands() {
        let mut sim = Simulator::default();
        sim.apply(Command::Accelerator(999.0));
        sim.apply(Command::Brake(-10.0));
        for _ in 0..10_000 { sim.step(Duration::from_millis(100)); }
        assert!((0.0..=210.0).contains(&sim.speed_kmh));
        assert!((0.0..=100.0).contains(&sim.fuel_pct));
        assert!((0.0..=7000.0).contains(&sim.rpm));
    }

    #[test]
    fn switching_off_engine_drops_rpm() {
        let mut sim = Simulator::default();
        sim.apply(Command::ToggleEngine);
        for _ in 0..100 { sim.step(Duration::from_millis(100)); }
        assert!(sim.rpm < 1.0);
    }
}
