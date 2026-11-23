//! Test-only spectral helpers: a mixed-radix FFT and alias detection.
//!
//! The FFT supports lengths whose prime factors are 2, 3 and 5, which covers
//! 4 800 and 48 000 samples at 48 kHz. With a rectangular window and a whole
//! number of signal periods in the analysis window, every component falls on an
//! exact bin and no window leakage correction is needed.

use std::f64::consts::TAU;

/// Computes the amplitude spectrum of a real signal.
///
/// The returned magnitude is normalised so that a full-scale real sinusoid has
/// a peak of 1.0 at its bin.
pub(crate) fn real_spectrum(samples: &[f32]) -> Vec<f64> {
    let mut re: Vec<f64> = samples.iter().map(|sample| f64::from(*sample)).collect();
    let mut im = vec![0.0_f64; samples.len()];
    fft(&mut re, &mut im);

    let scale = 2.0 / samples.len() as f64;
    re.iter()
        .zip(&im)
        .map(|(real, imaginary)| (real * real + imaginary * imaginary).sqrt() * scale)
        .collect()
}

/// Returns the largest magnitude in `bin ± radius`.
pub(crate) fn measure_peak_magnitude(spectrum: &[f64], bin: usize, radius: usize) -> f64 {
    let start = bin.saturating_sub(radius);
    let end = (bin + radius + 1).min(spectrum.len());
    spectrum[start..end].iter().copied().fold(0.0_f64, f64::max)
}

/// Finds the strongest non-harmonic component below `max_hz`.
///
/// Returns the bin and its magnitude. Harmonic bins (including the fundamental)
/// are excluded because they are the intended content of the waveform.
pub(crate) fn find_alias_peak(
    spectrum: &[f64],
    fundamental_bin: usize,
    fundamental_hz: f32,
    sample_rate_hz: f32,
    analysis_frames: usize,
    max_hz: f32,
) -> (usize, f64) {
    let last_bin = ((f64::from(max_hz) * analysis_frames as f64 / f64::from(sample_rate_hz))
        as usize)
        .min(spectrum.len().saturating_sub(1));
    let mut harmonic = vec![false; spectrum.len()];

    let mut multiple = 1_usize;
    while let Some(bin) = fundamental_bin.checked_mul(multiple)
        && bin < harmonic.len()
    {
        harmonic[bin] = true;
        multiple += 1;
    }
    debug_assert!(fundamental_hz > 0.0);

    let mut best_bin = 0_usize;
    let mut best = 0.0_f64;
    for bin in 1..last_bin {
        if harmonic[bin] {
            continue;
        }
        if spectrum[bin] > best {
            best = spectrum[bin];
            best_bin = bin;
        }
    }

    (best_bin, best)
}

/// In-place iterative radix-2 FFT for power-of-two lengths.
fn radix2(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    if n <= 1 {
        return;
    }

    let mut j = 0_usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut length = 2_usize;
    while length <= n {
        let angle = -TAU / length as f64;
        let (step_im, step_re) = angle.sin_cos();
        let mut start = 0_usize;
        while start < n {
            let mut w_re = 1.0_f64;
            let mut w_im = 0.0_f64;
            for offset in 0..length / 2 {
                let even = start + offset;
                let odd = even + length / 2;
                let odd_re = re[odd] * w_re - im[odd] * w_im;
                let odd_im = re[odd] * w_im + im[odd] * w_re;
                re[odd] = re[even] - odd_re;
                im[odd] = im[even] - odd_im;
                re[even] += odd_re;
                im[even] += odd_im;

                let next_re = w_re * step_re - w_im * step_im;
                w_im = w_re * step_im + w_im * step_re;
                w_re = next_re;
            }
            start += length;
        }
        length <<= 1;
    }
}

