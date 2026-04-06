fn hash2(x: i32, y: i32) -> u32 {
    let mut v = (x as u32).wrapping_mul(0x85eb_ca6b) ^ (y as u32).wrapping_mul(0xc2b2_ae35);
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb_352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846c_a68b);
    v ^ (v >> 16)
}

fn fade(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn value_at(x: i32, y: i32) -> f32 {
    hash2(x, y) as f32 / u32::MAX as f32
}

pub fn sample_value_noise(x: f32, y: f32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let tx = fade(x - x0 as f32);
    let ty = fade(y - y0 as f32);

    let v00 = value_at(x0, y0);
    let v10 = value_at(x1, y0);
    let v01 = value_at(x0, y1);
    let v11 = value_at(x1, y1);

    let top = lerp(v00, v10, tx);
    let bottom = lerp(v01, v11, tx);
    lerp(top, bottom, ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_value_noise_is_deterministic() {
        let a = sample_value_noise(1.25, 9.5);
        let b = sample_value_noise(1.25, 9.5);
        assert_eq!(a, b);
    }

    #[test]
    fn sample_value_noise_stays_in_unit_interval() {
        let sample = sample_value_noise(3.1, 4.2);
        assert!((0.0..=1.0).contains(&sample));
    }
}
