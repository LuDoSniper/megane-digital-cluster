//! Offline vehicle dynamics and control policies for the simulator ONLY.
//! Real OBD adapters implement TelemetrySource independently and must never call this module.

mod vehicle;
pub use vehicle::VehicleProfile;

use cluster_core::{
    Aperture, AssistanceMode, BodyOpenings, DataQuality, Gear, InterruptionReason, LampState, Reading, ShiftAdvice, TyreAlert, TyreMonitoring, TyreReading, Wheel,
    SpeedAssistance, TelemetrySource, TelemetryStatus, VehicleSnapshot, WarningLights,
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
    Accelerator(f64), Brake(f64), Clutch(f64), RoadSlope(f64),
    HoldAccelerator(bool), HoldBrake(bool), ToggleOpening(Aperture), SetTyrePressure(Wheel, f64), ToggleTyreSensor(Wheel),
    ShiftUp, ShiftDown, SetGear(Gear), ToggleEngine,
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
    /// What the driver requests (0–100); NOT the resulting engine throttle.
    pub accelerator_pct: f64,
    pub brake_pct: f64,
    pub clutch_pct: f64,
    /// Positive is uphill, negative downhill; used by the simulated physics only.
    pub road_slope_pct: f64,
    pub gear: Gear,
    pub engine_running: bool,
    pub paused: bool,
    pub scenario: Scenario,
    pub trip_km: f64,
    pub odometer_km: f64,
    pub elapsed_s: f64,
    pub profile: VehicleProfile,
    pub openings: BodyOpenings,
    pub tyre_pressure_bar: [f64; 4],
    pub tyre_sensor_fault: [bool; 4],
    pub keyboard_accelerator_down: bool,
    pub keyboard_brake_down: bool,
    keyboard_accelerator_owned: bool,
    keyboard_brake_owned: bool,
    pub hazards: bool,
    pub assistance: SpeedAssistance,
    lamps: [bool; 11],
}

impl Default for Simulator {
    fn default() -> Self {
        Self {
            speed_kmh: 42.0, rpm: 2226.0, coolant_c: 82.0,
            fuel_pct: 68.0, voltage: 14.1, accelerator_pct: 0.0, brake_pct: 0.0,
            clutch_pct: 0.0, road_slope_pct: 0.0,
            gear: Gear::Third, engine_running: true, paused: false,
            scenario: Scenario::Drive, trip_km: 0.0, odometer_km: 182_450.0,
            elapsed_s: 0.0, profile: VehicleProfile::default(),
            openings: BodyOpenings { open: [false;6] },
            tyre_pressure_bar: [2.2,2.2,2.0,2.0], tyre_sensor_fault: [false;4],
            keyboard_accelerator_down: false, keyboard_brake_down: false,
            keyboard_accelerator_owned: false, keyboard_brake_owned: false,
            hazards: false, assistance: SpeedAssistance::default(),
            lamps: [false; 11],
        }
    }
}

/// Advisory algorithm belongs to the simulator. The presentation never computes a gear recommendation.
fn shift_advice(gear: Gear, rpm: f64, running: bool) -> ShiftAdvice {
    if !running || !rpm.is_finite() { return ShiftAdvice::None; }
    match gear {
        Gear::Reverse | Gear::Neutral => ShiftAdvice::None,
        Gear::Fifth if rpm < 1300.0 => ShiftAdvice::Down,
        Gear::Fifth => ShiftAdvice::None,
        Gear::First if rpm > 3100.0 => ShiftAdvice::Up,
        Gear::First => ShiftAdvice::None,
        _ if rpm > 3000.0 => ShiftAdvice::Up,
        _ if rpm < 1300.0 => ShiftAdvice::Down,
        _ => ShiftAdvice::None,
    }
}

impl Simulator {
    fn interrupt(&mut self, reason: InterruptionReason) {
        if self.assistance.active {
            self.assistance.active = false;
            self.assistance.temporarily_overridden = false;
            self.assistance.interrupted_by = reason;
        }
    }

