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
    /// Parses a type name as Trino (or the SQLite stand-in) spells it. Never fails: text this
    /// crate does not model becomes [`PhysicalType::Other`].
    #[must_use]
    pub fn parse(s: &str) -> Self {
        let text = s.trim();
        // ASCII lower-casing keeps byte offsets, so `lower` and `text` index alike.
        let lower = text.to_ascii_lowercase();
        if let Some(inner) = lower
            .strip_prefix("array(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            let start = text.len() - inner.len() - 1;
            return Self::Array(Box::new(Self::parse(&text[start..start + inner.len()])));
        }
        Self::parse_scalar(&lower).unwrap_or_else(|| Self::Other(text.to_owned()))
    }

    fn parse_scalar(lower: &str) -> Option<Self> {
        let simple = match lower {
            "boolean" | "bool" => Some(Self::Boolean),
            "tinyint" => Some(Self::Tinyint),
            "smallint" => Some(Self::Smallint),
            "integer" | "int" => Some(Self::Integer),
            "bigint" => Some(Self::Bigint),
            "real" => Some(Self::Real),
            "double" | "double precision" => Some(Self::Double),
            "number" => Some(Self::Number),
            "varchar" | "text" => Some(Self::Varchar { length: None }),
            "date" => Some(Self::Date),
            _ => None,
        };
        if simple.is_some() {
            return simple;
        }
        if let Some(args) = call_args(lower, "decimal") {
            let (p, s) = args.split_once(',').unwrap_or((args, "0"));
            return Some(Self::Decimal {
                precision: p.trim().parse().ok()?,
                scale: s.trim().parse().ok()?,
            });
        }
        if let Some(args) = call_args(lower, "varchar") {
            return Some(Self::Varchar {
                length: Some(args.trim().parse().ok()?),
            });
        }
        if let Some(args) = call_args(lower, "char") {
            return Some(Self::Char {
                length: args.trim().parse().ok()?,
            });
        }
        let rest = lower.strip_prefix("timestamp")?;
        let (precision, rest) = match rest.strip_prefix('(') {
            Some(after) => {
                let (digits, tail) = after.split_once(')')?;
                (digits.trim().parse().ok()?, tail)
            }
            None => (3, rest),
        };
        let with_time_zone = match rest.trim() {
            "" | "without time zone" => false,
            "with time zone" => true,
            _ => return None,
        };
        Some(Self::Timestamp {
            precision,
            with_time_zone,
        })
    }

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

/// The syntactic parts of a decimal text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalParts<'a> {
    pub negative: bool,
    pub integer: &'a str,
    pub fraction: &'a str,
}

/// Splits decimal text `^-?[0-9]+(\.[0-9]+)?$`; `None` if `s` does not match.
///
/// The one implementation of this syntax: the printer's literal check and fixture conversion
/// share it. It only splits; precision rules are applied by the caller.
#[must_use]
pub fn split_decimal(s: &str) -> Option<DecimalParts<'_>> {
    let (negative, rest) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (integer, fraction) = match rest.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (rest, None),
    };
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if !digits(integer) || fraction.is_some_and(|f| !digits(f)) {
        return None;
    }
    Some(DecimalParts {
        negative,
        integer,
        fraction: fraction.unwrap_or(""),
    })
}

/// The text between `name(` and the final `)`, if `s` has that shape.
fn call_args<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    s.strip_prefix(name)?.strip_prefix('(')?.strip_suffix(')')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_type_parse_table() {
        use PhysicalType as P;
        let cases: Vec<(&str, P)> = vec![
            (
                "decimal(10,2)",
                P::Decimal {
                    precision: 10,
                    scale: 2,
                },
            ),
            ("varchar", P::Varchar { length: None }),
            ("varchar(255)", P::Varchar { length: Some(255) }),
            ("char(3)", P::Char { length: 3 }),
            (
                "timestamp(3)",
                P::Timestamp {
                    precision: 3,
                    with_time_zone: false,
                },
            ),
            (
                "timestamp(6) with time zone",
                P::Timestamp {
                    precision: 6,
                    with_time_zone: true,
                },
            ),
            (
                "timestamp",
                P::Timestamp {
                    precision: 3,
                    with_time_zone: false,
                },
            ),
            (
                "timestamp with time zone",
                P::Timestamp {
                    precision: 3,
                    with_time_zone: true,
                },
            ),
            (
                "array(varchar)",
                P::Array(Box::new(P::Varchar { length: None })),
            ),
            (
                "array(array(integer))",
                P::Array(Box::new(P::Array(Box::new(P::Integer)))),
            ),
            ("number", P::Number),
            ("text", P::Varchar { length: None }),
            ("int", P::Integer),
            ("TINYINT", P::Tinyint),
            ("  BigInt ", P::Bigint),
            ("boolean", P::Boolean),
            ("date", P::Date),
            (
                "map(varchar,integer)",
                P::Other("map(varchar,integer)".to_owned()),
            ),
        ];
        for (text, expected) in cases {
            assert_eq!(PhysicalType::parse(text), expected, "parse({text:?})");
            let rendered = expected.render();
            assert_eq!(
                PhysicalType::parse(&rendered),
                expected,
                "render/parse round trip of {rendered:?}"
            );
        }
    }

    #[test]
    fn split_decimal_rules() {
        assert_eq!(
            split_decimal("1.50"),
            Some(DecimalParts {
                negative: false,
                integer: "1",
                fraction: "50"
            })
        );
        assert_eq!(
            split_decimal("-0"),
            Some(DecimalParts {
                negative: true,
                integer: "0",
                fraction: ""
            })
        );
        assert!(split_decimal("007").is_some());
        for bad in [
            "1e5", "+1", "1.", ".5", "", "-", "1.2.3", " 1", "1 ", "\u{0661}",
        ] {
            assert_eq!(split_decimal(bad), None, "split_decimal({bad:?})");
        }
    }
}
