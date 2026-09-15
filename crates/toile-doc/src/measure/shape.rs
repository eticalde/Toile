use serde::{Deserialize, Serialize};

use crate::DocError;

/// What a body is generated from besides the tape: who the person is, before
/// how much they measure.
///
/// Every field is in the person's own units and never in a body model's: age
/// in years, the rest on their own `0.0` to `1.0` scales. Turning years into a
/// model's age parameter belongs to the model, and a file that stored the
/// model's numbers would change meaning the day the model did. Stature is not
/// here: it is the `estatura` measurement.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodyShape {
    /// `0.0` male, `1.0` female, and a blend of the two in between.
    pub sex: f64,
    /// The age, in years.
    pub age_years: f64,
    /// `0.0` slightest, `1.0` heaviest. Build rather than weight, so that no
    /// reader of the file takes it for kilograms.
    pub build: f64,
    /// `0.0` least muscular, `1.0` most.
    pub muscle: f64,
    /// `0.0` typical proportions, `1.0` atypical.
    pub proportions: f64,
}

impl BodyShape {
    /// Refuses a scale JSON cannot spell, which a file would write as `null`
    /// and refuse on the way back in.
    pub(crate) fn check(&self) -> Result<(), DocError> {
        let scales = [
            self.sex,
            self.age_years,
            self.build,
            self.muscle,
            self.proportions,
        ];
        if scales.iter().copied().all(f64::is_finite) {
            Ok(())
        } else {
            Err(DocError::NonFinite("phenotype".to_owned()))
        }
    }
}

impl Default for BodyShape {
    /// A 25-year-old adult at the midpoint of every other scale, which is the
    /// body a new mannequin starts as.
    fn default() -> BodyShape {
        BodyShape {
            sex: 0.5,
            age_years: 25.0,
            build: 0.5,
            muscle: 0.5,
            proportions: 0.5,
        }
    }
}
