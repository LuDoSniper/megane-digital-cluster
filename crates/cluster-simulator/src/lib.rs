//! Deterministic offline simulator. NEVER sends commands to a real vehicle.

use cluster_core::{
    AssistanceMode, DataQuality, Gear, LampState, Reading, SpeedAssistance,
    TelemetrySource, VehicleSnapshot, WarningLights,
};
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
pub enum Lamp { Engine, Oil, Battery, Abs, Airbag, Brake, Seatbelt, Left, Right, Beam, Dipped }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    Accelerator(f64), Brake(f64), ShiftUp, ShiftDown, SetGear(Gear), ToggleEngine,
    SetScenario(Scenario), ToggleLamp(Lamp), ToggleHazards,
    SetAssistance(AssistanceMode), SetAssistanceSpeed, AdjustAssistance(f64),
    CancelAssistance, ResumeAssistance, Pause, ResetTrip, ResetAll,
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
    pub hazards: bool,
    pub assistance: SpeedAssistance,
    lamps: [bool; 11],
}

impl Default for Simulator {
    fn default() -> Self {
        Self {
            speed_kmh: 42.0, rpm: 2200.0, coolant_c: 82.0,
            fuel_pct: 68.0, voltage: 14.1, accelerator_pct: 26.0, brake_pct: 0.0,
            gear: Gear::Third, engine_running: true, paused: false,
            scenario: Scenario::Drive, trip_km: 0.0, odometer_km: 182_450.0,
            elapsed_s: 0.0, hazards: false, assistance: SpeedAssistance::default(),
            lamps: [false; 11],
        }
    }
}

impl Simulator {
    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Accelerator(v) => self.accelerator_pct = v.clamp(0.0, 100.0),
            Command::Brake(v) => {
                self.brake_pct = v.clamp(0.0, 100.0);
                if self.brake_pct > 2.0 && self.assistance.mode == AssistanceMode::Cruise {
                    self.assistance.active = false;
                }
            }
            Command::ShiftUp => self.set_gear(self.gear.higher()),
            Command::ShiftDown => self.set_gear(self.gear.lower()),
            Command::SetGear(gear) => self.set_gear(gear),
            Command::ToggleEngine => {
                self.engine_running = !self.engine_running;
                self.voltage = if self.engine_running { 14.1 } else { 12.3 };
                if !self.engine_running { self.assistance.active = false; }
            }
            Command::SetScenario(s) => {
                self.scenario = s;
                if s == Scenario::ColdStart {
                    self.coolant_c = 22.0;
                    self.speed_kmh = 0.0;
                    self.gear = Gear::Neutral;
                    self.assistance.active = false;
                }
            }
            Command::ToggleLamp(lamp) => {
                let index = Self::lamp_index(lamp);
                self.lamps[index] = !self.lamps[index];
                if matches!(lamp, Lamp::Left | Lamp::Right) { self.hazards = false; }
            }
            Command::ToggleHazards => {
                self.hazards = !self.hazards;
                if self.hazards { self.lamps[7] = false; self.lamps[8] = false; }
            }
            Command::SetAssistance(mode) => {
                self.assistance.mode = mode;
                self.assistance.active = false;
                self.assistance.temporarily_overridden = false;
                self.assistance.setpoint_kmh = None;
            }
            Command::SetAssistanceSpeed => self.engage_assistance(),
            Command::AdjustAssistance(delta) => {
                if self.assistance.mode != AssistanceMode::Off {
                    let current = self.assistance.setpoint_kmh.unwrap_or_else(|| self.speed_kmh.max(30.0));
                    self.assistance.setpoint_kmh = Some((current + delta).clamp(30.0, 200.0));
                }
            }
            Command::CancelAssistance => self.assistance.active = false,
            Command::ResumeAssistance => {
                if self.engine_running && self.speed_kmh >= 30.0
                    && self.assistance.mode != AssistanceMode::Off
                    && self.assistance.setpoint_kmh.is_some() {
                    self.assistance.active = true;
                }
            }
            Command::Pause => self.paused = !self.paused,
            Command::ResetTrip => self.trip_km = 0.0,
            Command::ResetAll => *self = Self::default(),
        }
    }

    fn set_gear(&mut self, gear: Gear) {
        // Simulated interlock; real reverse engagement must never be commanded via telemetry.
        if (gear == Gear::Reverse || self.gear == Gear::Reverse) && self.speed_kmh > 2.0 {
            return;
        }
        self.gear = gear;
        if self.assistance.mode == AssistanceMode::Cruise { self.assistance.active = false; }
    }

    fn engage_assistance(&mut self) {
        if !self.engine_running || self.speed_kmh < 30.0 || self.gear == Gear::Reverse
            || self.gear == Gear::Neutral || self.assistance.mode == AssistanceMode::Off {
            return;
        }
        self.assistance.setpoint_kmh = Some(self.speed_kmh.round().clamp(30.0, 200.0));
        self.assistance.active = true;
    }

    fn lamp_index(lamp: Lamp) -> usize {
        match lamp {
            Lamp::Engine => 0, Lamp::Oil => 1, Lamp::Battery => 2, Lamp::Abs => 3,
            Lamp::Airbag => 4, Lamp::Brake => 5, Lamp::Seatbelt => 6,
            Lamp::Left => 7, Lamp::Right => 8, Lamp::Beam => 9, Lamp::Dipped => 10,
        }
    }

    pub fn lamp_active(&self, lamp: Lamp) -> bool {
        self.lamps[Self::lamp_index(lamp)] ||
            (lamp == Lamp::Engine && self.scenario == Scenario::EngineFault) ||
            (self.hazards && matches!(lamp, Lamp::Left | Lamp::Right))
    }

    pub fn indicator_lit(&self) -> bool {
        // ~90 flashes/minute, about 0.333 s on / 0.333 s off.
        self.elapsed_s.rem_euclid(2.0 / 3.0) < 1.0 / 3.0
    }

    pub fn step(&mut self, delta: Duration) {
        if self.paused { return; }
        let dt = delta.as_secs_f64().clamp(0.0, 0.2);
        if dt <= 0.0 { return; }
        self.elapsed_s += dt;
        // Deliberately approximate physics for visual tests, not ECU calibration.
        let mut throttle = if self.engine_running && self.gear != Gear::Neutral {
            self.accelerator_pct / 100.0
        } else { 0.0 };

        self.assistance.temporarily_overridden = false;
        if self.assistance.active {
            if let Some(target) = self.assistance.setpoint_kmh {
                match self.assistance.mode {
                    AssistanceMode::Cruise => {
                        if self.brake_pct > 2.0 || !self.engine_running || self.gear == Gear::Neutral || self.gear == Gear::Reverse {
                            self.assistance.active = false;
                        } else if throttle > 0.55 {
                            self.assistance.temporarily_overridden = true;
                        } else {
                            throttle = ((target - self.speed_kmh) * 0.055 + 0.20).clamp(0.0, 0.95);
                        }
                    }
                    AssistanceMode::Limiter => {
                        // Pedal "kickdown" demonstrator: deliberate override above 95%.
                        if self.accelerator_pct >= 95.0 {
                            self.assistance.temporarily_overridden = true;
                        } else if self.speed_kmh >= target { throttle = 0.0; }
                        else if self.speed_kmh > target - 2.0 { throttle *= 0.2; }
                    }
                    AssistanceMode::Off => self.assistance.active = false,
                }
            }
        }
        let drag = 0.18 + self.speed_kmh * 0.008;
        let acceleration = throttle * 4.5 - drag - self.brake_pct / 100.0 * 10.0;
        let old_speed = self.speed_kmh;
        let max_speed = if self.gear == Gear::Reverse { 25.0 } else { 210.0 };
        self.speed_kmh = (self.speed_kmh + acceleration * dt * 3.6).clamp(0.0, max_speed);
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
        let lamp = |which: Lamp| LampState::from_bool(self.lamp_active(which));
        let indicator = |which: Lamp| LampState::from_bool(self.lamp_active(which) && self.indicator_lit());
        WarningLights {
            engine: lamp(Lamp::Engine), oil_pressure: lamp(Lamp::Oil),
            battery: lamp(Lamp::Battery), abs: lamp(Lamp::Abs), airbag: lamp(Lamp::Airbag),
            brake: lamp(Lamp::Brake), seatbelt: lamp(Lamp::Seatbelt),
            left_indicator: indicator(Lamp::Left), right_indicator: indicator(Lamp::Right),
            main_beam: lamp(Lamp::Beam), dipped_beam: lamp(Lamp::Dipped),
        }
    }
}

