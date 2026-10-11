//! ISO 226 normalization and fractional-octave smoothing.

use crate::data::FRPoint;
use std::error::Error;
use std::fmt;

const ISO_FREQUENCIES: [f64; 29] = [
    20.0, 25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0,
    500.0, 630.0, 800.0, 1000.0, 1250.0, 1600.0, 2000.0, 2500.0, 3150.0, 4000.0, 5000.0, 6300.0,
    8000.0, 10000.0, 12500.0,
];

const ISO_A_F: [f64; 29] = [
    0.532, 0.506, 0.48, 0.455, 0.432, 0.409, 0.387, 0.367, 0.349, 0.33, 0.315, 0.301, 0.288, 0.276,
    0.267, 0.259, 0.253, 0.25, 0.246, 0.244, 0.243, 0.243, 0.243, 0.242, 0.242, 0.245, 0.254,
    0.271, 0.301,
];

const ISO_L_U: [f64; 29] = [
    -31.6, -27.2, -23.0, -19.1, -15.9, -13.0, -10.3, -8.1, -6.2, -4.5, -3.1, -2.0, -1.1, -0.4, 0.0,
    0.3, 0.5, 0.0, -2.7, -4.1, -1.0, 1.7, 2.5, 1.2, -2.1, -7.1, -11.2, -10.7, -3.1,
];

const ISO_T_F: [f64; 29] = [
    78.5, 68.7, 59.5, 51.1, 44.0, 37.5, 31.5, 26.5, 22.1, 17.9, 14.4, 11.4, 8.6, 6.2, 4.4, 3.0,
    2.2, 2.4, 3.5, 1.7, -1.3, -4.2, -6.0, -5.4, -1.5, 6.0, 12.6, 13.9, 12.3,
];

