//! Display-only, future-proof presentation component: no simulator commands.
//! Pictograms are vector prototypes inspired by ISO 2575 conventions, not certified artwork.

use cluster_core::{suggested_shift, AssistanceMode, DataQuality, LampState, ShiftAdvice, VehicleSnapshot};
use dioxus::prelude::*;

fn reading(value: &cluster_core::Reading<f64>, precision: usize) -> String {
    value.usable().map(|v| format!("{v:.precision$}")).unwrap_or_else(|| "—".to_string())
}

#[component]
pub fn Instrument(data: VehicleSnapshot, status: String) -> Element {
    let speed = reading(&data.speed_kmh, 0);
    let rpm = reading(&data.rpm, 0);
    let fuel = reading(&data.fuel_pct, 0);
    let coolant = reading(&data.coolant_c, 0);
    let trip = reading(&data.trip_km, 1);
    let mileage = reading(&data.simulated_odometer_km, 0);
    let rpm_pct = data.rpm.usable().map(|v| (v / 7000.0 * 100.0).clamp(0.0, 100.0)).unwrap_or(0.0);
    let speed_pct = data.speed_kmh.usable().map(|v| (v / 240.0 * 100.0).clamp(0.0, 100.0)).unwrap_or(0.0);
    let fuel_pct = data.fuel_pct.usable().copied().unwrap_or(0.0).clamp(0.0, 100.0);
    let coolant_pct = data.coolant_c.usable().map(|v| ((v - 40.0) / 80.0 * 100.0).clamp(0.0, 100.0)).unwrap_or(0.0);
    let shift = suggested_shift(data.gear, data.rpm.usable().copied(), data.engine_running);
    let shift_label = match shift {
        ShiftAdvice::None => "", ShiftAdvice::Up => "↑ PASSER LE RAPPORT SUPÉRIEUR",
        ShiftAdvice::Down => "↓ RÉTROGRADER",
    };
    let assist = data.assistance;
    let assist_label = match assist.mode {
        AssistanceMode::Off => "ASSISTANCE DÉSACTIVÉE",
        AssistanceMode::Cruise => "RÉGULATEUR",
        AssistanceMode::Limiter => "LIMITEUR",
    };
    let assist_status = if assist.temporarily_overridden { "DÉPASSEMENT" }
        else if assist.active { "ACTIF" }
        else if assist.mode != AssistanceMode::Off { "EN ATTENTE" }
        else { "" };
    let assist_speed = assist.setpoint_kmh.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".to_string());
    let unavailable = data.speed_kmh.quality != DataQuality::Simulated;
    let critical = data.lights.oil_pressure == LampState::On || data.lights.brake == LampState::On;
    let invalid = data.lights.engine == LampState::Unknown;

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
                        span { class: if unavailable { "run-bad" } else { "run-good" }, "●" }
                        "{status}"
                    }
                    Telltale { name: "right", state: data.lights.right_indicator, tone: "green", prominent: true }
                }

                div { class: "cluster-instruments",
                    div { class: "cluster-dial tach-dial", style: "--value: {rpm_pct}%;",
                        div { class: "dial-ring",
                            div { class: "dial-core",
                                small { "RÉGIME MOTEUR" }
                                strong { "{rpm}" }
                                span { "tr/min" }
                            }
                        }
                        div { class: "dial-scale", span { "0" } span { "1" } span { "2" } span { "3" } span { "4" } span { "5" } span { "6" } span { "7" } }
                        span { class: "dial-foot", "x 1000 tr/min" }
                    }
                    div { class: "cluster-primary",
                        div { class: "cluster-speed-heading", "VITESSE INSTANTANÉE" }
                        div { class: if unavailable { "cluster-speed unavailable" } else { "cluster-speed" }, "{speed}" }
                        div { class: "cluster-speed-unit", "km/h" }
                        div { class: "cluster-drive-card",
                            div { class: "gear-box", span { "RAPPORT" } strong { "{data.gear.label()}" } }
                            div { class: "shift-advice",
                                if shift == ShiftAdvice::None {
                                    span { class: "shift-idle", "CONDUITE NORMALE" }
                                } else {
                                    span { class: "shift-active", "{shift_label}" }
                                }
                                small { "INDICATION ÉCO · SIMULÉE" }
                            }
                        }
                        if assist.mode != AssistanceMode::Off {
                            div { class: if assist.active { "assist-strip active" } else { "assist-strip" },
                                span { "{assist_label}" }
                                strong { "{assist_speed} km/h" }
                                span { "{assist_status}" }
                            }
                        } else {
                            div { class: "assist-strip dormant", "RÉGULATEUR / LIMITEUR  —" }
                        }
                    }
                    div { class: "cluster-dial velocity-dial", style: "--value: {speed_pct}%;",
                        div { class: "dial-ring",
                            div { class: "dial-core",
                                small { "VITESSE" }
                                strong { "{speed}" }
                                span { "km/h" }
                            }
                        }
                        div { class: "dial-scale", span { "0" } span { "40" } span { "80" } span { "120" } span { "160" } span { "200" } span { "240" } }
                        span { class: "dial-foot", "VITESSE / km/h" }
                    }
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
                if critical || unavailable || invalid {
                    div { class: "cluster-critical",
                        if unavailable { "DONNÉES INDISPONIBLES — NE PAS SE FIER À CET AFFICHAGE" }
                        else if critical { "ALERTE FREIN / PRESSION D'HUILE (SIMULÉE)" }
                        else { "TÉMOINS INCONNUS — VÉRIFICATION NÉCESSAIRE" }
                    }
                }
                div { class: "cluster-bottom",
                    div { class: "reservoir",
                        div { class: "reservoir-label", span { "⛽ CARBURANT" } strong { "{fuel}%" } }
                        div { class: "cluster-track", div { style: "width: {fuel_pct}%;" } }
                        div { class: "reservoir-extremes", span { "E" } span { "F" } }
                    }
                    div { class: "cluster-trip",
                        small { "TRAJET SIMULÉ" } strong { "{trip} km" }
                    }
                    div { class: "cluster-mileage",
                        small { "KILOMÉTRAGE FICTIF" } strong { "{mileage} km" }
                    }
                    div { class: "reservoir",
                        div { class: "reservoir-label", span { "◉ TEMPÉRATURE" } strong { "{coolant}°C" } }
                        div { class: "cluster-track coolant", div { style: "width: {coolant_pct}%;" } }
                        div { class: "reservoir-extremes", span { "C" } span { "H" } }
                    }
                }
                div { class: "sim-watermark", "PROTOTYPE · DONNÉES SIMULÉES · NE PAS UTILISER POUR CONDUIRE" }
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
