#[derive(PartialEq, Eq, Debug)]
pub enum LuaError {
    /// Raised when the requested attribute is not found.
    GetAttrErr,
    /// Raised when set_attr is called on something other than a table.
    SetAttrErr,
    /// Raised when a conversion to int fails.
    IntConversionErr,
    /// Raised when a conversion to float fails.
    FloatConversionErr,
    /// Raised when a conversion to string fails.
    StringConversionErr,
    /// Raised when the called register is not a closure.
    NotAClosure,
    /// A generic error.
    Error(String),
}