// ISO 226 loudness calculation correction used by the original graph. Its
// indices correspond to 1/48-octave bins starting at 19.4806 Hz.
const FREE_FIELD: [f64; 480] = [
    -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000,
    -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000,
    -7.0000, -6.9275, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9104,
    -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9033, -7.0000, -7.0000, -7.0000, -7.0000,
    -7.0000, -7.0000, -7.0000, -6.9114, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9344, -7.0000, -7.0000, -7.0000, -7.0000, -7.0000, -6.9760, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9550, -7.0000, -7.0000, -7.0000,
    -7.0000, -7.0000, -7.0000, -6.9710, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000,
    -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.9000, -6.8476, -6.8000, -6.8000, -6.7614,
    -6.6605, -6.6000, -6.5630, -6.5000, -6.4713, -6.3775, -6.3000, -6.2937, -6.2038, -6.2000,
    -6.1059, -6.1000, -6.0137, -6.0000, -5.9271, -5.9000, -5.8456, -5.8000, -5.7496, -5.7000,
    -5.7000, -5.7000, -5.7000, -5.6837, -5.6000, -5.6000, -5.6000, -5.6000, -5.5983, -5.5154,
    -5.5000, -5.5000, -5.4252, -5.4000, -5.4000, -5.3470, -5.3000, -5.3000, -5.2513, -5.2000,
    -5.1659, -5.1000, -5.1000, -5.0771, -5.0000, -5.0000, -5.0000, -4.9000, -4.9000, -4.8103,
    -4.8000, -4.8000, -4.7326, -4.7000, -4.7000, -4.6433, -4.6000, -4.6000, -4.5554, -4.5000,
    -4.4738, -4.4000, -4.3766, -4.2851, -4.2000, -4.1962, -4.0989, -4.0031, -3.9087, -3.8155,
    -3.7238, -3.6243, -3.5351, -3.4383, -3.3430, -3.2490, -3.2000, -3.1568, -3.0668, -3.0000,
    -3.0000, -3.0000, -2.9879, -2.9000, -2.9000, -2.9000, -2.9921, -3.0000, -3.0000, -3.0000,
    -3.0000, -3.0666, -3.1000, -3.1000, -3.1000, -3.1459, -3.2000, -3.2000, -3.2320, -3.3000,
    -3.3239, -3.4000, -3.4000, -3.4073, -3.5000, -3.5000, -3.5000, -3.5000, -3.5000, -3.4239,
    -3.4000, -3.4000, -3.3396, -3.3000, -3.2486, -3.2000, -3.2000, -3.1651, -3.1000, -3.0782,
    -2.9801, -2.8877, -2.7924, -2.6984, -2.6015, -2.3184, -1.9485, -1.5778, -1.1964, -0.8903,
    -0.5344, -0.1539, 0.3316, 0.9083, 1.4305, 1.9369, 2.5105, 3.0759, 3.6024, 4.0027, 4.4847,
    5.0482, 5.5152, 5.8994, 6.2776, 6.7381, 7.1303, 7.5168, 7.8858, 8.2730, 8.6547, 8.9731, 9.2596,
    9.5420, 9.7857, 10.0111, 10.2325, 10.3532, 10.5220, 10.6000, 10.6000, 10.6000, 10.6000,
    10.5044, 10.4100, 10.3145, 10.2205, 10.1255, 10.0318, 9.9373, 9.7840, 9.6459, 9.4536, 9.2578,
    9.1234, 8.9670, 8.8736, 8.7552, 8.5660, 8.3879, 8.2881, 8.0958, 7.9064, 7.8099, 7.6287, 7.5201,
    7.3477, 7.2307, 7.0709, 6.9399, 6.7916, 6.6514, 6.5552, 6.4604, 6.3670, 6.2718, 6.1766, 6.0812,
    5.9743, 5.7916, 5.6975, 5.6020, 5.5078, 5.3247, 5.0547, 4.7686, 4.4154, 4.1009, 3.9385, 3.7344,
    3.3998, 3.0163, 2.6382, 2.2957, 1.9799, 1.6248, 1.3404, 1.0424, 0.6740, 0.3851, 0.0061,
    -0.4693, -0.8516, -1.2304, -1.5338, -1.8916, -2.2698, -2.6502, -3.0290, -3.3545, -3.5925,
    -3.8657, -4.2083, -4.4624, -4.6516, -4.8415, -5.0151, -5.0893, -5.0000, -5.0000, -5.0000,
    -4.9106, -4.8156, -4.7213, -4.6260, -4.3943, -4.1735, -3.9839, -3.7943, -3.6046, -3.4149,
    -3.1878, -2.9033, -2.6460, -2.4349, -2.1491, -1.8541, -1.4741, -1.0959, -0.8119, -0.4357,
    -0.1439, 0.1418, 0.4251, 0.7093, 1.0593, 1.3192, 1.4541, 1.5493, 1.6437, 1.7000, 1.7336,
    1.8000, 1.8000, 1.8000, 1.8000, 1.7926, 1.7000, 1.7000, 1.6079, 1.5133, 1.5000, 1.4237, 1.1863,
    0.9680, 0.7786, 0.4219, -0.0520, -0.5701, -1.1788, -1.8437, -2.5366, -3.2958, -4.1103, -5.0995,
    -5.7632, -6.4349, -7.2856, -7.8593, -9.9000,
];

const LOG_10_OVER_10: f64 = std::f64::consts::LN_10 / 10.0;

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessingError {
    InvalidFrequencyGrid,
    InvalidPhonLevel,
    InvalidSmoothingLevel,
    InvalidTargetFrequency,
    NonFiniteData,
    ParameterMismatch,
    SingularSmoothingMatrix,
}

impl fmt::Display for ProcessingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrequencyGrid => {
                write!(
                    formatter,
                    "frequencies must be finite, positive, and strictly increasing"
                )
            }
            Self::InvalidPhonLevel => {
                write!(formatter, "target phon level must be between 0 and 100")
            }
            Self::InvalidSmoothingLevel => {
                write!(formatter, "smoothing level must be between 0 and 12")
            }
            Self::InvalidTargetFrequency => {
                write!(formatter, "target frequency must be finite and positive")
            }
            Self::NonFiniteData => write!(formatter, "curve values must be finite"),
            Self::ParameterMismatch => {
                write!(formatter, "processing parameters do not match the curve")
            }
            Self::SingularSmoothingMatrix => write!(formatter, "smoothing matrix is singular"),
        }
    }
}

impl Error for ProcessingError {}

#[derive(Clone, Copy, Debug)]
struct IsoBand {
    a: f64,
    k: f64,
    c: f64,
    free_field: f64,
}

/// Cached ISO 226 coefficients for a fixed frequency grid.
#[derive(Clone, Debug)]
pub struct NormParams {
    frequencies: Vec<f32>,
    bands: Vec<IsoBand>,
}