impl TelemetrySource for Simulator {
    fn snapshot(&self) -> VehicleSnapshot {
        let mut reading = VehicleSnapshot {
            speed_kmh: Reading::simulated(self.speed_kmh),
            rpm: Reading::simulated(self.rpm), coolant_c: Reading::simulated(self.coolant_c),
            fuel_pct: Reading::simulated(self.fuel_pct), battery_v: Reading::simulated(self.voltage),
            trip_km: Reading::simulated(self.trip_km),
            simulated_odometer_km: Reading::simulated(self.odometer_km),
            gear: self.gear, engine_running: self.engine_running,
            assistance: self.assistance, lights: self.lamp_states(),
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
            reading.assistance = SpeedAssistance::default();
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
    fn reverse_is_protected_while_moving_and_available_at_rest() {
        let mut sim = Simulator::default();
        sim.apply(Command::SetGear(Gear::Reverse));
        assert_eq!(sim.gear, Gear::Third);
        sim.speed_kmh = 0.0;
        sim.apply(Command::SetGear(Gear::Reverse));
        assert_eq!(sim.gear, Gear::Reverse);
        sim.apply(Command::Accelerator(100.0));
        for _ in 0..100 { sim.step(Duration::from_millis(100)); }
        assert!(sim.speed_kmh <= 25.0);
    }

    #[test]
    fn indicators_blink_and_hazards_flash_both_sides() {
        let mut sim = Simulator::default();
        sim.apply(Command::ToggleLamp(Lamp::Left));
        assert_eq!(sim.snapshot().lights.left_indicator, LampState::On);
        sim.step(Duration::from_millis(400)); // clamped to 200 ms per frame
        sim.step(Duration::from_millis(200));
        assert_eq!(sim.snapshot().lights.left_indicator, LampState::Off);
        sim.apply(Command::ToggleHazards);
        assert!(sim.lamp_active(Lamp::Left) && sim.lamp_active(Lamp::Right));
    }

    #[test]
    fn cruise_cancels_on_brake_and_resumes() {
        let mut sim = Simulator::default();
        sim.apply(Command::SetAssistance(AssistanceMode::Cruise));
        sim.apply(Command::SetAssistanceSpeed);
        assert!(sim.assistance.active);
        sim.apply(Command::Brake(20.0));
        assert!(!sim.assistance.active);
        sim.apply(Command::Brake(0.0));
        sim.apply(Command::ResumeAssistance);
        assert!(sim.assistance.active);
    }

    #[test]
    fn limiter_can_be_overridden_using_kickdown() {
        let mut sim = Simulator::default();
        sim.apply(Command::SetAssistance(AssistanceMode::Limiter));
        sim.apply(Command::SetAssistanceSpeed);
        sim.apply(Command::Accelerator(100.0));
        sim.step(Duration::from_millis(50));
        assert!(sim.assistance.temporarily_overridden);
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
