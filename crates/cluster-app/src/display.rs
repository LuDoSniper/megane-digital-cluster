//! Presentation-only cluster: consumes a telemetry snapshot, never simulator commands.
//! All control policies and shift cues are produced upstream by the telemetry source.

use cluster_core::{
    AssistanceMode, InterruptionReason, LampState, Reading, ShiftAdvice,
    TelemetryStatus, VehicleSnapshot,
};
use dioxus::prelude::*;
use std::f64::consts::PI;

fn reading(value: &Reading<f64>, digits: usize) -> String {
    value.usable().map(|v| format!("{v:.digits$}")).unwrap_or_else(|| "—".to_string())
}

fn odometer_label(value: &Reading<f64>) -> String {
    let Some(km) = value.usable() else { return "—".to_string(); };
    if !km.is_finite() || *km < 0.0 { return "—".to_string(); }
    let digits = format!("{:.0}", km);
    let mut out = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i != 0 && i % 3 == 0 { out.push(' '); }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Screen coordinates; the gauge arc runs bottom-left → top → bottom-right.
fn polar(radius: f64, angle_deg: f64) -> (f64, f64) {
    let a = angle_deg * PI / 180.0;
    (160.0 + radius * a.cos(), 160.0 + radius * a.sin())
}

fn arc_path(radius: f64, start: f64, end: f64) -> String {
    let (sx, sy) = polar(radius, start);
    let (ex, ey) = polar(radius, end);
    let sweep = end - start;
    let large = if sweep > 180.0 { 1 } else { 0 };
    format!("M {sx:.2} {sy:.2} A {radius} {radius} 0 {large} 1 {ex:.2} {ey:.2}")
}

struct GaugeTick {
    x1: f64, y1: f64, x2: f64, y2: f64, lx: f64, ly: f64,
    major: bool, label: String,
}

fn ticks(max_value: usize, step: usize, major_every: usize, rpm: bool) -> Vec<GaugeTick> {
    let count = max_value / step;
    (0..=count).map(|i| {
        let angle = 135.0 + i as f64 / count as f64 * 270.0;
        let major = i % major_every == 0;
        let (x1, y1) = polar(122.0, angle);
        let (x2, y2) = polar(if major { 108.0 } else { 115.0 }, angle);
        let (lx, ly) = polar(88.0, angle);
        let value = i * step;
        GaugeTick {
            x1, y1, x2, y2, lx, ly, major,
            label: if rpm { format!("{}", value / 1000) } else { value.to_string() },
        }
    }).collect()
}

#[component]
fn Gauge(title: &'static str, display_value: String, raw_value: Option<f64>,
    maximum: usize, step: usize, major_every: usize, unit: &'static str, kind: &'static str) -> Element {
    let percentage = raw_value.map(|v| (v / maximum as f64).clamp(0.0, 1.0)).unwrap_or(0.0);
    let active_arc = arc_path(132.0, 135.0, 135.0 + (270.0 * percentage).max(0.01));
    let track = arc_path(132.0, 135.0, 405.0);
    let marks = ticks(maximum, step, major_every, kind == "rpm");
    rsx! {
        div { class: "cluster-dial gauge-{kind}",
            div { class: "gauge-stage",
                svg { class: "gauge-svg", view_box: "0 0 320 320", role: "img", aria_label: "Graduations de {title}",
                    circle { cx: "160", cy: "160", r: "150", class: "gauge-outer" }
                    path { d: "{track}", class: "gauge-rail" }
                    if raw_value.is_some() {
                        path { d: "{active_arc}", class: "gauge-active" }
                    }
                    for tick in marks {
                        line { x1: "{tick.x1}", y1: "{tick.y1}", x2: "{tick.x2}", y2: "{tick.y2}",
                            class: if tick.major { "major-tick" } else { "minor-tick" } }
                        if tick.major {
                            text { x: "{tick.lx}", y: "{tick.ly}", class: "gauge-number", "text-anchor": "middle", "dominant-baseline": "central", "{tick.label}" }
                        }
                    }
                }
                div { class: "gauge-readout",
                    span { "{title}" }
                    strong { "{display_value}" }
                    small { "{unit}" }
                }
            }
        }
    }
}

#[component]
pub fn Instrument(data: VehicleSnapshot) -> Element {
    let speed = reading(&data.speed_kmh, 0);
    let rpm_value = data.rpm.usable().copied();
    let rpm = rpm_value.map(|v| format!("{:.1}", v / 1000.0)).unwrap_or_else(|| "—".to_owned());
    let fuel = reading(&data.fuel_pct, 0);
    let coolant = reading(&data.coolant_c, 0);
    let trip = reading(&data.trip_km, 1);
    let mileage = odometer_label(&data.odometer_km);
    let odometer_source = match data.odometer_km.quality {
        cluster_core::DataQuality::Simulated => "ODOMÈTRE · FICTIF",
        cluster_core::DataQuality::Measured => "ODOMÈTRE · MESURÉ",
        cluster_core::DataQuality::Unknown | cluster_core::DataQuality::Stale => "ODOMÈTRE · INDISPONIBLE",
    };
    let fuel_pct = data.fuel_pct.usable().copied().unwrap_or(0.0).clamp(0.0, 100.0);
    let coolant_pct = data.coolant_c.usable().map(|v| ((v - 40.0) / 80.0 * 100.0).clamp(0.0, 100.0)).unwrap_or(0.0);
    let gear = data.gear.usable().map(|g| g.label()).unwrap_or("—");
    let shift = data.shift_advice.usable().copied();
    let shift_label = match shift {
        Some(ShiftAdvice::Up) => "↑ MONTER UN RAPPORT",
        Some(ShiftAdvice::Down) => "↓ RÉTROGRADER",
        Some(ShiftAdvice::None) => "RAPPORT ADAPTÉ",
        None => "CONSEIL INDISPONIBLE",
    };
    let assist = data.assistance.usable().copied();
    let assist_mode = match assist.map(|a| a.mode) {
        Some(AssistanceMode::Cruise) => "RÉGULATEUR",
        Some(AssistanceMode::Limiter) => "LIMITEUR",
        Some(AssistanceMode::Off) => "ASSISTANCE ARRÊTÉE",
        None => "ASSISTANCE INDISPONIBLE",
    };
    let assist_status = match assist {
        Some(a) if a.unable_to_hold => "CONSIGNE DÉPASSÉE · DESCENTE",
        Some(a) if a.temporarily_overridden => "DÉPASSEMENT TEMPORAIRE",
        Some(a) if a.active => "EN SERVICE",
        Some(a) => match a.interrupted_by {
            InterruptionReason::Brake => "EN VEILLE · FREIN",
            InterruptionReason::Clutch => "EN VEILLE · EMBRAYAGE",
            InterruptionReason::Neutral => "EN VEILLE · POINT MORT",
            InterruptionReason::EngineStopped => "EN VEILLE · MOTEUR",
            InterruptionReason::Fault => "INDISPONIBLE · DÉFAUT",
            InterruptionReason::DriverCancel => "EN VEILLE · CONDUCTEUR",
            InterruptionReason::None => "PRÊT À ACTIVER",
        },
        None => "ÉTAT INCONNU",
    };
    let setpoint = assist.and_then(|a| a.setpoint_kmh).map(|v| format!("{v:.0} km/h")).unwrap_or_else(|| "— km/h".to_string());
    let assist_active = assist.is_some_and(|a| a.active);
    let status = match data.status {
        TelemetryStatus::Simulated => "SIMULATION",
        TelemetryStatus::Paused => "SIMULATION EN PAUSE",
        TelemetryStatus::Partial => "CAPTEURS INDISPONIBLES",
        TelemetryStatus::Disconnected => "LIAISON PERDUE",
        TelemetryStatus::Measured => "TÉLÉMÉTRIE",
    };
    let degraded = matches!(data.status, TelemetryStatus::Partial | TelemetryStatus::Disconnected);
    let critical = data.lights.oil_pressure == LampState::On || data.lights.brake == LampState::On;
    let warning = if degraded { "DONNÉES INDISPONIBLES" }
        else if data.lights.oil_pressure == LampState::On { "PRESSION D'HUILE · ALERTE" }
        else if data.lights.brake == LampState::On { "FREINAGE · ALERTE" }
        else if data.lights.engine == LampState::On { "MOTEUR · CONTRÔLE REQUIS" }
        else { "PARAMÈTRES DE CONDUITE" };
    let warning_class = if critical || degraded { "center-alert urgent" }
        else if data.lights.engine == LampState::On { "center-alert attention" }
        else { "center-alert" };

    rsx! {
        section { class: "cluster-bezel", aria_label: "Prototype de combiné d'instruments numérique",
            div { class: "cluster-glass",
                div { class: "cluster-vignette" }
                div { class: "cluster-head",
                    Telltale { name: "left", state: data.lights.left_indicator, tone: "green", prominent: true }
                    div { class: "cluster-head-brand",
                        span { class: "diamond", "◇" }
                        span { "MÉGANE" }
                        small { "1.6 16V" }
                    }
                    div { class: "cluster-run-status",
                        span { class: if degraded { "run-bad" } else { "run-good" }, "●" }
                        "{status}"
                    }
                    Telltale { name: "right", state: data.lights.right_indicator, tone: "green", prominent: true }
                }
                div { class: "cluster-instruments",
                    Gauge { title: "VITESSE", display_value: speed, raw_value: data.speed_kmh.usable().copied(),
                        maximum: 240, step: 10, major_every: 4, unit: "km/h", kind: "speed" }
                    div { class: "cluster-primary",
                        div { class: "center-label", "CENTRE D'INFORMATION" }
                        div { class: "{warning_class}",
                            span { class: "center-alert-icon", if critical || degraded { "!" } else { "◆" } }
                            span { "{warning}" }
                        }
                        div { class: "center-driving",
                            div { class: "center-assist-header",
                                span { "{assist_mode}" }
                                span { class: if assist_active { "assist-live" } else { "assist-standby" }, "{assist_status}" }
                            }
                            strong { class: "center-assist-value", "{setpoint}" }
                            div { class: "center-assist-caption", "CONSIGNE DE VITESSE" }
                        }
                        div { class: "center-lower",
                            div { class: "compact-gear", span { "RAPPORT" } strong { "{gear}" } }
                            div { class: "compact-shift", "{shift_label}" }
                        }
                    }
                    Gauge { title: "COMPTE-TOURS", display_value: rpm, raw_value: rpm_value,
                        maximum: 7000, step: 250, major_every: 4, unit: "× 1 000 tr/min", kind: "rpm" }
                }
                div { class: "cluster-lamps", aria_label: "Témoins du véhicule (simulés)",
                    Telltale { name: "dipped", state: data.lights.dipped_beam, tone: "green" }
                    Telltale { name: "beam", state: data.lights.main_beam, tone: "blue" }
                    Telltale { name: "engine", state: data.lights.engine, tone: "amber" }
                    Telltale { name: "oil", state: data.lights.oil_pressure, tone: "red" }
                    Telltale { name: "battery", state: data.lights.battery, tone: "red" }
                    Telltale { name: "abs", state: data.lights.abs, tone: "amber" }
                    Telltale { name: "airbag", state: data.lights.airbag, tone: "red" }
                    Telltale { name: "brake", state: data.lights.brake, tone: "red" }
                    Telltale { name: "belt", state: data.lights.seatbelt, tone: "red" }
                }
                div { class: "cluster-bottom",
                    div { class: "reservoir",
                        div { class: "reservoir-label", span { "CARBURANT" } strong { "{fuel}%" } }
                        div { class: "cluster-track", div { style: "width: {fuel_pct}%;" } }
                        div { class: "reservoir-extremes", span { "E" } span { "F" } }
                    }
                    div { class: "distance-group",
                        div { class: "cluster-mileage",
                            small { "{odometer_source}" }
                            strong { "{mileage} km" }
                        }
                        div { class: "cluster-trip", small { "TRAJET PARTIEL" } strong { "{trip} km" } }
                    }
                    div { class: "reservoir",
                        div { class: "reservoir-label", span { "TEMPÉRATURE MOTEUR" } strong { "{coolant} °C" } }
                        div { class: "cluster-track coolant", div { style: "width: {coolant_pct}%;" } }
                        div { class: "reservoir-extremes", span { "C" } span { "H" } }
                    }
                }
                div { class: "sim-watermark", "PROTOTYPE · AUCUNE CONNEXION À LA VOITURE · NE PAS UTILISER POUR CONDUIRE" }
            }
        }
    }
}

#[component]
fn Telltale(name: &'static str, state: LampState, tone: &'static str, #[props(default = false)] prominent: bool) -> Element {
    let class = if prominent { "telltale prominent" } else { "telltale" };
    let state_class = match state {
        LampState::On => "lit", LampState::Off => "dark", LampState::Unknown => "unknown",
    };
    rsx! {
        div { class: "{class} {tone} {state_class}", title: "{name}",
            svg { view_box: "0 0 48 48", width: "34", height: "34", fill: "none", stroke: "currentColor", "stroke-width": "2.7", "stroke-linecap": "round", "stroke-linejoin": "round",
                if name == "left" { path { d: "M37 15 H21 V8 L6 24 L21 40 V32 H37 Z", fill: "currentColor", stroke: "none" } }
                if name == "right" { path { d: "M11 15 H27 V8 L42 24 L27 40 V32 H11 Z", fill: "currentColor", stroke: "none" } }
                if name == "engine" {
                    path { d: "M10 16 H17 V11 H30 V16 H37 V21 H41 V33 H36 L32 38 H17 L13 33 H7 V21 H10 Z" }
                    path { d: "M23 11 V7 M18 7 H29 M18 22 H30" }
                }
                if name == "oil" {
                    path { d: "M8 20 H30 L39 28 V34 H16 L8 27 Z" }
                    path { d: "M13 15 L21 20 M29 15 L23 20 M38 15 L43 20" }
                    circle { cx: "40", cy: "38", r: "2", fill: "currentColor" }
                }
                if name == "battery" {
                    rect { x: "7", y: "15", width: "34", height: "24", rx: "2" }
                    path { d: "M14 15 V9 H21 V15 M28 15 V9 H35 V15 M13 27 H23 M18 22 V32 M29 27 H38" }
                }
                if name == "abs" {
                    circle { cx: "24", cy: "24", r: "15" }
                    path { d: "M8 11 Q0 24 8 37 M40 11 Q48 24 40 37" }
                    text { x: "24", y: "29", "font-size": "12", "font-weight": "800", "text-anchor": "middle", stroke: "none", fill: "currentColor", "ABS" }
                }
                if name == "brake" {
                    circle { cx: "24", cy: "24", r: "15" }
                    path { d: "M7 10 Q-2 24 7 38 M41 10 Q50 24 41 38 M24 15 V28" }
                    circle { cx: "24", cy: "34", r: "1.7", fill: "currentColor" }
                }
                if name == "airbag" {
                    circle { cx: "14", cy: "13", r: "5" }
                    path { d: "M12 20 L19 30 H30 M8 21 V34 H26 M17 34 H34" }
                    circle { cx: "33", cy: "18", r: "9" }
                }
                if name == "belt" {
                    circle { cx: "19", cy: "11", r: "5" }
                    path { d: "M16 20 L12 34 H32 M15 20 L30 32 M28 16 L36 34" }
                }
                if name == "beam" || name == "dipped" {
                    path { d: "M13 12 Q2 24 13 36 H26 V12 Z" }
                    if name == "beam" {
                        path { d: "M29 14 H44 M29 21 H44 M29 28 H44 M29 35 H44" }
                    } else {
                        path { d: "M29 14 L43 19 M29 22 L43 27 M29 30 L43 35" }
                    }
                }
            }
            if state == LampState::Unknown { span { class: "unknown-mark", "?" } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_labels_are_actually_on_the_dial() {
        let speed = ticks(240, 10, 4, false);
        let rpm = ticks(7000, 250, 4, true);
        assert_eq!(speed.len(), 25);
        assert_eq!(rpm.len(), 29);
        assert_eq!(speed.iter().filter(|t| t.major).count(), 7);
        assert_eq!(rpm.iter().filter(|t| t.major).count(), 8);
        assert_eq!(speed.last().unwrap().label, "240");
        assert_eq!(rpm.last().unwrap().label, "7");
        assert!(speed.iter().any(|t| (t.lx - t.x1).abs() > 1.0));
    }

    #[test]
    fn measured_odometer_is_formatted_and_unknown_is_not_invented() {
        assert_eq!(odometer_label(&Reading::simulated(182_450.0)), "182 450");
        assert_eq!(odometer_label(&Reading::unknown()), "—");
        assert_eq!(odometer_label(&Reading::stale(182_450.0)), "—");
    }
}
