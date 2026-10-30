//! Domain model shared by every telemetry source and renderer.
//! The simulated odometer MUST NEVER be treated as a vehicle's legal mileage.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataQuality {
    /// A value from the simulator, never a measured vehicle value.
    Simulated,
    /// The input is not known: do not substitute a nominal reading.
    Unknown,
    /// Previously known reading whose update deadline has passed.
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Reading<T> {
    pub value: Option<T>,
    pub quality: DataQuality,
}

impl<T> Reading<T> {
    pub fn simulated(value: T) -> Self {
        Self { value: Some(value), quality: DataQuality::Simulated }
    }

    pub fn unknown() -> Self {
        Self { value: None, quality: DataQuality::Unknown }
    }

    pub fn stale(value: T) -> Self {
        Self { value: Some(value), quality: DataQuality::Stale }
    }

    pub fn usable(&self) -> Option<&T> {
        if self.quality == DataQuality::Simulated { self.value.as_ref() } else { None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LampState { On, Off, Unknown }

impl LampState {
    pub fn from_bool(active: bool) -> Self { if active { Self::On } else { Self::Off } }
    pub fn is_on(self) -> bool { self == Self::On }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gear { Reverse, Neutral, First, Second, Third, Fourth, Fifth }

impl Gear {
    pub fn label(self) -> &'static str {
        match self {
            Self::Reverse => "R", Self::Neutral => "N", Self::First => "1",
            Self::Second => "2", Self::Third => "3", Self::Fourth => "4", Self::Fifth => "5",
        }
    }

    pub fn higher(self) -> Self {
        match self {
            Self::Reverse | Self::Neutral => Self::First,
            Self::First => Self::Second, Self::Second => Self::Third,
            Self::Third => Self::Fourth, Self::Fourth => Self::Fifth, Self::Fifth => Self::Fifth,
        }
    }

    pub fn lower(self) -> Self {
        match self {
            Self::Fifth => Self::Fourth, Self::Fourth => Self::Third,
            Self::Third => Self::Second, Self::Second => Self::First,
            Self::First => Self::Neutral, Self::Neutral => Self::Reverse, Self::Reverse => Self::Reverse,
        }
    }

    pub fn rpm_per_kmh(self) -> f64 {
        match self {
            Self::First => 135.0, Self::Second => 81.0, Self::Third => 53.0,
            Self::Fourth => 39.0, Self::Fifth => 31.0, Self::Reverse => 130.0, Self::Neutral => 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarningLights {
    pub engine: LampState,
    pub oil_pressure: LampState,
    pub battery: LampState,
    pub abs: LampState,
    pub airbag: LampState,
    pub brake: LampState,
    pub seatbelt: LampState,
    pub left_indicator: LampState,
    pub right_indicator: LampState,
    pub main_beam: LampState,
    pub dipped_beam: LampState,
}

impl WarningLights {
    pub fn unknown() -> Self {
        Self {
            engine: LampState::Unknown, oil_pressure: LampState::Unknown,
            battery: LampState::Unknown, abs: LampState::Unknown,
            airbag: LampState::Unknown, brake: LampState::Unknown,
            seatbelt: LampState::Unknown, left_indicator: LampState::Unknown,
            right_indicator: LampState::Unknown, main_beam: LampState::Unknown,
            dipped_beam: LampState::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistanceMode { Off, Cruise, Limiter }

impl AssistanceMode {
    pub fn label(self) -> &'static str {
        match self { Self::Off => "OFF", Self::Cruise => "RÉGULATEUR", Self::Limiter => "LIMITEUR" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpeedAssistance {
    pub mode: AssistanceMode,
    /// Setpoint is a simulation value, not an ECU control state.
    pub setpoint_kmh: Option<f64>,
    pub active: bool,
    pub temporarily_overridden: bool,
}

impl Default for SpeedAssistance {
    fn default() -> Self {
        Self { mode: AssistanceMode::Off, setpoint_kmh: None, active: false, temporarily_overridden: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftAdvice { None, Up, Down }

/// Illustrative eco shift cue: never a manufacturer-approved driving recommendation.
pub fn suggested_shift(gear: Gear, rpm: Option<f64>, running: bool) -> ShiftAdvice {
    if !running { return ShiftAdvice::None; }
    let Some(rpm) = rpm else { return ShiftAdvice::None; };
    if !rpm.is_finite() { return ShiftAdvice::None; }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleSnapshot {
    pub speed_kmh: Reading<f64>,
    pub rpm: Reading<f64>,
    pub coolant_c: Reading<f64>,
    pub fuel_pct: Reading<f64>,
    pub battery_v: Reading<f64>,
    pub trip_km: Reading<f64>,
    /// Fictional simulator mileage, not the odometer read from a Renault ECU.
    pub simulated_odometer_km: Reading<f64>,
    pub gear: Gear,
    pub engine_running: bool,
    pub assistance: SpeedAssistance,
    pub lights: WarningLights,
    pub elapsed_simulation_s: f64,
}

impl Default for VehicleSnapshot {
    fn default() -> Self {
        Self {
            speed_kmh: Reading::unknown(), rpm: Reading::unknown(),
            coolant_c: Reading::unknown(), fuel_pct: Reading::unknown(),
            battery_v: Reading::unknown(), trip_km: Reading::unknown(),
            simulated_odometer_km: Reading::unknown(), gear: Gear::Neutral,
            engine_running: false, assistance: SpeedAssistance::default(),
            lights: WarningLights::unknown(), elapsed_simulation_s: 0.0,
        }
    }
}

/// Future OBD/CAN adapters implement the same read-only interface.
/// No actuation/write methods are deliberately exposed.
pub trait TelemetrySource {
    fn snapshot(&self) -> VehicleSnapshot;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_values_cannot_be_displayed_as_valid() {
        assert_eq!(Reading::<f64>::unknown().usable(), None);
        assert_eq!(Reading::stale(80.0).usable(), None);
        assert_eq!(Reading::simulated(80.0).usable(), Some(&80.0));
    }

    #[test]
    fn lamps_do_not_default_to_safe() {
        assert_eq!(VehicleSnapshot::default().lights.airbag, LampState::Unknown);
        assert_eq!(VehicleSnapshot::default().lights.brake, LampState::Unknown);
    }

    #[test]
    fn shift_advice_only_for_valid_forward_gears() {
        assert_eq!(suggested_shift(Gear::Third, Some(3500.0), true), ShiftAdvice::Up);
        assert_eq!(suggested_shift(Gear::Third, Some(1050.0), true), ShiftAdvice::Down);
        assert_eq!(suggested_shift(Gear::Neutral, Some(4000.0), true), ShiftAdvice::None);
        assert_eq!(suggested_shift(Gear::Fifth, None, true), ShiftAdvice::None);
        assert_eq!(suggested_shift(Gear::Second, Some(5000.0), false), ShiftAdvice::None);
    }

    #[test]
    fn gear_limits_are_respected() {
        assert_eq!(Gear::Fifth.higher(), Gear::Fifth);
        assert_eq!(Gear::Neutral.lower(), Gear::Reverse);
    }
}
