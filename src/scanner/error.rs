use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use super::Marker;

#[derive(Clone, PartialEq, Debug, Eq)]
pub struct ScanError {
    mark: Marker,
    info: String,
}

impl ScanError {
    pub fn new(loc: Marker, info: &str) -> ScanError {
        ScanError {
            mark: loc,
            info: info.to_owned(),
        }
    }

    pub fn marker(&self) -> &Marker {
        &self.mark
    }
}

impl Error for ScanError {
    fn description(&self) -> &str {
        self.info.as_ref()
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter) -> Result {
        write!(
            formatter,
            "{} at line {} column {}",
            self.info,
            self.mark.line,
            self.mark.col + 1 // col starts from 0
        )
    }
}