impl NormParams {
    pub fn new(frequencies: &[f32]) -> Result<Self, ProcessingError> {
        validate_frequencies(frequencies)?;
        let bands = frequencies
            .iter()
            .map(|&frequency| iso_band(frequency as f64, frequencies.len()))
            .collect();
        Ok(Self {
            frequencies: frequencies.to_vec(),
            bands,
        })
    }

    pub fn for_curve(curve: &[FRPoint]) -> Result<Self, ProcessingError> {
        Self::new(&curve.iter().map(|point| point.hz).collect::<Vec<_>>())
    }
}

/// Precomputed fractional-octave smoothing matrix for a frequency grid and level.
#[derive(Clone, Debug)]
pub struct SmoothingParams {
    frequencies: Vec<f32>,
    level: usize,
    g: [Vec<f64>; 3],
    diagonal: Vec<f64>,
    lower: [Vec<f64>; 2],
    d_squared: Vec<f64>,
}

/// Average aligned response curves in linear amplitude, then convert back to dB.
pub fn average_curves(curves: &[&[FRPoint]]) -> Result<Vec<FRPoint>, ProcessingError> {
    let first = curves
        .first()
        .ok_or(ProcessingError::InvalidFrequencyGrid)?;
    for curve in curves {
        validate_curve(curve)?;
        if curve.len() != first.len()
            || curve
                .iter()
                .zip(*first)
                .any(|(point, reference)| point.hz != reference.hz)
        {
            return Err(ProcessingError::ParameterMismatch);
        }
    }
    let mut result = Vec::with_capacity(first.len());
    for index in 0..first.len() {
        let mean_amplitude = curves
            .iter()
            .map(|curve| 10.0_f64.powf(curve[index].db as f64 / 20.0))
            .sum::<f64>()
            / curves.len() as f64;
        if !mean_amplitude.is_finite() || mean_amplitude <= 0.0 {
            return Err(ProcessingError::NonFiniteData);
        }
        result.push(FRPoint {
            hz: first[index].hz,
            db: (20.0 * mean_amplitude.log10()) as f32,
        });
    }
    Ok(result)
}

impl SmoothingParams {
    pub fn new(frequencies: &[f32], level: usize) -> Result<Self, ProcessingError> {
        validate_frequencies(frequencies)?;
        if level > 12 {
            return Err(ProcessingError::InvalidSmoothingLevel);
        }

        let mut params = Self {
            frequencies: frequencies.to_vec(),
            level,
            g: [Vec::new(), Vec::new(), Vec::new()],
            diagonal: Vec::new(),
            lower: [Vec::new(), Vec::new()],
            d_squared: Vec::new(),
        };
        if level == 0 || frequencies.len() < 3 {
            return Ok(params);
        }

        params.prepare()?;
        Ok(params)
    }

