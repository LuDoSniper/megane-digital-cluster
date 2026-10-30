mod config;
mod display;

use cluster_core::{Aperture, AssistanceMode, Gear, TelemetrySource, VehicleSnapshot, Wheel};
use cluster_simulator::{Command, Lamp, Scenario, Simulator};
use config::{AppConfig, Layout, Theme};
use display::Instrument;
use dioxus::prelude::*;
use dioxus_desktop::tao::window::Fullscreen;
use dioxus_desktop::{Config as DesktopConfig, LogicalSize, WindowBuilder};
use std::time::{Duration, Instant};

const CSS: &str = include_str!("../assets/main.css");

fn main() {
    let config = AppConfig::load();
    let mut window = WindowBuilder::new()
        .with_title("Mégane Digital Cluster — Simulateur")
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
    reading.set(sim.read().snapshot());
}

fn handle_shortcut(code: &str, sim: Signal<Simulator>, reading: Signal<VehicleSnapshot>, mut show_lab: Signal<bool>) {
    let state = sim.read().clone();
    let command = match code {
        "hold:ArrowUp" => Some(Command::HoldAccelerator(true)),
        "release:ArrowUp" => Some(Command::HoldAccelerator(false)),
        "hold:ArrowDown" => Some(Command::HoldBrake(true)),
        "release:ArrowDown" => Some(Command::HoldBrake(false)),
        "ArrowRight" => Some(Command::AdjustAssistance(1.0)),
        "ArrowLeft" => Some(Command::AdjustAssistance(-1.0)),
        "Digit0" | "Numpad0" => Some(Command::SetGear(Gear::Neutral)),
        "Digit1" | "Numpad1" => Some(Command::SetGear(Gear::First)),
        "Digit2" | "Numpad2" => Some(Command::SetGear(Gear::Second)),
        "Digit3" | "Numpad3" => Some(Command::SetGear(Gear::Third)),
        "Digit4" | "Numpad4" => Some(Command::SetGear(Gear::Fourth)),
        "Digit5" | "Numpad5" => Some(Command::SetGear(Gear::Fifth)),
        "KeyR" | "NumpadDecimal" => Some(Command::SetGear(Gear::Reverse)),
        "KeyJ" => Some(Command::Clutch(if state.clutch_pct > 2.0 { 0.0 } else { 100.0 })),
        "KeyQ" => Some(Command::ToggleLamp(Lamp::Left)),
        "KeyE" => Some(Command::ToggleLamp(Lamp::Right)),
        "KeyH" => Some(Command::ToggleHazards),
        "KeyB" => Some(Command::ToggleLamp(Lamp::Dipped)),
        "KeyF" => Some(Command::ToggleLamp(Lamp::Beam)),
        "KeyC" => Some(Command::SetAssistance(if state.assistance.mode == AssistanceMode::Cruise { AssistanceMode::Off } else { AssistanceMode::Cruise })),
        "KeyV" => Some(Command::SetAssistance(if state.assistance.mode == AssistanceMode::Limiter { AssistanceMode::Off } else { AssistanceMode::Limiter })),
        "Enter" | "NumpadEnter" => Some(Command::SetAssistanceSpeed),
        "NumpadAdd" | "Equal" => Some(Command::AdjustAssistance(1.0)),
        "NumpadSubtract" | "Minus" => Some(Command::AdjustAssistance(-1.0)),
        "KeyX" => Some(Command::CancelAssistance),
        "KeyZ" => Some(Command::ResumeAssistance),
        "Space" => Some(Command::Pause),
        "F2" => { show_lab.set(!show_lab()); None }
        "Escape" => { show_lab.set(false); None }
        _ => None,
    };
    if let Some(command) = command { dispatch(sim, reading, command); }
}

