use std::f64::consts::PI;

/// Helper: Converts an f64 value into 8.24 fixed-point format as u32.
fn to_fixed_unsigned(x: f64) -> u32 {
    let scale = (1 << 24) as f64;
    ((x * scale).round() as i32) as u32
}

/// A struct representing a single second–order filter section in floating–point.
#[derive(Debug, Clone, Copy)]
pub struct SecondOrderCoeffs {
    /// Numerator coefficients (b0, b1, b2)
    pub b: [f64; 3],
    /// Denominator coefficients (a1, a2).  
    /// Note: These are stored in the form required for your difference equation,
    /// where the filter is implemented as:
    ///   y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] + a1*y[n-1] + a2*y[n-2]
    pub a: [f64; 2],
}

impl SecondOrderCoeffs {
    /// Returns the coefficients as an array of 5 u32 values, in 8.24 fixed-point format.
    /// The order is: [b2, b1, b0, a2, a1]
    pub fn to_fixed(&self) -> [u32; 5] {
        [
            to_fixed_unsigned(self.b[0]),
            to_fixed_unsigned(self.b[1]),
            to_fixed_unsigned(self.b[2]),
            to_fixed_unsigned(self.a[1]),
            to_fixed_unsigned(self.a[0]),
        ]
    }
}

/// A struct that holds both low-pass and high-pass coefficients for a Linkwitz-Riley crossover.
/// Each branch has two cascaded second–order sections.
#[derive(Debug)]
pub struct LinkwitzRileyCoeffs {
    // Low-pass branch: Filter1 and Filter2 (for a 4th-order LR lowpass)
    pub lowpass_filter1: SecondOrderCoeffs,
    pub lowpass_filter2: SecondOrderCoeffs,
    // High-pass branch: Filter1 and Filter2 (for a 4th-order LR highpass)
    pub highpass_filter1: SecondOrderCoeffs,
    pub highpass_filter2: SecondOrderCoeffs,
}

impl LinkwitzRileyCoeffs {
    /// Create a new set of Linkwitz-Riley coefficients.
    ///
    /// Parameters:
    /// - `fs`: sampling frequency in Hz (e.g., 192000.0)
    /// - `fc`: crossover frequency in Hz (e.g., 100.0)
    /// - `lowpass_gain_db`: overall gain in dB for the low-pass branch (e.g., 3.0)
    ///
    /// In this design:
    /// - The low-pass branch is implemented as two cascaded second-order sections.
    ///   A common split is to assign the full numerator to Filter1 and reduce Filter2 by 1/√2.
    /// - The high-pass branch uses the standard Butterworth high-pass coefficients
    ///   (with no additional gain applied) so that both sections are identical.
    pub fn new(fs: f64, fc: f64, lowpass_gain_db: f64) -> Self {
        // -------------
        // Pre-warping common for both branches using the bilinear transform:
        // K = tan(π * fc / fs)
        // -------------
        #[allow(non_snake_case)]
        let K = (PI * fc / fs).tan();
        let sqrt2 = f64::sqrt(2.0);
        // norm is used to normalize the Butterworth prototype.
        let norm = 1.0 + sqrt2 * K + K * K;

        // =========================
        // LOW-PASS DESIGN
        // =========================
        // Canonical low-pass numerator coefficients (Butterworth)
        // b0_lp_std = (K^2) / norm;  b1_lp_std = 2*(K^2) / norm;  b2_lp_std = (K^2) / norm.
        let b0_lp_std = (K * K) / norm;
        let b1_lp_std = 2.0 * b0_lp_std;
        let b2_lp_std = b0_lp_std;

        // Canonical denominator (for low-pass) [before sign inversion]:
        // a1_std = 2*(K^2 – 1) / norm; a2_std = (1 – √2*K + K^2)/norm.
        let a1_lp_std = 2.0 * (K * K - 1.0) / norm;
        let a2_lp_std = (1.0 - sqrt2 * K + K * K) / norm;
        // Invert the sign for implementation on the ADAU1467:
        let a1_lp = -a1_lp_std;
        let a2_lp = -a2_lp_std;

        // Low-pass gain conversion from dB to linear:
        let lowpass_gain = 10_f64.powf(lowpass_gain_db / 20.0);
        // Apply gain to the numerator:
        let b0_lp = b0_lp_std * lowpass_gain;
        let b1_lp = b1_lp_std * lowpass_gain;
        let b2_lp = b2_lp_std * lowpass_gain;

        // For a 4th-order LR low-pass, we split the overall filter into two cascaded second-order sections.
        // One common method is to give the full numerator to Filter1 and reduce Filter2 by a factor of 1/√2.
        let lowpass_filter1 = SecondOrderCoeffs {
            b: [b0_lp, b1_lp, b2_lp],
            a: [a1_lp, a2_lp],
        };
        let lowpass_filter2 = SecondOrderCoeffs {
            b: [b0_lp / sqrt2, b1_lp / sqrt2, b2_lp / sqrt2],
            a: [a1_lp, a2_lp],
        };

        // =========================
        // HIGH-PASS DESIGN
        // =========================
        // For a Butterworth high-pass filter via bilinear transform the numerator becomes:
        // b0_hp_std = 1 / norm; b1_hp_std = -2 / norm; b2_hp_std = 1 / norm.
        let b0_hp_std = 1.0 / norm;
        let b1_hp_std = -2.0 / norm;
        let b2_hp_std = 1.0 / norm;
        // Denominator for high-pass is the same as for low-pass (canonical) BEFORE sign inversion.
        // After sign inversion, we have:
        let a1_hp = -a1_lp_std; // same as lowpass inverted value.
        let a2_hp = -a2_lp_std; // same as lowpass inverted value.
        // In many designs the high-pass section is implemented with unity gain.
        let highpass_filter1 = SecondOrderCoeffs {
            b: [b0_hp_std, b1_hp_std, b2_hp_std],
            a: [a1_hp, a2_hp],
        };
        // For a fourth–order LR high-pass, both cascaded sections are identical.
        let highpass_filter2 = highpass_filter1;

        Self {
            lowpass_filter1,
            lowpass_filter2,
            highpass_filter1,
            highpass_filter2,
        }
    }

    /// Returns all coefficients in 8.24 fixed–point format.
    ///
    /// The return value is a tuple of four arrays (one for each section) in the order:
    /// (lowpass_filter1, lowpass_filter2, highpass_filter1, highpass_filter2).
    /// Each array is of length 5 and the order of coefficients is:
    /// [b0, b1, b2, a1, a2].
    pub fn as_fixed(&self) -> ([u32; 5], [u32; 5], [u32; 5], [u32; 5]) {
        (
            self.lowpass_filter1.to_fixed(),
            self.lowpass_filter2.to_fixed(),
            self.highpass_filter1.to_fixed(),
            self.highpass_filter2.to_fixed(),
        )
    }
}