/// Recursive mixed-radix decimation-in-time FFT (radices 2, 3 and 5).
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    debug_assert_eq!(im.len(), n);
    if n <= 1 {
        return;
    }

    if n.is_power_of_two() {
        radix2(re, im);
        return;
    }

    let radix = if n.is_multiple_of(2) {
        2
    } else if n.is_multiple_of(3) {
        3
    } else if n.is_multiple_of(5) {
        5
    } else {
        n
    };
    let stride = n / radix;

    let mut sub_re = vec![vec![0.0_f64; stride]; radix];
    let mut sub_im = vec![vec![0.0_f64; stride]; radix];
    for (branch, (branch_re, branch_im)) in sub_re.iter_mut().zip(sub_im.iter_mut()).enumerate() {
        for index in 0..stride {
            branch_re[index] = re[index * radix + branch];
            branch_im[index] = im[index * radix + branch];
        }
        fft(branch_re, branch_im);
    }

    let mut out_re = vec![0.0_f64; n];
    let mut out_im = vec![0.0_f64; n];
    for frequency in 0..n {
        let reduced = frequency % stride;
        let mut sum_re = 0.0_f64;
        let mut sum_im = 0.0_f64;
        for branch in 0..radix {
            let angle = -TAU * (branch * frequency) as f64 / n as f64;
            let (step_im, step_re) = angle.sin_cos();
            let branch_re = sub_re[branch][reduced];
            let branch_im = sub_im[branch][reduced];
            sum_re += branch_re * step_re - branch_im * step_im;
            sum_im += branch_re * step_im + branch_im * step_re;
        }
        out_re[frequency] = sum_re;
        out_im[frequency] = sum_im;
    }

    re.copy_from_slice(&out_re);
    im.copy_from_slice(&out_im);
}

#[cfg(test)]
mod tests {
    use super::{fft, real_spectrum};

    #[test]
    fn mixed_radix_fft_matches_a_direct_dft() {
        let n = 30_usize;
        let samples: Vec<f32> = (0..n)
            .map(|index| (0.3 * index as f32).sin() + (0.11 * index as f32).cos())
            .collect();

        let spectrum = real_spectrum(&samples);
        for (bin, measured) in spectrum.iter().enumerate().take(n / 2) {
            let mut re = 0.0_f64;
            let mut im = 0.0_f64;
            for (index, sample) in samples.iter().enumerate() {
                let angle = -std::f64::consts::TAU * (bin * index) as f64 / n as f64;
                re += f64::from(*sample) * angle.cos();
                im += f64::from(*sample) * angle.sin();
            }
            let expected = (re * re + im * im).sqrt() * 2.0 / n as f64;
            assert!((measured - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn bin_aligned_sine_lands_on_one_bin() {
        let n = 4_800_usize;
        let frequency_bin = 11_usize;
        let samples: Vec<f32> = (0..n)
            .map(|index| {
                (std::f64::consts::TAU * frequency_bin as f64 * index as f64 / n as f64).sin()
                    as f32
            })
            .collect();

        let spectrum = real_spectrum(&samples);
        assert!((spectrum[frequency_bin] - 1.0).abs() < 1e-9);

        let largest_other = spectrum
            .iter()
            .take(n / 2)
            .enumerate()
            .filter(|(bin, _)| *bin != frequency_bin)
            .map(|(_, magnitude)| *magnitude)
            .fold(0.0_f64, f64::max);
        println!("largest non-fundamental bin: {largest_other:.3e}");
        assert!(largest_other < 1e-6);
    }

    #[test]
    fn fft_handles_power_of_two_lengths() {
        let mut re = vec![1.0, 1.0, 1.0, 1.0];
        let mut im = vec![0.0; 4];
        fft(&mut re, &mut im);
        assert!((re[0] - 4.0).abs() < 1e-12);
        assert!(re[1].abs() < 1e-12);
    }

    #[test]
    fn forty_eight_thousand_point_fft_is_exact() {
        let n = 48_000_usize;
        let samples: Vec<f32> = (0..n)
            .map(|index| (std::f64::consts::TAU * 440.0 * index as f64 / n as f64).sin() as f32)
            .collect();
        let spectrum = real_spectrum(&samples);
        assert!((spectrum[440] - 1.0).abs() < 1e-6);
        assert!(spectrum[40] < 1e-6);
    }
}
