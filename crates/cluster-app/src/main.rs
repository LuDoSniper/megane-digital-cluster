mod config;

use cluster_core::{DataQuality, LampState, Reading, TelemetrySource, VehicleSnapshot};
use cluster_simulator::{Command, Lamp, Scenario, Simulator};
use config::{AppConfig, Theme};
use dioxus::prelude::*;
use dioxus_desktop::{Config as DesktopConfig, LogicalSize, WindowBuilder};
use dioxus_desktop::tao::window::Fullscreen;
use std::time::{Duration, Instant};

const CSS: &str = include_str!("../assets/main.css");

fn main() {
    let config = AppConfig::load();
    let mut window = WindowBuilder::new()
        .with_title("Mégane Digital Cluster — Prototype")
        .with_inner_size(LogicalSize::new(config.window.width as f64, config.window.height as f64))
        .with_min_inner_size(LogicalSize::new(960.0, 600.0))
        .with_resizable(true);
    if config.window.fullscreen {
        window = window.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    dioxus::LaunchBuilder::new()
        .with_context(config)
        .with_cfg(DesktopConfig::new().with_window(window))
        .launch(app);
}

fn dispatch(mut sim: Signal<Simulator>, mut reading: Signal<VehicleSnapshot>, command: Command) {
    sim.write().apply(command);
    let updated = sim.read().snapshot();
    reading.set(updated);
}

fn display(value: &Reading<f64>, digits: usize) -> String {
    value.usable().map(|v| format!("{v:.digits$}")).unwrap_or_else(|| "—".to_owned())
}

fn lamp_class(state: LampState, tone: &str) -> String {
    match state {
        LampState::On => format!("lamp lamp-{tone} lamp-active"),
        LampState::Off => "lamp".to_owned(),
        LampState::Unknown => "lamp lamp-unknown".to_owned(),
    }
}

fn app() -> Element {
    let config = use_context::<AppConfig>();
    let mut sim = use_signal(Simulator::default);
    let mut reading = use_signal(|| Simulator::default().snapshot());
    let mut theme = use_signal(|| config.interface.theme);
    let tick_ms = config.interface.tick_ms;

    use_future(move || async move {
        let mut previous = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_millis(tick_ms)).await;
            let now = Instant::now();
            let delta = now.duration_since(previous);
            previous = now;
            sim.write().step(delta);
            reading.set(sim.read().snapshot());
        }
    });

    let state = sim.read().clone();
    let data = reading.read().clone();
    let current_theme = theme();
    let theme_class = current_theme.class();
    let speed = display(&data.speed_kmh, 0);
    let rpm = display(&data.rpm, 0);
    let rpm_sweep = data.rpm.usable().map(|v| (v / 7000.0 * 270.0).clamp(0.0, 270.0)).unwrap_or(0.0);
    let rpm_percentage = data.rpm.usable().map(|v| (v / 7000.0 * 100.0).clamp(0.0, 100.0)).unwrap_or(0.0);
    let rpm_zone = if data.rpm.usable().is_some_and(|v| *v >= 6000.0) { "rpm-alert" } else { "" };
    let signal_text = if state.scenario == Scenario::SensorFailure { "CAPTEUR INDISPONIBLE" }
        else if state.scenario == Scenario::LinkLost { "LIAISON PERDUE" }
        else if state.paused { "SIMULATION EN PAUSE" } else { "TÉLÉMÉTRIE SIMULÉE" };
    let connection_class = if state.scenario == Scenario::LinkLost || state.scenario == Scenario::SensorFailure { "status-bad" } else { "status-good" };
    let fuel = data.fuel_pct.usable().copied().unwrap_or(0.0).clamp(0.0, 100.0);
    let coolant = display(&data.coolant_c, 0);
    let battery = display(&data.battery_v, 1);
    let trip = display(&data.trip_km, 1);
    let odometer = display(&data.simulated_odometer_km, 1);
    let speed_quality = match data.speed_kmh.quality {
        DataQuality::Simulated => "SIM", DataQuality::Unknown => "INCONNU", DataQuality::Stale => "PÉRIMÉ",
    };

    rsx! {
        document::Style { "{CSS}" }
        div { class: "app-shell {theme_class}",
            header { class: "app-header",
                div { class: "brand",
                    span { class: "brand-icon", "M" }
                    div {
                        strong { "MÉGANE" }
                        small { "DIGITAL CLUSTER / LAB 01" }
                    }
                }
                div { class: "header-middle",
                    span { class: "live-dot" }
                    "MODE SIMULATION · PHASE 1"
                }
                div { class: "theme-controls",
                    span { "THÈME" }
                    for choice in [Theme::Rs, Theme::Classic, Theme::Ice] {
                        button {
                            key: "{choice:?}",
                            class: if current_theme == choice { "theme-button selected" } else { "theme-button" },
                            onclick: move |_| theme.set(choice),
                            "{choice.label()}"
                        }
                    }
                }
            }
            main { class: "workspace",
                section { class: "instrument-panel",
                    div { class: "instrument-heading",
                        div {
                            span { class: "eyebrow", "INSTRUMENTATION" }
                            h1 { "Tableau de bord" }
                        }
                        div { class: "connection {connection_class}",
                            span { class: "connection-dot" }
                            "{signal_text}"
                        }
                    }
                    div { class: "cockpit",
                        div { class: "cockpit-top",
                            div { class: "top-left",
                                span { class: "eyebrow", "RENAULT MÉGANE II · 1.6 16V" }
                                span { class: "vehicle-type", "SEDAN / 2004" }
                            }
                            div { class: "gear-status",
                                span { "RAPPORT" }
                                strong { "{data.gear.label()}" }
                            }
                            div { class: "engine-state", if data.engine_running { "MOTEUR EN MARCHE" } else { "MOTEUR ARRÊTÉ" } }
                        }
                        div { class: "gauges",
                            div { class: "speed-area",
                                span { class: "gauge-label", "VITESSE" }
                                div { class: "speed-value", "{speed}" }
                                span { class: "speed-unit", "km/h" }
                                div { class: "speed-baseline" }
                                span { class: "quality-label", "SOURCE · {speed_quality}" }
                            }
                            div { class: "rpm-area",
                                div {
                                    class: "rpm-dial {rpm_zone}",
                                    style: "--rpm-sweep: {rpm_sweep}deg;",
                                    div { class: "rpm-inner",
                                        span { class: "gauge-label", "RÉGIME MOTEUR" }
                                        strong { "{rpm}" }
                                        span { "tr/min" }
                                    }
                                }
                                div { class: "rpm-ticks", span { "0" } span { "2" } span { "4" } span { "6" } span { "7" } }
                            }
                        }
                        div { class: "rpm-progress",
                            div { class: "rpm-progress-fill", style: "width: {rpm_percentage}%;" }
                        }
                        div { class: "warnings",
                            Warning { label: "MOTEUR", state: data.lights.engine, tone: "amber" }
                            Warning { label: "HUILE", state: data.lights.oil_pressure, tone: "red" }
                            Warning { label: "BATTERIE", state: data.lights.battery, tone: "red" }
                            Warning { label: "ABS", state: data.lights.abs, tone: "amber" }
                            Warning { label: "AIRBAG", state: data.lights.airbag, tone: "red" }
                            Warning { label: "FREIN", state: data.lights.brake, tone: "red" }
                            Warning { label: "CEINTURE", state: data.lights.seatbelt, tone: "red" }
                            Warning { label: "◀", state: data.lights.left_indicator, tone: "green" }
                            Warning { label: "▶", state: data.lights.right_indicator, tone: "green" }
                            Warning { label: "PHARES", state: data.lights.main_beam, tone: "blue" }
                        }
                        div { class: "telemetry-strip",
                            Metric { label: "CARBURANT", value: "{display(&data.fuel_pct, 0)}", unit: "%", progress: Some(fuel) }
                            Metric { label: "TEMP. MOTEUR", value: "{coolant}", unit: "°C" }
                            Metric { label: "BATTERIE", value: "{battery}", unit: "V" }
                            Metric { label: "TRAJET SIMULÉ", value: "{trip}", unit: "km" }
                        }
                        div { class: "cockpit-foot",
                            span { "SIMULATION UNIQUEMENT · AUCUNE CONNEXION AU VÉHICULE" }
                            span { "ODOMÈTRE FICTIF : {odometer} km" }
                        }
                    }
                    div { class: "info-line",
                        span { class: "info-icon", "i" }
                        "Les données et voyants sont fictifs. Un signal inconnu est indiqué « ? », jamais comme un voyant éteint."
                    }
                }
                aside { class: "control-panel",
                    div { class: "aside-header",
                        div { span { class: "eyebrow", "LABORATOIRE" } h2 { "Simulateur" } }
                        span { class: "tag", "LIVE" }
                    }
                    div { class: "control-section",
                        span { class: "field-label", "ÉTAT DU MOTEUR" }
                        div { class: "button-group",
                            button {
                                class: if state.engine_running { "action-button danger" } else { "action-button positive" },
                                onclick: move |_| dispatch(sim, reading, Command::ToggleEngine),
                                if state.engine_running { "Arrêter le moteur" } else { "Démarrer le moteur" }
                            }
                            button { class: "action-button secondary", onclick: move |_| dispatch(sim, reading, Command::Pause),
                                if state.paused { "Reprendre" } else { "Pause" }
                            }
                        }
                    }
                    div { class: "control-section",
                        label { class: "field-label", r#for: "scenario", "SCÉNARIO DE TEST" }
                        select {
                            id: "scenario",
                            value: "{state.scenario.label()}",
                            onchange: move |event| {
                                let selected = event.value();
                                if let Some(sc) = Scenario::ALL.into_iter().find(|s| s.label() == selected) {
                                    dispatch(sim, reading, Command::SetScenario(sc));
                                }
                            },
                            for scenario in Scenario::ALL {
                                option { key: "{scenario:?}", value: "{scenario.label()}", "{scenario.label()}" }
                            }
                        }
                    }
                    div { class: "control-section",
                        div { class: "range-header", span { class: "field-label", "ACCÉLÉRATEUR" } strong { "{state.accelerator_pct:.0}%" } }
                        input { r#type: "range", min: "0", max: "100", step: "1",
                            value: "{state.accelerator_pct:.0}",
                            oninput: move |event| if let Ok(v) = event.value().parse::<f64>() { dispatch(sim, reading, Command::Accelerator(v)); }
                        }
                        div { class: "range-header", span { class: "field-label", "FREIN" } strong { "{state.brake_pct:.0}%" } }
                        input { r#type: "range", min: "0", max: "100", step: "1",
                            value: "{state.brake_pct:.0}",
                            oninput: move |event| if let Ok(v) = event.value().parse::<f64>() { dispatch(sim, reading, Command::Brake(v)); }
                        }
                    }
                    div { class: "control-section",
                        span { class: "field-label", "BOÎTE MANUELLE" }
                        div { class: "shift-row",
                            button { class: "shift-button", onclick: move |_| dispatch(sim, reading, Command::ShiftDown), "−" }
                            div { class: "shift-value", "{state.gear.label()}" }
                            button { class: "shift-button", onclick: move |_| dispatch(sim, reading, Command::ShiftUp), "+" }
                        }
                    }
                    div { class: "control-section",
                        span { class: "field-label", "ACTIVER LES TÉMOINS" }
                        div { class: "toggle-grid",
                            LampToggle { label: "Moteur", lamp: Lamp::Engine, active: state.lamp_active(Lamp::Engine), sim, reading }
                            LampToggle { label: "Huile", lamp: Lamp::Oil, active: state.lamp_active(Lamp::Oil), sim, reading }
                            LampToggle { label: "Batterie", lamp: Lamp::Battery, active: state.lamp_active(Lamp::Battery), sim, reading }
                            LampToggle { label: "ABS", lamp: Lamp::Abs, active: state.lamp_active(Lamp::Abs), sim, reading }
                            LampToggle { label: "Airbag", lamp: Lamp::Airbag, active: state.lamp_active(Lamp::Airbag), sim, reading }
                            LampToggle { label: "Frein", lamp: Lamp::Brake, active: state.lamp_active(Lamp::Brake), sim, reading }
                            LampToggle { label: "Ceinture", lamp: Lamp::Seatbelt, active: state.lamp_active(Lamp::Seatbelt), sim, reading }
                            LampToggle { label: "Clign. G", lamp: Lamp::Left, active: state.lamp_active(Lamp::Left), sim, reading }
                            LampToggle { label: "Clign. D", lamp: Lamp::Right, active: state.lamp_active(Lamp::Right), sim, reading }
                            LampToggle { label: "Phares", lamp: Lamp::Beam, active: state.lamp_active(Lamp::Beam), sim, reading }
                        }
                    }
                    div { class: "control-footer",
                        button { class: "reset-button", onclick: move |_| dispatch(sim, reading, Command::ResetTrip), "Réinitialiser trajet" }
                        button { class: "reset-button", onclick: move |_| dispatch(sim, reading, Command::ResetAll), "Réinitialiser tout" }
                    }
                }
            }
            footer { class: "app-footer",
                span { "MÉGANE DIGITAL CLUSTER · v0.1.0" }
                span { "PROTOTYPE NON HOMOLOGUÉ — NE PAS UTILISER COMME INSTRUMENT DE CONDUITE" }
            }
        }
    }
}

#[component]
fn Warning(label: &'static str, state: LampState, tone: &'static str) -> Element {
    let class = lamp_class(state, tone);
    rsx! { div { class: "{class}", span { "{label}" }, if state == LampState::Unknown { em { "?" } } } }
}

#[component]
fn Metric(label: &'static str, value: String, unit: &'static str, progress: Option<f64>) -> Element {
    rsx! {
        div { class: "metric",
            span { class: "metric-label", "{label}" }
            div { class: "metric-value", strong { "{value}" }, span { "{unit}" } }
            if let Some(progress) = progress {
                div { class: "metric-track", div { class: "metric-bar", style: "width: {progress}%;" } }
            }
        }
    }
}

#[component]
fn LampToggle(label: &'static str, lamp: Lamp, active: bool, sim: Signal<Simulator>, reading: Signal<VehicleSnapshot>) -> Element {
    rsx! {
        button {
            class: if active { "lamp-toggle active" } else { "lamp-toggle" },
            onclick: move |_| dispatch(sim, reading, Command::ToggleLamp(lamp)),
            "{label}"
        }
    }
}