    fn prepare(&mut self) -> Result<(), ProcessingError> {
        let x: Vec<f64> = self
            .frequencies
            .iter()
            .map(|frequency| (*frequency as f64).ln())
            .collect();
        let h: Vec<f64> = x.windows(2).map(|pair| pair[1] - pair[0]).collect();
        let reciprocal: Vec<f64> = h.iter().map(|width| 1.0 / width).collect();
        let n = self.frequencies.len() - 2;

        self.g = [
            reciprocal[..n].to_vec(),
            (0..n)
                .map(|index| -(reciprocal[index] + reciprocal[index + 1]))
                .collect(),
            reciprocal[1..n + 1].to_vec(),
        ];

        let scale = self.level as f64 * 0.01;
        let d: Vec<f64> = (0..self.frequencies.len())
            .map(|index| {
                scale
                    * (1.0 / 80.0_f64).powf((index as f64 / self.frequencies.len() as f64).powi(2))
            })
            .collect();
        let d_squared: Vec<f64> = d.iter().map(|value| value * value).collect();
        let dg: [Vec<f64>; 3] = std::array::from_fn(|row| {
            self.g[row]
                .iter()
                .enumerate()
                .map(|(index, value)| value * d[index + row])
                .collect()
        });

        let h6: Vec<f64> = h.iter().map(|width| width / 6.0).collect();
        let mut matrix = [
            (0..n)
                .map(|index| 2.0 * (h6[index] + h6[index + 1]))
                .collect::<Vec<_>>(),
            (0..n.saturating_sub(1))
                .map(|index| h6[index + 1])
                .collect::<Vec<_>>(),
            (0..n.saturating_sub(2)).map(|_| 0.0).collect::<Vec<_>>(),
        ];
        for offset in 0..3 {
            for row in 0..3 - offset {
                for column in 0..n - offset {
                    matrix[offset][column] += dg[row][column + offset] * dg[row + offset][column];
                }
            }
        }

        self.diagonal.push(matrix[0][0]);
        self.lower = [Vec::with_capacity(n - 1), Vec::with_capacity(n - 2)];
        if n > 1 {
            self.lower[0].push(matrix[1][0] / self.diagonal[0]);
        }
        if n > 2 {
            self.lower[1].push(matrix[2][0] / self.diagonal[0]);
        }
        for column in 1..n {
            let mut pivot = matrix[0][column];
            pivot -= self.diagonal[column - 1] * self.lower[0][column - 1].powi(2);
            if column >= 2 {
                pivot -= self.diagonal[column - 2] * self.lower[1][column - 2].powi(2);
            }
            if !pivot.is_finite() || pivot <= f64::EPSILON {
                return Err(ProcessingError::SingularSmoothingMatrix);
            }
            self.diagonal.push(pivot);

            let first = if column < matrix[1].len() {
                let mut value = matrix[1][column];
                if column > 0 {
                    value -= self.diagonal[column - 1]
                        * self.lower[0][column - 1]
                        * self.lower[1][column - 1];
                }
                value / pivot
            } else {
                0.0
            };
            let second = if column < matrix[2].len() {
                matrix[2][column] / pivot
            } else {
                0.0
            };
            self.lower[0].push(first);
            if column < matrix[2].len() {
                self.lower[1].push(second);
            }
        }
        self.d_squared = d_squared;
        Ok(())
    }
}

/// Compute the dB offset that brings a curve to the requested ISO 226 phon level.
pub fn normalize(
    curve: &[FRPoint],
    target_phon: f32,
    params: &NormParams,
) -> Result<f32, ProcessingError> {
    validate_curve(curve)?;
    if !(0.0..=100.0).contains(&target_phon) {
        return Err(ProcessingError::InvalidPhonLevel);
    }
    if curve.len() != params.frequencies.len()
        || curve
            .iter()
            .zip(&params.frequencies)
            .any(|(point, frequency)| point.hz != *frequency)
    {
        return Err(ProcessingError::ParameterMismatch);
    }

    let target = target_phon as f64 * LOG_10_OVER_10;
    let mut offset = 0.0;
    for _ in 0..100 {
        let (loudness, derivative) = loudness_and_derivative(curve, &params.bands, offset);
        if !loudness.is_finite() || !derivative.is_finite() || loudness <= 0.0 || derivative <= 0.0
        {
            return Err(ProcessingError::NonFiniteData);
        }
        let step = (loudness.ln() - target) * loudness / derivative;
        offset -= step;
        if !offset.is_finite() {
            return Err(ProcessingError::NonFiniteData);
        }
        if step.abs() <= 0.01 {
            return Ok(offset as f32);
        }
    }
    Err(ProcessingError::NonFiniteData)
}

/// Return the normalization offset that places the channel amplitude-average at
/// 60 dB at the first measurement frequency above `target_hz`.
pub fn normalize_by_hz(channels: &[&[FRPoint]], target_hz: f32) -> Result<f32, ProcessingError> {
    if !target_hz.is_finite() || target_hz <= 0.0 {
        return Err(ProcessingError::InvalidTargetFrequency);
    }
    if channels.is_empty() {
        return Err(ProcessingError::InvalidFrequencyGrid);
    }
    for channel in channels {
        validate_curve(channel)?;
    }

    let reference = channels[0];
    let index = reference
        .partition_point(|point| point.hz <= target_hz)
        .min(reference.len() - 1);
    let frequency = reference[index].hz;
    if channels
        .iter()
        .any(|channel| channel.len() != reference.len() || channel[index].hz != frequency)
    {
        return Err(ProcessingError::ParameterMismatch);
    }

    let amplitude_mean = channels
        .iter()
        .map(|channel| 10.0_f64.powf(channel[index].db as f64 / 20.0))
        .sum::<f64>()
        / channels.len() as f64;
    if !amplitude_mean.is_finite() || amplitude_mean <= 0.0 {
        return Err(ProcessingError::NonFiniteData);
    }
    Ok((60.0 - 20.0 * amplitude_mean.log10()) as f32)
}

