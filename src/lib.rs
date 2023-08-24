pub mod parser;
pub mod serial;

/**
 * Rust Implementation of RoundTo npm crate
 * @author - yexiuph
 */
pub struct RoundTo {}

impl RoundTo {
    fn round(method: fn(f32) -> f32, number: f32, precision: i32) -> f32 {
        let factor = 10_f32.powi(precision);
        let abs_number = number.abs();
        (method(abs_number * factor) / factor) * if number < 0.0 { -1.0 } else { 1.0 }
    }

    pub fn round_to(number: f32, precision: i32) -> f32 {
        Self::round(f32::round, number, precision)
    }

    pub fn round_to_ceil(number: f32, precision: i32) -> f32 {
        Self::round(f32::ceil, number, precision)
    }

    pub fn round_to_floor(number: f32, precision: i32) -> f32 {
        Self::round(f32::floor, number, precision)
    }
}
