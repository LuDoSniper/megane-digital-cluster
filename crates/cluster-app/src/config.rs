//! Window options intentionally default to windowed mode.
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub interface: InterfaceConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { window: WindowConfig::default(), interface: InterfaceConfig::default() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { fullscreen: false, width: 1360, height: 840 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceConfig {
    pub theme: Theme,
    pub layout: Layout,
    pub tick_ms: u64,
    pub show_lab_on_start: bool,
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        Self { theme: Theme::Rs, layout: Layout::Original, tick_ms: 50, show_lab_on_start: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme { Rs, Classic, Ice }

impl Theme {
    pub fn class(self) -> &'static str {
        match self { Self::Rs => "theme-rs", Self::Classic => "theme-classic", Self::Ice => "theme-ice" }
    }
    pub fn label(self) -> &'static str {
        match self { Self::Rs => "RS", Self::Classic => "Classique", Self::Ice => "Glacier" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Layout { Original, Touring, Focus }
impl Layout {
    pub fn class(self) -> &'static str { match self { Self::Original=>"layout-original",Self::Touring=>"layout-touring",Self::Focus=>"layout-focus" } }
    pub fn label(self) -> &'static str { match self { Self::Original=>"RS Digital (original)",Self::Touring=>"Grand Touring",Self::Focus=>"Essentiel" } }
}

impl AppConfig {
    pub fn validate(&mut self) {
        self.window.width = self.window.width.clamp(960, 3840);
        self.window.height = self.window.height.clamp(600, 2160);
        self.interface.tick_ms = self.interface.tick_ms.clamp(16, 250);
    }

    pub fn load() -> Self {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(path) = env::var_os("CLUSTER_CONFIG") { candidates.push(path.into()); }
        candidates.push(PathBuf::from("config/cluster.toml"));
        if let Ok(exe) = env::current_exe() {
            if let Some(parent) = exe.parent() {
                candidates.push(parent.join("config/cluster.toml"));
                candidates.push(parent.join("cluster.toml"));
            }
        }
        for path in candidates {
            if !path.exists() { continue; }
            match fs::read_to_string(&path).ok().and_then(|content| toml::from_str::<AppConfig>(&content).ok()) {
                Some(mut config) => {
                    config.validate();
                    eprintln!("Configuration chargée : {}", path.display());
                    return config;
                }
                None => eprintln!("Configuration invalide : {} — valeurs sûres par défaut", path.display()),
            }
            // An invalid explicit file must not silently load a different config.
            return Self::default();
        }
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windowed_by_default() { assert!(!AppConfig::default().window.fullscreen); }
    #[test]
    fn incomplete_toml_is_safe() {
        let cfg: AppConfig = toml::from_str("[window]\nfullscreen = true\n").unwrap();
        assert!(cfg.window.fullscreen);
        assert_eq!(cfg.window.width, 1360);
    }
    #[test]
    fn bounds_are_clamped() {
        let mut c = AppConfig::default();
        c.window.width = 0;
        c.interface.tick_ms = 1;
        c.validate();
        assert_eq!(c.window.width, 960);
        assert_eq!(c.interface.tick_ms, 16);
    }
}