fn app() -> Element {
    let config = use_context::<AppConfig>();
    let mut sim = use_signal(Simulator::default);
    let mut reading = use_signal(|| Simulator::default().snapshot());
    let mut theme = use_signal(|| config.interface.theme);
    let mut layout = use_signal(|| config.interface.layout);
    let mut show_lab = use_signal(|| config.interface.show_lab_on_start);
    let mut show_help = use_signal(|| false);
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

    // Global shortcuts, including when the developer panel does not have focus.
    // Ignore text entry and modifier combinations. All interactions are simulator-only.
    use_future(move || async move {
        let mut eval = document::eval(r#"
            const keys = new Set(['ArrowLeft','ArrowRight',
              'Digit0','Digit1','Digit2','Digit3','Digit4','Digit5',
              'Numpad0','Numpad1','Numpad2','Numpad3','Numpad4','Numpad5',
              'NumpadDecimal','NumpadEnter','NumpadAdd','NumpadSubtract',
              'KeyR','KeyJ','KeyQ','KeyE','KeyH','KeyB','KeyF','KeyC','KeyV',
              'KeyX','KeyZ','Space','Enter','Equal','Minus','F2','Escape']);
            const pedals = new Set(['ArrowUp','ArrowDown']);
            const repeating = new Set(['ArrowLeft','ArrowRight','NumpadAdd','NumpadSubtract','Equal','Minus']);
            const held = new Set();
            const focusedInput = (ev) => ev.target && ((['SELECT','TEXTAREA'].includes(ev.target.tagName) || (ev.target.tagName === 'INPUT' && ev.target.type !== 'range')) || ev.target.isContentEditable);
            const keyDown = (ev) => {
              if (ev.ctrlKey || ev.altKey || ev.metaKey || focusedInput(ev)) return;
              if (pedals.has(ev.code)) {
                ev.preventDefault();
                if (!held.has(ev.code)) { held.add(ev.code); dioxus.send('hold:'+ev.code); }
              } else if (keys.has(ev.code)) {
                ev.preventDefault();
                if (!ev.repeat || repeating.has(ev.code)) dioxus.send(ev.code);
              }
            };
            const keyUp = (ev) => {
              if (pedals.has(ev.code) && held.delete(ev.code)) {
                ev.preventDefault(); dioxus.send('release:'+ev.code);
              }
            };
            const releaseAll = () => { for(const code of held) dioxus.send('release:'+code); held.clear(); };
            window.addEventListener('keydown',keyDown);
            window.addEventListener('keyup',keyUp);
            window.addEventListener('blur',releaseAll);
            document.addEventListener('visibilitychange', () => { if(document.hidden) releaseAll(); });
            await new Promise(() => {});
        "#);
        while let Ok(code) = eval.recv::<String>().await {
            handle_shortcut(&code, sim, reading, show_lab);
        }
    });

    let state = sim.read().clone();
    let data = reading.read().clone();
    let sim_setpoint = state.assistance.setpoint_kmh.map(|v| format!("{v:.0} km/h")).unwrap_or_else(|| "—".to_string());
    let current_theme = theme();
    let theme_class = current_theme.class();
    let current_layout = layout();
    let layout_class = current_layout.class();
    let lab_class = if show_lab() { "lab-open" } else { "lab-closed" };

    rsx! {
        document::Style { "{CSS}" }
        div { class: "app-shell {theme_class} {layout_class} {lab_class}",
            div { class: "view-toolbar",
                div { class: "view-title", span { class: "view-emblem", "◇" } span { "MÉGANE DIGITAL CLUSTER" } }
                div { class: "view-tools",
                    button { class: "subtle-control", onclick: move |_| show_help.set(!show_help()), "⌨ Raccourcis" }
                    button { class: "subtle-control lab-action", onclick: move |_| show_lab.set(!show_lab()),
                        if show_lab() { "✕ Masquer le laboratoire" } else { "⚙ Ouvrir le laboratoire · F2" }
                    }
                }
            }
            if show_help() {
                div { class: "shortcut-help",
                    button { class: "help-close", onclick: move |_| show_help.set(false), "✕" }
                    strong { "RACCOURCIS SIMULATEUR" }
                    p { "↑ maintenue : accélérer · ↓ maintenue : freiner · relâcher : pédale revient à 0 % · ← → : consigne −/+ 1 km/h · J : embrayage · Pavé 1–5 : rapport · 0 : point mort · R : marche arrière (à l'arrêt)." }
                    p { "Q / E : clignotants · H : détresse · B : feux de croisement · F : pleins phares · C : régulateur · V : limiteur." }
                    p { "Entrée : mémoriser vitesse (≥ 30 km/h) · + / − : consigne · X : annuler · Z : reprendre · Espace : pause · F2 : labo · Échap : fermer." }
                }
            }
            main { class: "instrument-workspace",
                Instrument { data: data.clone(), layout: current_layout }
                if show_lab() {
                    aside { class: "control-panel",
                        div { class: "aside-header",
                            div { span { class: "eyebrow", "LABORATOIRE / PHASE 1" } h2 { "Simulation" } }
                            span { class: "tag", "OFFLINE" }
                        }
                        div { class: "control-section",
                            span { class: "field-label", "MOTEUR ET SCÉNARIOS" }
                            div { class: "button-group",
                                button { class: "action-button", onclick: move |_| dispatch(sim, reading, Command::ToggleEngine),
                                    if state.engine_running { "Arrêter" } else { "Démarrer" }
                                }
                                button { class: "action-button", onclick: move |_| dispatch(sim, reading, Command::Pause),
                                    if state.paused { "Reprendre" } else { "Pause" }
                                }
                            }
                            select {
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
                            input { r#type: "range", min: "0", max: "100", step: "1", value: "{state.accelerator_pct:.0}",
                                oninput: move |event| if let Ok(v) = event.value().parse::<f64>() { dispatch(sim, reading, Command::Accelerator(v)); }
                            }
                            div { class: "range-header", span { class: "field-label", "FREIN" } strong { "{state.brake_pct:.0}%" } }
                            input { r#type: "range", min: "0", max: "100", step: "1", value: "{state.brake_pct:.0}",
                                oninput: move |event| if let Ok(v) = event.value().parse::<f64>() { dispatch(sim, reading, Command::Brake(v)); }
                            }
                            div { class: "range-header", span { class: "field-label", "EMBRAYAGE" } strong { "{state.clutch_pct:.0}%" } }
                            input { r#type: "range", min: "0", max: "100", step: "1", value: "{state.clutch_pct:.0}",
                                oninput: move |event| if let Ok(v) = event.value().parse::<f64>() { dispatch(sim, reading, Command::Clutch(v)); }
                            }
                            div { class: "range-header", span { class: "field-label", "PENTE (DESCENTE − / MONTÉE +)" } strong { "{state.road_slope_pct:.0}%" } }
                            input { r#type: "range", min: "-12", max: "12", step: "1", value: "{state.road_slope_pct:.0}",
                                oninput: move |event| if let Ok(v) = event.value().parse::<f64>() { dispatch(sim, reading, Command::RoadSlope(v)); }
                            }
                            div { class: "sim-info-line",
                                span { "PÉDALE DEMANDÉE · {state.accelerator_pct:.0}%" }
                                strong { "GAZ EFFECTIF · {state.assistance.effective_throttle_pct:.0}%" }
                            }
                        }
                        div { class: "control-section",
                            span { class: "field-label", "BOÎTE MANUELLE · R / N / 1–5" }
                            div { class: "gear-keys",
                                for (gear, label) in [(Gear::Reverse,"R"),(Gear::Neutral,"N"),(Gear::First,"1"),(Gear::Second,"2"),(Gear::Third,"3"),(Gear::Fourth,"4"),(Gear::Fifth,"5")] {
                                    button {
                                        key: "{label}", class: if state.gear == gear { "gear-key selected" } else { "gear-key" },
                                        onclick: move |_| dispatch(sim, reading, Command::SetGear(gear)), "{label}"
                                    }
                                }
                            }
                            small { class: "mini-help", "Marche arrière autorisée uniquement sous 2 km/h." }
                        }
                        div { class: "control-section",
                            span { class: "field-label", "ÉCLAIRAGE ET SIGNALISATION" }
                            div { class: "toggle-grid",
                                LampToggle { label: "Clign. gauche", lamp: Lamp::Left, active: state.lamp_active(Lamp::Left), sim, reading }
                                LampToggle { label: "Clign. droite", lamp: Lamp::Right, active: state.lamp_active(Lamp::Right), sim, reading }
                                button { class: if state.hazards { "lamp-toggle active" } else { "lamp-toggle" }, onclick: move |_| dispatch(sim, reading, Command::ToggleHazards), "Feux de détresse" }
                                LampToggle { label: "Croisement", lamp: Lamp::Dipped, active: state.lamp_active(Lamp::Dipped), sim, reading }
                                LampToggle { label: "Pleins phares", lamp: Lamp::Beam, active: state.lamp_active(Lamp::Beam), sim, reading }
                            }
                        }
                        div { class: "control-section",
                            span { class: "field-label", "RÉGULATEUR / LIMITEUR (SIMULATION)" }
                            div { class: "assist-controls",
                                for (mode, label) in [(AssistanceMode::Off,"OFF"),(AssistanceMode::Cruise,"RÉG."),(AssistanceMode::Limiter,"LIM.")] {
                                    button { key: "{label}", class: if state.assistance.mode == mode { "gear-key selected" } else { "gear-key" },
                                        onclick: move |_| dispatch(sim, reading, Command::SetAssistance(mode)), "{label}"
                                    }
                                }
                            }
                            div { class: "button-group",
                                button { class: "action-button", onclick: move |_| dispatch(sim, reading, Command::SetAssistanceSpeed), "SET" }
                                button { class: "action-button", onclick: move |_| dispatch(sim, reading, Command::ResumeAssistance), "RESUME" }
                            }
                            div { class: "assist-controls",
                                button { class: "action-button", onclick: move |_| dispatch(sim, reading, Command::AdjustAssistance(-1.0)), "− 1 km/h" }
                                button { class: "action-button", onclick: move |_| dispatch(sim, reading, Command::AdjustAssistance(1.0)), "+ 1 km/h" }
                                button { class: "action-button", onclick: move |_| dispatch(sim, reading, Command::CancelAssistance), "CANCEL" }
                            }
                            small { class: "mini-help", "Régulateur : maintient la consigne sans pédale ; frein / embrayage / changement de rapport = veille. Limiteur : plafonne le gaz, reste actif au freinage ; dépassement volontaire de 98 à 100 % (point dur simulé). Aucune assistance ne freine en descente." }
                            div { class: "sim-info-line",
                                span { "CONSIGNE : {sim_setpoint}" }
                                strong { if state.assistance.active { "ACTIF" } else { "EN VEILLE / OFF" } }
                            }
                        }
                        div { class: "control-section",
                            span { class: "field-label", "OUVRANTS · SEDAN 4 PORTES" }
                            div { class: "toggle-grid",
                                for aperture in Aperture::ALL {
                                    button { key: "{aperture:?}",
                                        class: if state.openings.is_open(aperture) { "lamp-toggle active" } else { "lamp-toggle" },
                                        onclick: move |_| dispatch(sim, reading, Command::ToggleOpening(aperture)),
                                        "{aperture.label()} · "
                                        if state.openings.is_open(aperture) { "ouvert" } else { "fermé" }
                                    }
                                }
                            }
                        }
                        div { class: "control-section",
                            span { class: "field-label", "PRESSION DES PNEUMATIQUES · SIMULATION" }
                            for wheel in Wheel::ALL {
                                div { key: "{wheel:?}", class: "range-header",
                                    span { class: "field-label", "{wheel.label()}" }
                                    strong { "{state.tyre_pressure_bar[wheel.index()]:.1} bar" }
                                }
                                input { r#type: "range", min: "0", max: "3.5", step: "0.1",
                                    value: "{state.tyre_pressure_bar[wheel.index()]}",
                                    oninput: move |event| if let Ok(v)=event.value().parse::<f64>() { dispatch(sim, reading, Command::SetTyrePressure(wheel,v)); }
                                }
                                button { class: if state.tyre_sensor_fault[wheel.index()] { "lamp-toggle active" } else { "lamp-toggle" },
                                    onclick: move |_| dispatch(sim, reading, Command::ToggleTyreSensor(wheel)),
                                    if state.tyre_sensor_fault[wheel.index()] { "Capteur indisponible · rétablir" } else { "Simuler panne capteur" }
                                }
                            }
                            small { class: "mini-help", "Seuils illustratifs du simulateur, différents de la logique Renault. Pressions réelles à vérifier sur l'étiquette du véhicule." }
                        }
                        div { class: "control-section",
                            span { class: "field-label", "INJECTION DE VOYANTS (SIMULÉS)" }
                            div { class: "toggle-grid",
                                LampToggle { label: "Moteur", lamp: Lamp::Engine, active: state.lamp_active(Lamp::Engine), sim, reading }
                                LampToggle { label: "Huile", lamp: Lamp::Oil, active: state.lamp_active(Lamp::Oil), sim, reading }
                                LampToggle { label: "Batterie", lamp: Lamp::Battery, active: state.lamp_active(Lamp::Battery), sim, reading }
                                LampToggle { label: "ABS", lamp: Lamp::Abs, active: state.lamp_active(Lamp::Abs), sim, reading }
                                LampToggle { label: "Airbag", lamp: Lamp::Airbag, active: state.lamp_active(Lamp::Airbag), sim, reading }
                                LampToggle { label: "Frein", lamp: Lamp::Brake, active: state.lamp_active(Lamp::Brake), sim, reading }
                                LampToggle { label: "Ceinture", lamp: Lamp::Seatbelt, active: state.lamp_active(Lamp::Seatbelt), sim, reading }
                            }
                        }
                        div { class: "control-section",
                            span { class: "field-label", "AFFICHAGE" }
                            span { class: "field-label", "DISPOSITION · PAR DÉFAUT RS DIGITAL" }
                            div { class: "layout-controls",
                                for choice in [Layout::Original, Layout::Touring, Layout::Focus] {
                                    button { key: "{choice:?}", class: if current_layout == choice { "theme-button selected" } else { "theme-button" },
                                        onclick: move |_| layout.set(choice), "{choice.label()}" }
                                }
                            }
                            span { class: "field-label", "COULEURS" }
                            div { class: "theme-controls",
                                for choice in [Theme::Rs, Theme::Classic, Theme::Ice] {
                                    button { key: "{choice:?}", class: if current_theme == choice { "theme-button selected" } else { "theme-button" },
                                        onclick: move |_| theme.set(choice), "{choice.label()}" }
                                }
                            }
                        }
                        div { class: "control-footer",
                            button { class: "reset-button", onclick: move |_| dispatch(sim, reading, Command::ResetTrip), "RAZ trajet" }
                            button { class: "reset-button", onclick: move |_| dispatch(sim, reading, Command::ResetAll), "Réinitialiser simulation" }
                        }
                    }
                }
            }
            div { class: "safety-caption", "AFFICHAGE EXPÉRIMENTAL · AUCUNE CONNEXION OBD · INSTRUMENTS RENAULT À CONSERVER" }
        }
    }
}

#[component]
fn LampToggle(label: &'static str, lamp: Lamp, active: bool, sim: Signal<Simulator>, reading: Signal<VehicleSnapshot>) -> Element {
    rsx! {
        button { class: if active { "lamp-toggle active" } else { "lamp-toggle" },
            onclick: move |_| dispatch(sim, reading, Command::ToggleLamp(lamp)), "{label}" }
    }
}