/// Smooth dB values using the original graph's log-frequency LDL algorithm.
pub fn smooth(
    curve: &[FRPoint],
    params: &SmoothingParams,
) -> Result<Vec<FRPoint>, ProcessingError> {
    validate_curve(curve)?;
    if curve.len() != params.frequencies.len()
        || curve
            .iter()
            .zip(&params.frequencies)
            .any(|(point, frequency)| point.hz != *frequency)
    {
        return Err(ProcessingError::ParameterMismatch);
    }
    if params.level == 0 || curve.len() < 3 {
        return Ok(curve.to_vec());
    }

    let n = curve.len() - 2;
    let mut gradient = vec![0.0; n];
    for row in 0..3 {
        for (index, coefficient) in params.g[row].iter().enumerate() {
            gradient[index] += coefficient * curve[index + row].db as f64;
        }
    }

    for index in 0..n {
        let value = gradient[index];
        for offset in 1..=2 {
            if index + offset < n {
                gradient[index + offset] -= params.lower[offset - 1][index] * value;
            }
        }
        gradient[index] /= params.diagonal[index];
    }
    for index in (0..n).rev() {
        let value = gradient[index];
        for offset in 1..=2 {
            if index >= offset {
                gradient[index - offset] -= params.lower[offset - 1][index - offset] * value;
            }
        }
    }

    let mut output = curve.to_vec();
    for row in 0..3 {
        for (index, coefficient) in params.g[row].iter().enumerate() {
            output[index + row].db -=
                (coefficient * params.d_squared[index + row] * gradient[index]) as f32;
        }
    }
    Ok(output)
}

fn validate_frequencies(frequencies: &[f32]) -> Result<(), ProcessingError> {
    if frequencies.is_empty()
        || frequencies
            .iter()
            .any(|frequency| !frequency.is_finite() || *frequency <= 0.0)
        || frequencies.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(ProcessingError::InvalidFrequencyGrid);
    }
    Ok(())
}

fn validate_curve(curve: &[FRPoint]) -> Result<(), ProcessingError> {
    validate_frequencies(&curve.iter().map(|point| point.hz).collect::<Vec<_>>())?;
    if curve.iter().any(|point| !point.db.is_finite()) {
        return Err(ProcessingError::NonFiniteData);
    }
    Ok(())
}

fn iso_band(frequency: f64, count: usize) -> IsoBand {
    let upper = ISO_FREQUENCIES.partition_point(|anchor| *anchor < frequency);
    let (lower, upper) = if upper == 0 {
        (0, 0)
    } else if upper == ISO_FREQUENCIES.len() {
        (upper - 1, upper - 1)
    } else {
        (upper - 1, upper)
    };
    let fraction = if lower == upper {
        0.0
    } else {
        let log_frequency = frequency.ln();
        (log_frequency - ISO_FREQUENCIES[lower].ln())
            / (ISO_FREQUENCIES[upper].ln() - ISO_FREQUENCIES[lower].ln())
    };
    let interpolate =
        |values: &[f64; 29]| values[lower] + fraction * (values[upper] - values[lower]);
    let a = interpolate(&ISO_A_F);
    let m = a * (4.0_f64.log10() - 10.0 + interpolate(&ISO_L_U) / 10.0);
    let k = 0.005076 / 10.0_f64.powf(m) - 10.0_f64.powf(a * interpolate(&ISO_T_F) / 10.0);
    let c = 10.0_f64.powf(9.4 + 4.0 * m) / count as f64;
    let free_field_index = (0.5 + 48.0 * (frequency / 19.4806).log2())
        .floor()
        .clamp(0.0, (FREE_FIELD.len() - 1) as f64) as usize;
    IsoBand {
        a,
        k,
        c,
        free_field: FREE_FIELD[free_field_index],
    }
}

