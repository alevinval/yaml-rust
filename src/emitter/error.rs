use std::error::Error;
use std::fmt::Display;
use std::fmt::Error as FmtError;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Copy, Clone, Debug)]
pub enum EmitError {
    FmtError(FmtError),
    BadHashmapKey,
}

impl Error for EmitError {
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl Display for EmitError {
    fn fmt(&self, formatter: &mut Formatter) -> Result {
        match *self {
            EmitError::FmtError(ref err) => Display::fmt(err, formatter),
            EmitError::BadHashmapKey => formatter.write_str("bad hashmap key"),
        }
    }
}

impl From<FmtError> for EmitError {
    fn from(f: FmtError) -> Self {
        EmitError::FmtError(f)
    }
}
