//! Values and physical types exchanged with an engine.

/// A physical (engine side) column type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalType {
    Boolean,
    Tinyint,
    Smallint,
    Integer,
    Bigint,
    Real,
    Double,
    Decimal {
        precision: u8,
        scale: u8,
    },
    /// Trino `NUMBER`: an unconstrained PostgreSQL `numeric`.
    Number,
    Varchar {
        length: Option<u32>,
    },
    Char {
        length: u32,
    },
    Date,
    Timestamp {
        precision: u8,
        with_time_zone: bool,
    },
    Array(Box<PhysicalType>),
    /// Anything this crate does not model; keeps the original type text.
    Other(String),
}

impl PhysicalType {
    /// Canonical lower-case Trino spelling.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Boolean => "boolean".to_owned(),
            Self::Tinyint => "tinyint".to_owned(),
            Self::Smallint => "smallint".to_owned(),
            Self::Integer => "integer".to_owned(),
            Self::Bigint => "bigint".to_owned(),
            Self::Real => "real".to_owned(),
            Self::Double => "double".to_owned(),
            Self::Decimal { precision, scale } => format!("decimal({precision},{scale})"),
            Self::Number => "number".to_owned(),
            Self::Varchar { length: None } => "varchar".to_owned(),
            Self::Varchar { length: Some(n) } => format!("varchar({n})"),
            Self::Char { length } => format!("char({length})"),
            Self::Date => "date".to_owned(),
            Self::Timestamp {
                precision,
                with_time_zone: false,
            } => format!("timestamp({precision})"),
            Self::Timestamp {
                precision,
                with_time_zone: true,
            } => format!("timestamp({precision}) with time zone"),
            Self::Array(inner) => format!("array({})", inner.render()),
            Self::Other(s) => s.clone(),
        }
    }
}

/// A column as described by the engine's metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalColumn {
    pub name: String,
    pub physical: PhysicalType,
    pub nullable: bool,
}

/// A column of a query result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub physical: PhysicalType,
}

/// A value of a query result.
///
/// `Decimal`, `Date`, `Timestamp` and `TimestampTz` stay canonical strings; numeric
/// comparison is a later phase's business.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Long(i64),
    Double(f64),
    Decimal(String),
    String(String),
    Date(String),
    Timestamp(String),
    TimestampTz(String),
    Array(Vec<Value>),
    /// A value of a type the decoder does not understand; keeps the raw JSON.
    Unsupported(serde_json::Value),
}