fn loudness_and_derivative(curve: &[FRPoint], bands: &[IsoBand], offset: f64) -> (f64, f64) {
    curve
        .iter()
        .zip(bands)
        .fold((0.0, 0.0), |(loudness, derivative), (point, band)| {
            let level = (LOG_10_OVER_10 * (point.db as f64 + offset - band.free_field)).exp();
            let transformed = band.k + level.powf(band.a);
            let partial = LOG_10_OVER_10
                * level
                * band.a
                * level.powf(band.a - 1.0)
                * band.c
                * 4.0
                * transformed.powi(3);
            (
                loudness + band.c * transformed.powi(4),
                derivative + partial,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve(frequencies: &[f32], levels: &[f32]) -> Vec<FRPoint> {
        frequencies
            .iter()
            .zip(levels)
            .map(|(&hz, &db)| FRPoint { hz, db })
            .collect()
    }

    #[test]
    fn smoothing_level_zero_is_a_passthrough() {
        let input = curve(&[100.0, 200.0, 400.0], &[1.0, 8.0, -2.0]);
        let params = SmoothingParams::new(&[100.0, 200.0, 400.0], 0).unwrap();
        let output = smooth(&input, &params).unwrap();
        assert!(output
            .iter()
            .zip(&input)
            .all(|(actual, original)| actual.hz == original.hz && actual.db == original.db));
    }

    #[test]
    fn smoothing_preserves_a_flat_curve_and_reduces_a_peak() {
        let frequencies: Vec<f32> = (0..101)
            .map(|index| 20.0 * 1000.0_f32.powf(index as f32 / 100.0))
            .collect();
        let flat = vec![60.0; frequencies.len()];
        let params = SmoothingParams::new(&frequencies, 6).unwrap();
        let flat_output = smooth(&curve(&frequencies, &flat), &params).unwrap();
        assert!(flat_output
            .iter()
            .all(|point| (point.db - 60.0).abs() < 0.01));

        let mut peak = flat;
        peak[50] += 20.0;
        let peak_output = smooth(&curve(&frequencies, &peak), &params).unwrap();
        assert!((peak_output[48].db - 62.3655).abs() < 0.01);
        assert!((peak_output[50].db - 66.6412).abs() < 0.01);
        assert!((peak_output[51].db - 64.6127).abs() < 0.01);
    }

    #[test]
    fn phon_normalization_converges_to_the_requested_loudness() {
        let input = curve(&[100.0, 500.0, 1000.0, 5000.0], &[45.0, 55.0, 60.0, 50.0]);
        let params = NormParams::for_curve(&input).unwrap();
        let offset = normalize(&input, 60.0, &params).unwrap();
        assert!((offset - 1.7028).abs() < 0.01);
        let shifted: Vec<FRPoint> = input
            .iter()
            .map(|point| FRPoint {
                hz: point.hz,
                db: point.db + offset,
            })
            .collect();
        let (loudness, _) = loudness_and_derivative(&shifted, &params.bands, 0.0);
        assert!((loudness.ln() / LOG_10_OVER_10 - 60.0).abs() <= 0.01);
    }

    #[test]
    fn frequency_normalization_uses_the_original_bisect_and_amplitude_average() {
        let left = curve(&[100.0, 500.0, 1000.0], &[40.0, 50.0, 60.0]);
        let right = curve(&[100.0, 500.0, 1000.0], &[40.0, 54.0, 40.0]);
        let offset = normalize_by_hz(&[&left, &right], 500.0).unwrap();
        let mean = 10.0_f64.powf(60.0 / 20.0) + 10.0_f64.powf(40.0 / 20.0);
        let expected = 60.0 - 20.0 * (mean / 2.0).log10();
        assert!((offset as f64 - expected).abs() < 0.001);
    }

    #[test]
    fn channel_average_uses_linear_amplitude() {
        let left = curve(&[100.0, 500.0], &[40.0, 50.0]);
        let right = curve(&[100.0, 500.0], &[40.0, 54.0]);
        let average = average_curves(&[&left, &right]).unwrap();
        assert!((average[0].db - 40.0).abs() < 0.001);
        let expected =
            20.0 * ((10.0_f64.powf(50.0 / 20.0) + 10.0_f64.powf(54.0 / 20.0)) / 2.0).log10();
        assert!((average[1].db as f64 - expected).abs() < 0.001);
    }

    #[test]
    fn invalid_frequency_grids_are_rejected() {
        assert_eq!(
            SmoothingParams::new(&[100.0, 100.0, 200.0], 3).unwrap_err(),
            ProcessingError::InvalidFrequencyGrid
        );
    }
}
