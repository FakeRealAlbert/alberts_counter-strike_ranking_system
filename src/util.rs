#![allow(dead_code)]

pub fn sum_vector(vec: Vec<f64>) -> f64 {
    let mut sum = 0.0;
    for i in vec { sum += i }
    sum
}

pub fn remap_value_clamped(val: f64, in_low: f64, in_high: f64, out_low: f64, out_high: f64) -> f64 {
    let clamped_val = val.clamp(in_low,in_high);

    let interpolated = (clamped_val - in_low ) / (in_high - in_low);
    let clamped = interpolated.clamp(0.0, 1.0);

    clamped * out_high + ( 1.0 - clamped ) * out_low
}