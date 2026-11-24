pub const WINDOW: usize = 2048;
pub const BANDS: usize = 18;

// Hann-windowed radix-2 FFT; logarithmic bands retain bass detail.
#[cfg(test)]
pub fn levels(samples: &[f32], sample_rate: u32) -> [f32; BANDS] {
    levels_with_gain(samples, sample_rate, 3.0)
}

pub fn levels_with_gain(samples: &[f32], sample_rate: u32, gain: f32) -> [f32; BANDS] {
    let mut re = [0.0f32; WINDOW];
    let mut im = [0.0f32; WINDOW];
    for (i, &s) in samples.iter().take(WINDOW).enumerate() {
        re[i] = s * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / WINDOW as f32).cos());
    }
    let mut j = 0;
    for i in 1..WINDOW {
        let mut bit = WINDOW >> 1;
        while j & bit != 0 { j ^= bit; bit >>= 1; }
        j ^= bit;
        if i < j { re.swap(i, j); im.swap(i, j); }
    }
    let mut len = 2;
    while len <= WINDOW {
        let angle = -std::f32::consts::TAU / len as f32;
        let (wi, wr) = angle.sin_cos();
        for start in (0..WINDOW).step_by(len) {
            let (mut r, mut s) = (1.0, 0.0);
            for i in start..start + len / 2 {
                let k = i + len / 2;
                let (tr, ti) = (r * re[k] - s * im[k], r * im[k] + s * re[k]);
                re[k] = re[i] - tr; im[k] = im[i] - ti;
                re[i] += tr; im[i] += ti;
                (r, s) = (r * wr - s * wi, r * wi + s * wr);
            }
        }
        len *= 2;
    }
    let mut peaks = [0.0f32; BANDS];
    for bin in 1..WINDOW / 2 {
        let hz = bin as f32 * sample_rate as f32 / WINDOW as f32;
        if !(40.0..=16000.0).contains(&hz) { continue; }
        let band = ((hz / 40.0).ln() / (16000.0f32 / 40.0).ln() * BANDS as f32) as usize;
        let magnitude = re[bin].hypot(im[bin]) / (WINDOW as f32 / 4.0);
        let index = band.min(BANDS - 1);
        peaks[index] = peaks[index].max(magnitude);
    }
    peaks.map(|m| ((m * gain * 35.0).ln_1p() / 20.0f32.ln()).clamp(0.0, 1.0))
}

pub fn smooth(previous: &mut [f32; BANDS], target: &[f32; BANDS]) {
    for (old, &new) in previous.iter_mut().zip(target) {
        let speed = if new > *old { 0.7 } else { 0.3 };
        *old += (new - *old) * speed;
        if *old < 0.015 { *old = 0.0; }
    }
}

pub fn mono_color(bands: &[f32; BANDS], x: f32, y: f32, color: [u8; 3]) -> [u8; 3] {
    let band = ((x.clamp(0.0, 1.0) * BANDS as f32) as usize).min(BANDS - 1);
    if bands[band] > (1.0 - y.clamp(0.0, 1.0)) * 0.8 + 0.04 { color } else { [0; 3] }
}

// x/y are normalized key positions; y=0 is the top row.
#[cfg(test)]
pub fn color(bands: &[f32; BANDS], x: f32, y: f32) -> [u8; 3] {
    let x = x.clamp(0.0, 1.0);
    let band = ((x * BANDS as f32) as usize).min(BANDS - 1);
    let height = bands[band];
    let value = ((height - (1.0 - y.clamp(0.0, 1.0)) * 0.8) * 5.0).clamp(0.0, 1.0) * 0.8;
    let h = (0.65 - x * 0.65) * 6.0;
    let c = value * 0.9;
    let second = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as usize {
        0 => (c, second, 0.0), 1 => (second, c, 0.0),
        2 => (0.0, c, second), 3 => (0.0, second, c),
        _ => (second, 0.0, c),
    };
    [((r + value * 0.1) * 255.0) as u8, ((g + value * 0.1) * 255.0) as u8, ((b + value * 0.1) * 255.0) as u8]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_color_retains_bar_height_without_generating_shades() {
        let color = [255, 60, 0];
        assert_eq!(mono_color(&[0.5; BANDS], 0.5, 1.0, color), color);
        assert_eq!(mono_color(&[0.5; BANDS], 0.5, 0.0, color), [0; 3]);
        assert_eq!(mono_color(&[0.0; BANDS], 0.5, 1.0, color), [0; 3]);
    }
    fn tone(hz: f32, amplitude: f32, rate: u32) -> Vec<f32> {
        (0..WINDOW).map(|i| amplitude * (std::f32::consts::TAU * hz * i as f32 / rate as f32).sin()).collect()
    }
    #[test]
    fn gain_changes_spectrum_strength() {
        let samples = tone(1000.0, 0.01, 48000);
        let low = levels_with_gain(&samples, 48000, 0.3);
        let high = levels_with_gain(&samples, 48000, 8.0);
        assert!(high.iter().copied().fold(0.0, f32::max) > low.iter().copied().fold(0.0, f32::max) + 0.3);
    }
    fn strongest(bands: &[f32; BANDS]) -> usize {
        (0..BANDS).max_by(|&a, &b| bands[a].total_cmp(&bands[b])).unwrap()
    }
    #[test]
    fn silence_is_black() {
        assert_eq!(levels(&[0.0; WINDOW], 48000), [0.0; BANDS]);
        assert_eq!(color(&[0.0; BANDS], 0.5, 1.0), [0, 0, 0]);
    }
    #[test]
    fn bass_and_treble_use_different_columns_at_both_sample_rates() {
        for rate in [44100, 48000] {
            let low = levels(&tone(100.0, 0.5, rate), rate);
            let high = levels(&tone(6000.0, 0.5, rate), rate);
            assert!(strongest(&low) < 5);
            assert!(strongest(&high) > 12);
        }
    }
    #[test]
    fn louder_signal_has_taller_bar() {
        let quiet = levels(&tone(1000.0, 0.01, 48000), 48000);
        let loud = levels(&tone(1000.0, 0.8, 48000), 48000);
        assert!(loud[strongest(&loud)] > quiet[strongest(&quiet)] + 0.5);
    }
    #[test]
    fn bars_rise_from_bottom_and_use_multiple_colors() {
        let half = [0.5; BANDS];
        assert_ne!(color(&half, 0.2, 1.0), [0, 0, 0]);
        assert_eq!(color(&half, 0.2, 0.0), [0, 0, 0]);
        assert_ne!(color(&half, 0.2, 1.0), color(&half, 0.8, 1.0));
    }
    #[test]
    fn bars_decay_to_black_after_silence() {
        let mut bands = [1.0; BANDS];
        for _ in 0..20 { smooth(&mut bands, &[0.0; BANDS]); }
        assert_eq!(bands, [0.0; BANDS]);
    }
}
