//! What a fit is allowed to produce.
//!
//! The reference can say what its AutoEQ is allowed to fit: which filter
//! types, how many, over what frequency, gain and Q, and it ships presets for
//! the devices it knows. These are the rules of the program, not of the
//! picture: the panel that sets them is one of several ways to set them.

use crate::equalizer::FilterType;

/// What a fit is allowed to produce.
///
/// A limit of zero means no limit, as in the reference.
#[derive(Clone, Debug, PartialEq)]
pub struct Constraints {
    /// Whether the fit may use peaking filters.
    pub allow_peaking: bool,
    /// Whether the fit may use low shelf filters.
    pub allow_low_shelf: bool,
    /// Whether the fit may use high shelf filters.
    pub allow_high_shelf: bool,
    /// How many filters the fit may produce; zero is no cap.
    pub max_filters: usize,
    /// The lowest frequency the fit may use; zero is no limit.
    pub freq_min: f64,
    /// The highest frequency the fit may use; zero is no limit.
    pub freq_max: f64,
    /// The lowest gain the fit may use; zero is no limit.
    pub gain_min: f64,
    /// The highest gain the fit may use; zero is no limit.
    pub gain_max: f64,
    /// The lowest Q the fit may use; zero is no limit.
    pub q_min: f64,
    /// The highest Q the fit may use; zero is no limit.
    pub q_max: f64,
    /// The band frequencies a graphic fit snaps to. When this is set, the fit
    /// places its filters on these bands rather than wherever the curve
    /// deviates.
    pub graphic_bands: Vec<f64>,
}

impl Default for Constraints {
    fn default() -> Self {
        Self::none()
    }
}

impl Constraints {
    /// No limits at all: peaking, low shelf and high shelf, anywhere.
    pub fn none() -> Self {
        Self {
            allow_peaking: true,
            allow_low_shelf: true,
            allow_high_shelf: true,
            max_filters: 0,
            freq_min: 0.0,
            freq_max: 0.0,
            gain_min: 0.0,
            gain_max: 0.0,
            q_min: 0.0,
            q_max: 0.0,
            graphic_bands: Vec::new(),
        }
    }

    /// One of the generic presets, by its identifier.
    ///
    /// These are the reference's own defaults. Its presets for particular
    /// devices are its data about those devices, and are not carried here.
    pub fn preset(id: &str) -> Option<Self> {
        PRESETS
            .iter()
            .find(|preset| preset.id == id)
            .map(|preset| preset.constraints())
    }

    /// Whether the fit may use this kind of filter.
    pub fn allows(&self, filter_type: FilterType) -> bool {
        match filter_type {
            FilterType::Peaking => self.allow_peaking,
            FilterType::LowShelf => self.allow_low_shelf,
            FilterType::HighShelf => self.allow_high_shelf,
        }
    }

    /// Whether a filter at this frequency is within the band.
    pub fn frequency_allowed(&self, hz: f64) -> bool {
        if self.freq_min > 0.0 && hz < self.freq_min {
            return false;
        }
        if self.freq_max > 0.0 && hz > self.freq_max {
            return false;
        }
        true
    }

    /// Whether a filter with this gain is within the bounds.
    pub fn gain_allowed(&self, gain: f64) -> bool {
        if self.gain_min != 0.0 && gain < self.gain_min {
            return false;
        }
        if self.gain_max != 0.0 && gain > self.gain_max {
            return false;
        }
        true
    }

    /// Whether a filter with this Q is within the bounds.
    pub fn q_allowed(&self, q: f64) -> bool {
        if self.q_min > 0.0 && q < self.q_min {
            return false;
        }
        if self.q_max > 0.0 && q > self.q_max {
            return false;
        }
        true
    }

    /// Whether the fit may still produce another filter.
    pub fn may_add(&self, produced: usize) -> bool {
        self.max_filters == 0 || produced < self.max_filters
    }
}