    fn valid_control_conditions(&self) -> bool {
        self.engine_running && (30.0..=200.0).contains(&self.speed_kmh)
            && matches!(self.gear, Gear::First | Gear::Second | Gear::Third | Gear::Fourth | Gear::Fifth)
            && self.brake_pct <= 2.0 && self.clutch_pct <= 2.0
            && self.scenario != Scenario::EngineFault && self.scenario != Scenario::SensorFailure
            && self.scenario != Scenario::LinkLost
    }

    fn activate(&mut self, use_saved_setpoint: bool) {
        if self.assistance.mode == AssistanceMode::Off || !self.valid_control_conditions() { return; }
        if !use_saved_setpoint {
            self.assistance.setpoint_kmh = Some(self.speed_kmh.round().clamp(30.0, 200.0));
        }
        if self.assistance.setpoint_kmh.is_none() { return; }
        self.assistance.active = true;
        self.assistance.interrupted_by = InterruptionReason::None;
        self.assistance.temporarily_overridden = false;
        self.assistance.unable_to_hold = false;
    }

    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Accelerator(v) => { self.accelerator_pct = v.clamp(0.0,100.0); self.keyboard_accelerator_owned = false; },
            Command::HoldAccelerator(held) => { self.keyboard_accelerator_down=held; self.keyboard_accelerator_owned=true; },
            Command::HoldBrake(held) => { self.keyboard_brake_down=held; self.keyboard_brake_owned=true; },
            Command::ToggleOpening(which) => self.openings.open[which.index()] = !self.openings.open[which.index()],
            Command::SetTyrePressure(which, pressure) => { self.tyre_pressure_bar[which.index()] = pressure.clamp(0.0,3.5); },
            Command::ToggleTyreSensor(which) => { let i=which.index(); self.tyre_sensor_fault[i]=!self.tyre_sensor_fault[i]; },
            Command::Brake(v) => {
                self.brake_pct = v.clamp(0.0, 100.0); self.keyboard_brake_owned = false;
                if self.brake_pct > 2.0 && self.assistance.mode == AssistanceMode::Cruise {
                    self.interrupt(InterruptionReason::Brake);
                }
                // Braking does not cancel a speed LIMITER: it never drives the throttle.
            }
            Command::Clutch(v) => {
                self.clutch_pct = v.clamp(0.0, 100.0);
                if self.clutch_pct > 2.0 && self.assistance.mode == AssistanceMode::Cruise {
                    self.interrupt(InterruptionReason::Clutch);
                }
            }
            Command::RoadSlope(v) => self.road_slope_pct = v.clamp(-12.0, 12.0),
            Command::ShiftUp => self.set_gear(self.gear.higher()),
            Command::ShiftDown => self.set_gear(self.gear.lower()),
            Command::SetGear(gear) => self.set_gear(gear),
            Command::ToggleEngine => {
                self.engine_running = !self.engine_running;
                self.voltage = if self.engine_running { 14.1 } else { 12.3 };
                if !self.engine_running { self.interrupt(InterruptionReason::EngineStopped); }
            }
            Command::SetScenario(s) => {
                self.scenario = s;
                if s == Scenario::ColdStart {
                    self.coolant_c = 22.0;
                    self.speed_kmh = 0.0;
                    self.set_gear(Gear::Neutral);
                }
                if matches!(s, Scenario::EngineFault | Scenario::SensorFailure | Scenario::LinkLost) {
                    self.interrupt(InterruptionReason::Fault);
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
                self.assistance = SpeedAssistance { mode, ..SpeedAssistance::default() };
            }
            Command::SetAssistanceSpeed => self.activate(false),
            Command::AdjustAssistance(delta) => {
                if self.assistance.mode != AssistanceMode::Off {
                    if self.assistance.setpoint_kmh.is_some() || self.valid_control_conditions() {
                        let current = self.assistance.setpoint_kmh.unwrap_or_else(|| self.speed_kmh.round());
                        self.assistance.setpoint_kmh = Some((current + delta).clamp(30.0, 200.0));
                        if !self.assistance.active { self.activate(true); }
                    }
                }
            }
            Command::CancelAssistance => self.interrupt(InterruptionReason::DriverCancel),
            Command::ResumeAssistance => self.activate(true),
            Command::Pause => self.paused = !self.paused,
            Command::ResetTrip => self.trip_km = 0.0,
            Command::ResetAll => *self = Self::default(),
        }
    }

    fn set_gear(&mut self, gear: Gear) {
        // Virtual gearbox: reverse engages only near standstill. Never affects real vehicle.
        if (gear == Gear::Reverse || self.gear == Gear::Reverse) && self.speed_kmh > 2.0 { return; }
        if self.gear != gear {
            self.gear = gear;
            if gear == Gear::Neutral || gear == Gear::Reverse {
                self.interrupt(InterruptionReason::Neutral);
            } else if self.assistance.mode == AssistanceMode::Cruise {
                self.interrupt(InterruptionReason::Clutch);
            }
        }
    }

    fn lamp_index(lamp: Lamp) -> usize {
        match lamp {
            Lamp::Engine => 0, Lamp::Oil => 1, Lamp::Battery => 2, Lamp::Abs => 3,
            Lamp::Airbag => 4, Lamp::Brake => 5, Lamp::Seatbelt => 6,
            Lamp::Left => 7, Lamp::Right => 8, Lamp::Beam => 9, Lamp::Dipped => 10,
        }
    }

    pub fn lamp_active(&self, lamp: Lamp) -> bool {
        self.lamps[Self::lamp_index(lamp)]
            || (lamp == Lamp::Engine && self.scenario == Scenario::EngineFault)
            || (self.hazards && matches!(lamp, Lamp::Left | Lamp::Right))
    }

    pub fn indicator_lit(&self) -> bool {
        // 90 flashes per minute: ~333 ms illuminated, ~333 ms dark.
        self.elapsed_s.rem_euclid(2.0 / 3.0) < 1.0 / 3.0
    }

    pub fn step(&mut self, delta: Duration) {
        if self.paused { return; }
        let dt = delta.as_secs_f64().clamp(0.0, 0.2);
        if dt <= 0.0 { return; }
        self.elapsed_s += dt;
        // Continuous physical pedal input: fast press, spring return after key release.
        if self.keyboard_accelerator_owned {
            let target = if self.keyboard_accelerator_down { 100.0 } else { 0.0 };
            let rate = if self.keyboard_accelerator_down { 340.0 } else { 420.0 };
            self.accelerator_pct += (target-self.accelerator_pct).clamp(-rate*dt, rate*dt);
        }
        if self.keyboard_brake_owned {
            let target = if self.keyboard_brake_down { 100.0 } else { 0.0 };
            let rate = if self.keyboard_brake_down { 360.0 } else { 520.0 };
            self.brake_pct += (target-self.brake_pct).clamp(-rate*dt, rate*dt);
            if self.brake_pct > 2.0 && self.assistance.mode == AssistanceMode::Cruise { self.interrupt(InterruptionReason::Brake); }
        }

        let p = self.profile;
        let pedal_throttle = if self.engine_running { self.accelerator_pct / 100.0 } else { 0.0 };
        let drive_connected = self.engine_running && self.clutch_pct <= 2.0 && self.gear != Gear::Neutral;
        let coupled_rpm = p.rpm_at_speed(self.gear, self.speed_kmh);
        // Simulated low-speed clutch slip lets first gear move from standstill.
        let launch_slip_rpm = if self.speed_kmh < 9.0 { (1.0-self.speed_kmh/9.0) * pedal_throttle * 1600.0 } else { 0.0 };
        let engine_rpm = if !self.engine_running { 0.0 } else if drive_connected {
            coupled_rpm.max(760.0+launch_slip_rpm).clamp(760.0,6900.0)
        } else { 760.0 + self.accelerator_pct*35.0 };
        let max_traction_n = if drive_connected && coupled_rpm < 6750.0 { p.max_drive_force_n(self.gear, engine_rpm) } else { 0.0 };
        let resistance_n = p.resistance_n(self.speed_kmh, self.road_slope_pct);
        let mut throttle = if drive_connected { pedal_throttle } else { 0.0 };
        self.assistance.temporarily_overridden = false;
        self.assistance.unable_to_hold = false;
        if self.assistance.active {
            if self.assistance.mode == AssistanceMode::Cruise && !self.valid_control_conditions() {
                self.interrupt(if self.brake_pct > 2.0 { InterruptionReason::Brake }
                    else if self.clutch_pct > 2.0 { InterruptionReason::Clutch }
                    else { InterruptionReason::Fault });
            } else if !self.engine_running || matches!(self.gear, Gear::Neutral | Gear::Reverse) {
                self.interrupt(InterruptionReason::Neutral);
            }
        }
        if self.assistance.active {
            if let Some(target) = self.assistance.setpoint_kmh {
                let desired_accel = (target - self.speed_kmh) * 0.065; // m/s² per km/h error
                let requested_force = resistance_n + p.mass_kg * desired_accel;
                let regulated_throttle = if max_traction_n > 1.0 { (requested_force / max_traction_n).clamp(0.0,1.0) } else { 0.0 };
                match self.assistance.mode {
                    AssistanceMode::Cruise => {
                        if pedal_throttle > regulated_throttle + 0.035 { self.assistance.temporarily_overridden=true; throttle=pedal_throttle; }
                        else { throttle=regulated_throttle; }
                    }
                    AssistanceMode::Limiter => {
                        if self.accelerator_pct >= 98.0 { self.assistance.temporarily_overridden=true; }
                        else { throttle=throttle.min(regulated_throttle); }
                    }
                    AssistanceMode::Off => self.interrupt(InterruptionReason::DriverCancel),
                }
                self.assistance.unable_to_hold = self.speed_kmh > target + 2.0 && !self.assistance.temporarily_overridden;
            }
        }
        self.assistance.effective_throttle_pct = throttle * 100.0;
        let brake_force = (self.brake_pct / 100.0) * p.mass_kg * 9.5;
        let engine_brake_force = if drive_connected && pedal_throttle < 0.03 { 180.0 } else { 0.0 };
        let net_force = max_traction_n * throttle - resistance_n - brake_force - engine_brake_force;
        let acceleration = net_force / p.mass_kg;
        let old_speed = self.speed_kmh;
        let maximum = if self.gear == Gear::Reverse { 25.0 } else { 195.0 }; // conservative published upper-speed approximation
        self.speed_kmh = (self.speed_kmh + acceleration * dt * 3.6).clamp(0.0, maximum);
        let target_rpm = if !self.engine_running { 0.0 } else if !drive_connected { 760.0+self.accelerator_pct*35.0 }
            else { p.rpm_at_speed(self.gear,self.speed_kmh).max(760.0+launch_slip_rpm).clamp(760.0,6900.0) };
        self.rpm += (target_rpm - self.rpm) * (dt * 6.0).clamp(0.0, 1.0);
        self.rpm = self.rpm.clamp(0.0, 7000.0);
        let cruise_temp = if self.engine_running { 90.0 } else { 30.0 };
        self.coolant_c += (cruise_temp - self.coolant_c) * dt * 0.016;
        self.voltage += ((if self.engine_running { 14.1 } else { 12.3 }) - self.voltage) * dt;
        let distance = (old_speed + self.speed_kmh) * 0.5 * dt / 3600.0;
        self.trip_km += distance;
        self.odometer_km += distance;
        if self.engine_running {
            self.fuel_pct = (self.fuel_pct - dt * (0.0004 + throttle * 0.001)).max(0.0);
        }
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
        let status = if self.scenario == Scenario::LinkLost { TelemetryStatus::Disconnected }
            else if self.scenario == Scenario::SensorFailure { TelemetryStatus::Partial }
            else if self.paused { TelemetryStatus::Paused }
            else { TelemetryStatus::Simulated };
        let mut data = VehicleSnapshot {
            speed_kmh: Reading::simulated(self.speed_kmh),
            rpm: Reading::simulated(self.rpm), coolant_c: Reading::simulated(self.coolant_c),
            fuel_pct: Reading::simulated(self.fuel_pct), battery_v: Reading::simulated(self.voltage),
            trip_km: Reading::simulated(self.trip_km),
            odometer_km: Reading::simulated(self.odometer_km),
            gear: Reading::simulated(self.gear), engine_running: Reading::simulated(self.engine_running),
            assistance: Reading::simulated(self.assistance),
            shift_advice: Reading::simulated(shift_advice(self.gear, self.rpm, self.engine_running)),
            openings: Reading::simulated(self.openings),
            tyres: Reading::simulated(TyreMonitoring { wheels: std::array::from_fn(|i| {
                let pressure = self.tyre_pressure_bar[i];
                let reference = if i < 2 { 2.2 } else { 2.0 };
                // Test-only thresholds! Real wheel fault states come from the adapter.
                let alert = if self.tyre_sensor_fault[i] { TyreAlert::SensorFault }
                    else if pressure < reference-0.65 { TyreAlert::Critical }
                    else if pressure < reference-0.30 { TyreAlert::Underinflated }
                    else { TyreAlert::Normal };
                TyreReading { pressure_bar: if self.tyre_sensor_fault[i] { Reading::unknown() } else { Reading::simulated(pressure) }, alert }
            }) }),
            status, lights: self.lamp_states(), elapsed_simulation_s: self.elapsed_s,
        };
        if self.scenario == Scenario::SensorFailure {
            data.speed_kmh = Reading::unknown();
            data.rpm = Reading::unknown();
            data.coolant_c = Reading::unknown();
            data.shift_advice = Reading::unknown();
            data.assistance = Reading::unknown();
            data.tyres = Reading::unknown();
            data.openings = Reading::unknown();
        } else if self.scenario == Scenario::LinkLost {
            data.speed_kmh.quality = DataQuality::Stale;
            data.rpm.quality = DataQuality::Stale;
            data.coolant_c.quality = DataQuality::Stale;
            data.fuel_pct.quality = DataQuality::Stale;
            data.battery_v.quality = DataQuality::Stale;
            data.trip_km.quality = DataQuality::Stale;
            data.odometer_km.quality = DataQuality::Stale;
            data.gear = Reading::unknown();
            data.engine_running = Reading::unknown();
            data.assistance = Reading::unknown();
            data.shift_advice = Reading::unknown();
            data.lights = WarningLights::unknown();
            data.tyres = Reading::unknown();
            data.openings = Reading::unknown();
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(sim: &mut Simulator, seconds: u32) {
        for _ in 0..seconds * 10 { sim.step(Duration::from_millis(100)); }
    }

    fn moving(speed: f64) -> Simulator {
        Simulator { speed_kmh: speed, gear: Gear::Fourth, accelerator_pct: 0.0, ..Simulator::default() }
    }

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
    fn reverse_requires_near_stop() {
        let mut sim = Simulator::default();
        sim.apply(Command::SetGear(Gear::Reverse));
        assert_eq!(sim.gear, Gear::Third);
        sim.speed_kmh = 0.0;
        sim.apply(Command::SetGear(Gear::Reverse));
        assert_eq!(sim.gear, Gear::Reverse);
        sim.apply(Command::Accelerator(100.0));
        run(&mut sim, 10);
        assert!(sim.speed_kmh <= 25.0);
    }

    #[test]
    fn cruise_actually_maintains_speed_without_driver_throttle() {
        let mut sim = moving(70.0);
        sim.apply(Command::SetAssistance(AssistanceMode::Cruise));
        sim.apply(Command::SetAssistanceSpeed);
        assert!(sim.assistance.active);
        run(&mut sim, 20);
        assert!((sim.speed_kmh - 70.0).abs() < 2.0, "speed {}", sim.speed_kmh);
        assert!(sim.assistance.effective_throttle_pct > 0.0);
        assert_eq!(sim.accelerator_pct, 0.0); // input is not overwritten by controller
    }

    #[test]
    fn braking_cancels_cruise_but_preserves_resume_setpoint() {
        let mut sim = moving(70.0);
        sim.apply(Command::SetAssistance(AssistanceMode::Cruise));
        sim.apply(Command::SetAssistanceSpeed);
        sim.apply(Command::Brake(25.0));
        assert!(!sim.assistance.active);
        assert_eq!(sim.assistance.interrupted_by, InterruptionReason::Brake);
        assert_eq!(sim.assistance.setpoint_kmh, Some(70.0));
        sim.apply(Command::ResumeAssistance);
        assert!(!sim.assistance.active); // must release pedal
        sim.apply(Command::Brake(0.0));
        sim.apply(Command::ResumeAssistance);
        assert!(sim.assistance.active);
    }

    #[test]
    fn clutch_or_manual_shift_interrupts_cruise() {
        let mut sim = moving(70.0);
        sim.apply(Command::SetAssistance(AssistanceMode::Cruise));
        sim.apply(Command::SetAssistanceSpeed);
        sim.apply(Command::Clutch(15.0));
        assert!(!sim.assistance.active);
        sim.apply(Command::Clutch(0.0));
        sim.apply(Command::ResumeAssistance);
        sim.apply(Command::SetGear(Gear::Fifth));
        assert!(!sim.assistance.active);
    }

    #[test]
    fn limiter_caps_pedal_without_braking_or_disabling_on_brake() {
        let mut sim = moving(70.0);
        sim.apply(Command::SetAssistance(AssistanceMode::Limiter));
        sim.apply(Command::SetAssistanceSpeed);
        sim.apply(Command::Accelerator(75.0));
        run(&mut sim, 15);
        assert!(sim.speed_kmh < 72.0, "speed {}", sim.speed_kmh);
        assert!(sim.assistance.effective_throttle_pct < sim.accelerator_pct);
        sim.apply(Command::Brake(20.0));
        assert!(sim.assistance.active);
    }

    #[test]
    fn limiter_kickdown_bypasses_cap_then_returns() {
        let mut sim = moving(70.0);
        sim.apply(Command::SetAssistance(AssistanceMode::Limiter));
        sim.apply(Command::SetAssistanceSpeed);
        sim.apply(Command::Accelerator(100.0));
        run(&mut sim, 2);
        assert!(sim.speed_kmh > 70.0);
        assert!(sim.assistance.temporarily_overridden);
        sim.apply(Command::Accelerator(55.0));
        run(&mut sim, 2);
        assert!(!sim.assistance.temporarily_overridden);
        assert!(sim.assistance.active);
    }

    #[test]
    fn downhill_can_exceed_setpoint_without_automatic_braking() {
        let mut sim = moving(70.0);
        sim.apply(Command::RoadSlope(-12.0));
        sim.apply(Command::SetAssistance(AssistanceMode::Limiter));
        sim.apply(Command::SetAssistanceSpeed);
        run(&mut sim, 20);
        assert!(sim.speed_kmh > 72.0, "speed {}", sim.speed_kmh);
        assert!(sim.assistance.unable_to_hold);
        assert_eq!(sim.assistance.effective_throttle_pct, 0.0);
    }

    #[test]
    fn invalid_control_arming_and_neutral_disarm() {
        let mut sim = moving(20.0);
        sim.apply(Command::SetAssistance(AssistanceMode::Cruise));
        sim.apply(Command::SetAssistanceSpeed);
        assert!(!sim.assistance.active);
        sim.speed_kmh = 70.0;
        sim.apply(Command::SetAssistanceSpeed);
        assert!(sim.assistance.active);
        sim.apply(Command::SetGear(Gear::Neutral));
        assert!(!sim.assistance.active);
        assert_eq!(sim.assistance.interrupted_by, InterruptionReason::Neutral);
    }

    #[test]
    fn disabled_driver_pedal_does_not_activate_a_real_car() {
        let mut sim = moving(70.0);
        sim.apply(Command::ToggleEngine);
        sim.apply(Command::Accelerator(100.0));
        run(&mut sim, 2);
        assert_eq!(sim.assistance.effective_throttle_pct, 0.0);
    }

    #[test]
    fn indicators_blink_and_hazards_flash_together() {
        let mut sim = Simulator::default();
        sim.apply(Command::ToggleLamp(Lamp::Left));
        assert_eq!(sim.snapshot().lights.left_indicator, LampState::On);
        sim.step(Duration::from_millis(200));
        sim.step(Duration::from_millis(200));
        assert_eq!(sim.snapshot().lights.left_indicator, LampState::Off);
        sim.apply(Command::ToggleHazards);
        assert!(sim.lamp_active(Lamp::Left) && sim.lamp_active(Lamp::Right));
    }

    #[test]
    fn simulator_publishes_raw_state_including_shift_advice() {
        let mut sim = Simulator::default();
        sim.rpm = 3500.0;
        assert_eq!(sim.snapshot().shift_advice.usable(), Some(&ShiftAdvice::Up));
        sim.apply(Command::SetScenario(Scenario::LinkLost));
        assert!(sim.snapshot().speed_kmh.usable().is_none());
        assert!(sim.snapshot().gear.usable().is_none());
        assert!(sim.snapshot().assistance.usable().is_none());
        assert_eq!(sim.snapshot().lights.abs, LampState::Unknown);
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
    fn keyboard_pedals_accelerate_then_spring_back() {
        let mut sim=Simulator::default();
        sim.apply(Command::HoldAccelerator(true));
        run(&mut sim, 1);
        assert!(sim.accelerator_pct > 99.0);
        sim.apply(Command::HoldAccelerator(false));
        run(&mut sim, 1);
        assert_eq!(sim.accelerator_pct, 0.0);
        sim.apply(Command::HoldBrake(true));
        run(&mut sim, 1);
        assert_eq!(sim.brake_pct, 100.0);
        sim.apply(Command::HoldBrake(false));
        run(&mut sim, 1);
        assert_eq!(sim.brake_pct, 0.0);
    }
    #[test]
    fn door_and_tyre_states_publish_and_fail_closed() {
        let mut sim=Simulator::default();
        sim.apply(Command::ToggleOpening(Aperture::RearRight));
        sim.apply(Command::SetTyrePressure(Wheel::FrontLeft, 1.3));
        let snap=sim.snapshot();
        assert!(snap.openings.usable().unwrap().is_open(Aperture::RearRight));
        assert_eq!(snap.tyres.usable().unwrap().wheels[0].alert,TyreAlert::Critical);
        sim.apply(Command::ToggleTyreSensor(Wheel::FrontLeft));
        assert_eq!(sim.snapshot().tyres.usable().unwrap().wheels[0].alert,TyreAlert::SensorFault);
        sim.apply(Command::SetScenario(Scenario::LinkLost));
        assert!(sim.snapshot().openings.usable().is_none());
        assert!(sim.snapshot().tyres.usable().is_none());
    }
    #[test]
    fn gear_ratio_changes_acceleration_and_rpm() {
        let mut low=moving(50.0); let mut high=low.clone();
        low.apply(Command::SetGear(Gear::Second)); high.apply(Command::SetGear(Gear::Fifth));
        low.apply(Command::Accelerator(65.0));high.apply(Command::Accelerator(65.0));
        run(&mut low,2);run(&mut high,2);
        assert!(low.speed_kmh>high.speed_kmh, "2nd={} 5th={}",low.speed_kmh,high.speed_kmh);
        assert!(low.rpm>high.rpm);
    }

    #[test]
    fn simulation_values_stay_bounded() {
        let mut sim = Simulator::default();
        sim.apply(Command::Accelerator(999.0));
        sim.apply(Command::Brake(-2.0));
        sim.apply(Command::RoadSlope(-999.0));
        run(&mut sim, 200);
        assert!((0.0..=195.0).contains(&sim.speed_kmh));
        assert!((0.0..=100.0).contains(&sim.fuel_pct));
        assert!((0.0..=7000.0).contains(&sim.rpm));
        assert_eq!(sim.road_slope_pct, -12.0);
    }
}
