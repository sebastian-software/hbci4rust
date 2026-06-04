pub const DATE_FORMAT: &str = "%Y-%m-%d";
pub const DATE_UNDEFINED: &str = "1999-01-01";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SepaKind {
    Pain001,
    Pain008,
    Camt052,
}