/// One of the presets the reference ships.
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub max_filters: usize,
    pub allow_peaking: bool,
    pub allow_low_shelf: bool,
    pub allow_high_shelf: bool,
    pub freq_min: f64,
    pub freq_max: f64,
    pub gain_min: f64,
    pub gain_max: f64,
    pub q_min: f64,
    pub q_max: f64,
    pub graphic_bands: &'static [f64],
}

impl Preset {
    /// The constraints this preset stands for.
    pub fn constraints(&self) -> Constraints {
        Constraints {
            allow_peaking: self.allow_peaking,
            allow_low_shelf: self.allow_low_shelf,
            allow_high_shelf: self.allow_high_shelf,
            max_filters: self.max_filters,
            freq_min: self.freq_min,
            freq_max: self.freq_max,
            gain_min: self.gain_min,
            gain_max: self.gain_max,
            q_min: self.q_min,
            q_max: self.q_max,
            graphic_bands: self.graphic_bands.to_vec(),
        }
    }
}

/// The presets the reference ships, in the order it lists them.
pub const PRESETS: [Preset; 3] = [
    Preset {
        id: "default",
        label: "Default",
        max_filters: 0,
        allow_peaking: true,
        allow_low_shelf: true,
        allow_high_shelf: true,
        freq_min: 20.0,
        freq_max: 20000.0,
        gain_min: 0.0,
        gain_max: 0.0,
        q_min: 0.1,
        q_max: 10.0,
        graphic_bands: &[],
    },
    Preset {
        id: "auto-eq",
        label: "Auto EQ",
        max_filters: 0,
        allow_peaking: true,
        allow_low_shelf: true,
        allow_high_shelf: true,
        freq_min: 20.0,
        freq_max: 6000.0,
        gain_min: 0.0,
        gain_max: 0.0,
        q_min: 0.1,
        q_max: 10.0,
        graphic_bands: &[],
    },
    Preset {
        id: "generic-10-band",
        label: "Generic 10-band graphic EQ",
        max_filters: 10,
        allow_peaking: true,
        allow_low_shelf: false,
        allow_high_shelf: false,
        freq_min: 0.0,
        freq_max: 0.0,
        gain_min: -10.0,
        gain_max: 10.0,
        q_min: 0.0,
        q_max: 1.4,
        graphic_bands: &[
            31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_presets_are_the_reference_s_own() {
        // The values come from the file the reference loads them from.
        let default = Constraints::preset("default").expect("the default preset exists");
        assert!(default.allow_peaking && default.allow_low_shelf && default.allow_high_shelf);
        assert_eq!(default.max_filters, 0, "no cap");
        assert_eq!((default.freq_min, default.freq_max), (20.0, 20000.0));
        assert_eq!(
            (default.gain_min, default.gain_max),
            (0.0, 0.0),
            "no gain limit"
        );
        assert_eq!((default.q_min, default.q_max), (0.1, 10.0));

        let graphic = Constraints::preset("generic-10-band").expect("the graphic preset exists");
        assert_eq!(graphic.max_filters, 10);
        assert!(!graphic.allow_low_shelf && !graphic.allow_high_shelf);
        assert_eq!(graphic.graphic_bands.len(), 10);
        assert_eq!(graphic.graphic_bands[0], 31.0);
        assert_eq!(graphic.q_max, 1.4);

        // Only the generic presets travel: the reference's presets for
        // particular devices are its data about those devices.
        assert_eq!(PRESETS.len(), 3, "only the generic presets");
        assert!(
            Constraints::preset("no-such-preset").is_none(),
            "an unknown preset should not resolve"
        );
    }

    #[test]
    fn a_zero_limit_means_no_limit() {
        let constraints = Constraints::none();
        assert!(constraints.frequency_allowed(20.0));
        assert!(constraints.frequency_allowed(20_000.0));
        assert!(constraints.gain_allowed(-40.0));
        assert!(constraints.gain_allowed(40.0));
        assert!(constraints.q_allowed(0.01));
        assert!(constraints.may_add(100), "no cap");
    }
}
