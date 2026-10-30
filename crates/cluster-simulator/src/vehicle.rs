//! K4M/JH3 technical *profile*, not a measured digital twin.
//! References in docs/VEHICLE_SOURCES.md; gearbox code must be checked on the car.
use cluster_core::Gear;
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct VehicleProfile {
    pub mass_kg: f64,
    pub tyre_width_mm: f64,
    pub tyre_aspect_pct: f64,
    pub rim_in: f64,
    pub final_drive: f64,
    pub ratios: [f64; 5],
    pub reverse_ratio: f64,
    pub drag_area_m2: f64,
    pub rolling_coefficient: f64,
    pub drivetrain_efficiency: f64,
}
impl Default for VehicleProfile {
    fn default() -> Self { Self {
        mass_kg: 1350.0, // laden driver + fluids approximation; trim/options vary
        tyre_width_mm: 195.0, tyre_aspect_pct: 65.0, rim_in: 15.0,
        final_drive: 4.06, ratios: [3.72, 2.04, 1.39, 1.09, 0.89], reverse_ratio: 3.54,
        drag_area_m2: 0.70, rolling_coefficient: 0.012, drivetrain_efficiency: 0.88,
    } }
}
impl VehicleProfile {
    pub fn wheel_radius_m(self) -> f64 { (self.rim_in * 25.4 / 2.0 + self.tyre_width_mm * self.tyre_aspect_pct / 100.0) / 1000.0 }
    pub fn circumference_m(self) -> f64 { 2.0 * PI * self.wheel_radius_m() }
    pub fn ratio(self, gear: Gear) -> f64 { match gear {
        Gear::Reverse => self.reverse_ratio, Gear::Neutral => 0.0,
        Gear::First => self.ratios[0], Gear::Second => self.ratios[1], Gear::Third => self.ratios[2], Gear::Fourth => self.ratios[3], Gear::Fifth => self.ratios[4],
    } }
    pub fn rpm_at_speed(self, gear: Gear, speed_kmh: f64) -> f64 {
        self.ratio(gear) * self.final_drive * speed_kmh.max(0.0) / 3.6 / self.circumference_m() * 60.0
    }
    pub fn speed_at_rpm(self, gear: Gear, rpm: f64) -> f64 {
        if self.ratio(gear) <= 0.0 { return 0.0; }
        rpm / (self.ratio(gear) * self.final_drive) * self.circumference_m() * 60.0 / 1000.0
    }
    /// Approximate K4M torque curve anchored at 152 Nm/4200 rpm and 83kW/6000 rpm.
    /// Intermediate samples are estimated, not a factory map.
    pub fn torque_nm(self, rpm: f64) -> f64 {
        const TABLE: [(f64, f64); 8] = [(760.0, 85.0), (1200.0, 106.0), (2000.0, 130.0), (3000.0, 143.0),
            (4200.0, 152.0), (5000.0, 148.0), (6000.0, 132.1), (6700.0, 108.0)];
        let r = rpm.clamp(TABLE[0].0, TABLE[TABLE.len()-1].0);
        for pair in TABLE.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if r <= b.0 { return a.1 + (b.1-a.1)*(r-a.0)/(b.0-a.0); }
        }
        TABLE[TABLE.len()-1].1
    }
    pub fn max_drive_force_n(self, gear: Gear, rpm: f64) -> f64 {
        self.torque_nm(rpm) * self.ratio(gear) * self.final_drive * self.drivetrain_efficiency / self.wheel_radius_m()
    }
    pub fn resistance_n(self, speed_kmh: f64, slope_pct: f64) -> f64 {
        let v = speed_kmh / 3.6;
        0.5 * 1.225 * self.drag_area_m2 * v * v + self.mass_kg * 9.81 * self.rolling_coefficient + self.mass_kg * 9.81 * slope_pct / 100.0
    }
}
#[cfg(test)] mod tests { use super::*;
    #[test] fn jh3_142_ratio_controls_wheel_speed() {
        let p = VehicleProfile::default();
        assert!(p.speed_at_rpm(Gear::First, 3000.0) < p.speed_at_rpm(Gear::Second, 3000.0));
        assert!(p.speed_at_rpm(Gear::Second, 3000.0) < p.speed_at_rpm(Gear::Fifth, 3000.0));
        assert!((p.speed_at_rpm(Gear::Fifth, 3000.0)-99.0).abs() < 5.0);
        assert!((p.rpm_at_speed(Gear::Third, 70.0)-3000.0).abs() < 500.0);
    }
    #[test] fn rated_power_is_plausible() { let p=VehicleProfile::default();
        let kw=p.torque_nm(6000.0)*6000.0*std::f64::consts::TAU/60.0/1000.0;
        assert!((kw-83.0).abs()<0.5);
    }
}
