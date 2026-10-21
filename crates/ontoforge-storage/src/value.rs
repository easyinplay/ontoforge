//! Property values of the edit store and their one JSON encoding (D-10, D-11).
//!
//! `to_json` is the only encoding of a value in the edit log and in the history snapshots, and it
//! is the same encoding the typed edit tables hold (ADR-0024), so a replay needs no type
//! inference:
//!
//! - boolean, integers and finite doubles as JSON booleans and numbers,
//! - `decimal` as canonical decimal text with exactly the scale of the property (`"12.50"`);
//!   input is a decimal string, a JSON number is accepted only as an integer,
//! - `date` as `YYYY-MM-DD`,
//! - `timestamp` as UTC with milliseconds, `YYYY-MM-DDTHH:MM:SS.fffZ`; input needs a UTC offset
//!   (`Z` or `+HH:MM`), is converted to UTC, and more than three fractional digits are refused
//!   instead of truncated,
//! - `array` as a JSON array of its elements, `geopoint` as `{"lat": .., "lon": ..}`.

use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use ontoforge_engine::sqlite::fixture::{normalize_decimal, parse_date};
use ontoforge_expr::rowpred::Scalar;
use ontoforge_schema::ir::{BaseType, PropertyDef};
use serde_json::{Map, Number, Value};

/// A value of one property, in canonical form once it passed [`PropertyValue::canonical`] or came
/// out of [`PropertyValue::from_json`].
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Null,
    Bool(bool),
    Int(i64),
    Double(f64),
    /// Canonical decimal text with exactly the scale of the property.
    Decimal(String),
    String(String),
    /// `YYYY-MM-DD`.
    Date(String),
    /// UTC with milliseconds: `YYYY-MM-DDTHH:MM:SS.fffZ`.
    Timestamp(String),
    Array(Vec<PropertyValue>),
    Geopoint {
        lat: f64,
        lon: f64,
    },
}

/// Why a value does not fit its property.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ValueError(pub String);

fn fail<T>(reason: impl Into<String>) -> Result<T, ValueError> {
    Err(ValueError(reason.into()))
}

impl PropertyValue {
    /// Checks type, range, precision and canonical text against `def`.
    pub fn check(&self, def: &PropertyDef) -> Result<(), ValueError> {
        self.canonical(def).map(|_| ())
    }

    /// The canonical form of the value for `def`: decimals padded to the scale, timestamps in
    /// UTC with milliseconds, a `float` rounded to the nearest `f32`. A value that does not fit
    /// is an error and is never repaired by rounding or truncating.
    pub fn canonical(&self, def: &PropertyDef) -> Result<Self, ValueError> {
        if matches!(self, Self::Null) {
            return if def.required {
                fail("a required property cannot be null")
            } else {
                Ok(Self::Null)
            };
        }
        canonical_as(self, def, def.base_type)
    }

    /// The JSON encoding used by the edit log and the history.
    pub fn to_json(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(b) => Value::Bool(*b),
            Self::Int(i) => Value::from(*i),
            Self::Double(d) => Number::from_f64(*d).map_or(Value::Null, Value::Number),
            Self::Decimal(s) | Self::String(s) | Self::Date(s) | Self::Timestamp(s) => {
                Value::String(s.clone())
            }
            Self::Array(items) => Value::Array(items.iter().map(Self::to_json).collect()),
            Self::Geopoint { lat, lon } => {
                let mut point = Map::new();
                point.insert("lat".to_owned(), Self::Double(*lat).to_json());
                point.insert("lon".to_owned(), Self::Double(*lon).to_json());
                Value::Object(point)
            }
        }
    }

    /// Reads a value written by [`PropertyValue::to_json`] (or the equivalent client JSON) and
    /// returns it in canonical form.
    pub fn from_json(def: &PropertyDef, v: &Value) -> Result<Self, ValueError> {
        from_json_as(def, def.base_type, v)?.canonical(def)
    }

    /// Reads one CSV cell of a property of type `def` and returns the value in canonical form.
    /// An array is a JSON array text, a geopoint is `lat,lon`, a boolean is `true` or `false`.
    /// The cell is taken as it is: nothing is trimmed, and an empty cell is the caller's to treat
    /// as "not set".
    pub fn from_text(def: &PropertyDef, cell: &str) -> Result<Self, ValueError> {
        use PropertyValue as V;
        let value = match def.base_type {
            BaseType::Byte | BaseType::Short | BaseType::Integer | BaseType::Long => V::Int(
                cell.parse::<i64>()
                    .map_err(|_| ValueError("expected an integer".to_owned()))?,
            ),
            BaseType::Float | BaseType::Double => V::Double(
                cell.parse::<f64>()
                    .map_err(|_| ValueError("expected a number".to_owned()))?,
            ),
            BaseType::Boolean => match cell {
                "true" => V::Bool(true),
                "false" => V::Bool(false),
                _ => return fail("expected true or false"),
            },
            BaseType::String => V::String(cell.to_owned()),
            BaseType::Decimal => V::Decimal(cell.to_owned()),
            BaseType::Date => V::Date(cell.to_owned()),
            BaseType::Timestamp => V::Timestamp(cell.to_owned()),
            BaseType::Geopoint => {
                const SHAPE: &str = "expected `lat,lon`, two numbers separated by a comma";
                let (lat, lon) = cell
                    .split_once(',')
                    .ok_or_else(|| ValueError(SHAPE.to_owned()))?;
                let number = |part: &str| {
                    part.trim()
                        .parse::<f64>()
                        .map_err(|_| ValueError(SHAPE.to_owned()))
                };
                V::Geopoint {
                    lat: number(lat)?,
                    lon: number(lon)?,
                }
            }
            BaseType::Array => {
                let json: Value = serde_json::from_str(cell).map_err(|_| {
                    ValueError("expected a JSON array such as [\"a\", \"b\"]".to_owned())
                })?;
                if json.is_null() {
                    return fail("expected a JSON array, found null");
                }
                return Self::from_json(def, &json);
            }
        };
        value.canonical(def)
    }

    /// The value as a row predicate scalar; types a predicate cannot use become `Scalar::Null`.
    pub fn to_scalar(&self) -> Scalar {
        match self {
            Self::Bool(b) => Scalar::Bool(*b),
            Self::Int(i) => Scalar::Int(*i),
            Self::Double(d) => Scalar::Double(*d),
            Self::String(s) => Scalar::String(s.clone()),
            _ => Scalar::Null,
        }
    }
}

fn canonical_as(
    value: &PropertyValue,
    def: &PropertyDef,
    base: BaseType,
) -> Result<PropertyValue, ValueError> {
    use PropertyValue as V;
    match (base, value) {
        (BaseType::Byte, V::Int(i)) => int_in(*i, "byte", i8::MIN.into(), i8::MAX.into()),
        (BaseType::Short, V::Int(i)) => int_in(*i, "short", i16::MIN.into(), i16::MAX.into()),
        (BaseType::Integer, V::Int(i)) => int_in(*i, "integer", i32::MIN.into(), i32::MAX.into()),
        (BaseType::Long, V::Int(i)) => Ok(V::Int(*i)),
        (BaseType::Float, V::Double(d)) => float(*d),
        (BaseType::Double, V::Double(d)) if d.is_finite() => Ok(V::Double(*d)),
        (BaseType::Double, V::Double(_)) => fail("a double must be finite"),
        (BaseType::Boolean, V::Bool(b)) => Ok(V::Bool(*b)),
        (BaseType::String, V::String(s)) => Ok(V::String(s.clone())),
        (BaseType::Decimal, V::Decimal(s)) => {
            let (precision, scale) = def.effective_decimal();
            decimal(s, precision, scale).map(V::Decimal)
        }
        (BaseType::Date, V::Date(s)) => date(s).map(V::Date),
        (BaseType::Timestamp, V::Timestamp(s)) => timestamp(s).map(V::Timestamp),
        (BaseType::Geopoint, V::Geopoint { lat, lon }) => geopoint(*lat, *lon),
        (BaseType::Array, V::Array(items)) => {
            let element = def
                .items
                .ok_or_else(|| ValueError("an array property has no `items`".to_owned()))?;
            if element == BaseType::Array {
                return fail("an array cannot hold arrays");
            }
            items
                .iter()
                .map(|item| match item {
                    V::Null => fail("an array cannot hold null"),
                    item => canonical_as(item, def, element),
                })
                .collect::<Result<Vec<_>, _>>()
                .map(V::Array)
        }
        (base, other) => fail(format!(
            "expected {}, found {}",
            base.as_str(),
            describe(other)
        )),
    }
}

fn describe(value: &PropertyValue) -> &'static str {
    match value {
        PropertyValue::Null => "null",
        PropertyValue::Bool(_) => "a boolean",
        PropertyValue::Int(_) => "an integer",
        PropertyValue::Double(_) => "a double",
        PropertyValue::Decimal(_) => "a decimal",
        PropertyValue::String(_) => "a string",
        PropertyValue::Date(_) => "a date",
        PropertyValue::Timestamp(_) => "a timestamp",
        PropertyValue::Array(_) => "an array",
        PropertyValue::Geopoint { .. } => "a geopoint",
    }
}

fn int_in(i: i64, name: &str, min: i64, max: i64) -> Result<PropertyValue, ValueError> {
    if (min..=max).contains(&i) {
        Ok(PropertyValue::Int(i))
    } else {
        fail(format!("{i} is outside the range {min} to {max} of {name}"))
    }
}

/// A `float` is stored in 32 bits; the value is rounded to the nearest `f32` once, here, so the
/// log, the history and both databases hold the same number.
#[allow(
    clippy::cast_possible_truncation,
    reason = "rounding to f32 is the definition of a float value"
)]
fn float(d: f64) -> Result<PropertyValue, ValueError> {
    if d.is_finite() && d.abs() <= f64::from(f32::MAX) {
        Ok(PropertyValue::Double(f64::from(d as f32)))
    } else {
        fail("a float must be finite and fit 32 bits")
    }
}

fn geopoint(lat: f64, lon: f64) -> Result<PropertyValue, ValueError> {
    if !(-90.0..=90.0).contains(&lat) {
        return fail("the latitude must be between -90 and 90");
    }
    if !(-180.0..=180.0).contains(&lon) {
        return fail("the longitude must be between -180 and 180");
    }
    Ok(PropertyValue::Geopoint { lat, lon })
}

/// The decimal `text` with exactly `scale` fractional digits. Nothing is rounded: more
/// fractional digits than `scale`, or more integer digits than `precision - scale`, is an error.
/// The rule is the engine's, so the fixtures of the stand-in and the edit store agree.
fn decimal(text: &str, precision: u32, scale: u32) -> Result<String, ValueError> {
    normalize_decimal(text, precision, scale).map_err(ValueError)
}

/// A date written exactly as `YYYY-MM-DD`; the rule is the engine's.
fn date(text: &str) -> Result<String, ValueError> {
    parse_date(text)
        .map(|_| text.to_owned())
        .map_err(ValueError)
}

/// The instant of an RFC 3339 `text` with a UTC offset, as UTC text with exactly three
/// fractional digits. A space may stand for the `T`.
fn timestamp(text: &str) -> Result<String, ValueError> {
    const SHAPE: &str = "expected a timestamp with a UTC offset, such as 2026-03-29T02:30:00Z";
    if !text.is_ascii() {
        return fail(SHAPE);
    }
    let mut rfc = text.to_owned();
    if rfc.as_bytes().get(10) == Some(&b' ') {
        rfc.replace_range(10..11, "T");
    }
    if let Some(dot) = rfc.find('.') {
        let digits = rfc[dot + 1..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        if digits > 3 {
            return fail("fractional seconds must have at most three digits (timestamp(3))");
        }
    }
    let Ok(parsed) = DateTime::parse_from_rfc3339(&rfc) else {
        return fail(SHAPE);
    };
    if parsed.nanosecond() >= 1_000_000_000 {
        return fail("a leap second is not a valid timestamp here");
    }
    Ok(parsed
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn from_json_as(def: &PropertyDef, base: BaseType, v: &Value) -> Result<PropertyValue, ValueError> {
    use PropertyValue as V;
    if v.is_null() {
        return Ok(V::Null);
    }
    let wrong = |expected: &str| fail(format!("expected {expected}, found {}", json_kind(v)));
    match base {
        BaseType::Byte | BaseType::Short | BaseType::Integer | BaseType::Long => v
            .as_i64()
            .map(V::Int)
            .map_or_else(|| wrong("an integer"), Ok),
        BaseType::Float | BaseType::Double => v
            .as_f64()
            .map(V::Double)
            .map_or_else(|| wrong("a number"), Ok),
        BaseType::Boolean => v
            .as_bool()
            .map(V::Bool)
            .map_or_else(|| wrong("true or false"), Ok),
        BaseType::String => v
            .as_str()
            .map(|s| V::String(s.to_owned()))
            .map_or_else(|| wrong("a string"), Ok),
        BaseType::Decimal => match v {
            Value::String(s) => Ok(V::Decimal(s.clone())),
            // A JSON number with a fraction has already been through an f64 when it gets here
            // and 17 or more significant digits are gone; only an integer is exact.
            Value::Number(n) if n.is_i64() || n.is_u64() => Ok(V::Decimal(n.to_string())),
            Value::Number(_) => fail(
                "send a decimal as a string such as \"12.50\"; a JSON number with a fraction                  passes through a binary float and can be rounded",
            ),
            _ => wrong("a decimal string or number"),
        },
        BaseType::Date => v
            .as_str()
            .map(|s| V::Date(s.to_owned()))
            .map_or_else(|| wrong("a date string"), Ok),
        BaseType::Timestamp => v
            .as_str()
            .map(|s| V::Timestamp(s.to_owned()))
            .map_or_else(|| wrong("a timestamp string"), Ok),
        BaseType::Geopoint => {
            let coordinate = |key: &str| v.get(key).and_then(Value::as_f64);
            match (coordinate("lat"), coordinate("lon")) {
                (Some(lat), Some(lon)) => Ok(V::Geopoint { lat, lon }),
                _ => wrong("an object with numeric `lat` and `lon`"),
            }
        }
        BaseType::Array => {
            let Value::Array(items) = v else {
                return wrong("an array");
            };
            let element = def
                .items
                .ok_or_else(|| ValueError("an array property has no `items`".to_owned()))?;
            items
                .iter()
                .map(|item| from_json_as(def, element, item))
                .collect::<Result<Vec<_>, _>>()
                .map(V::Array)
        }
    }
}

fn json_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn def(base_type: BaseType) -> PropertyDef {
        PropertyDef {
            base_type,
            required: false,
            display_name: None,
            description: None,
            items: None,
            precision: Some(8),
            scale: Some(2),
            indexed: false,
        }
    }

    fn array_of(items: BaseType) -> PropertyDef {
        PropertyDef {
            items: Some(items),
            ..def(BaseType::Array)
        }
    }

    #[allow(
        clippy::needless_pass_by_value,
        reason = "test helper that reads best with literals"
    )]
    fn canonical(base_type: BaseType, value: PropertyValue) -> Result<PropertyValue, ValueError> {
        value.canonical(&def(base_type))
    }

    #[test]
    fn integer_ranges_follow_the_type() {
        for (base, ok, bad) in [
            (BaseType::Byte, 127, 128),
            (BaseType::Short, 32_767, 32_768),
            (BaseType::Integer, 2_147_483_647, 2_147_483_648),
        ] {
            assert!(canonical(base, PropertyValue::Int(ok)).is_ok(), "{base:?}");
            assert!(
                canonical(base, PropertyValue::Int(bad)).is_err(),
                "{base:?}"
            );
        }
        assert!(canonical(BaseType::Long, PropertyValue::Int(i64::MIN)).is_ok());
        assert!(canonical(BaseType::Integer, PropertyValue::String("1".into())).is_err());
    }

    #[test]
    fn decimal_is_padded_and_never_rounded() {
        let cases = [
            ("12.5", "12.50"),
            ("7", "7.00"),
            ("007.10", "7.10"),
            ("-0.00", "0.00"),
            ("-1.5", "-1.50"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                canonical(BaseType::Decimal, PropertyValue::Decimal(input.into())),
                Ok(PropertyValue::Decimal(expected.into())),
                "{input}"
            );
        }
        for bad in ["1.234", "1000000.00", "1e3", "", "abc", " 1"] {
            assert!(
                canonical(BaseType::Decimal, PropertyValue::Decimal(bad.into())).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn timestamp_needs_an_offset_and_is_stored_in_utc_milliseconds() {
        let ok = [
            ("2026-03-29T02:30:00+02:00", "2026-03-29T00:30:00.000Z"),
            ("2026-03-29T00:30:00Z", "2026-03-29T00:30:00.000Z"),
            ("2026-03-29 00:30:00.5Z", "2026-03-29T00:30:00.500Z"),
            ("2026-03-29T00:30:00.123-01:00", "2026-03-29T01:30:00.123Z"),
        ];
        for (input, expected) in ok {
            assert_eq!(
                canonical(BaseType::Timestamp, PropertyValue::Timestamp(input.into())),
                Ok(PropertyValue::Timestamp(expected.into())),
                "{input}"
            );
        }
        for bad in [
            "2026-03-29T00:30:00",
            "2026-03-29T00:30:00.1234Z",
            "2026-03-29",
            "2026-13-29T00:30:00Z",
            "yesterday",
        ] {
            assert!(
                canonical(BaseType::Timestamp, PropertyValue::Timestamp(bad.into())).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn date_is_a_calendar_day_written_as_iso() {
        assert!(canonical(BaseType::Date, PropertyValue::Date("2024-02-29".into())).is_ok());
        for bad in [
            "2023-02-29",
            "2024-2-29",
            "29.02.2024",
            "2024-02-29T00:00:00Z",
        ] {
            assert!(
                canonical(BaseType::Date, PropertyValue::Date(bad.into())).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn float_is_rounded_to_32_bits_once() {
        let value = canonical(BaseType::Float, PropertyValue::Double(1.1)).expect("float");
        assert_eq!(value, PropertyValue::Double(f64::from(1.1_f32)));
        assert_eq!(value.canonical(&def(BaseType::Float)), Ok(value.clone()));
        assert!(canonical(BaseType::Float, PropertyValue::Double(1e300)).is_err());
        assert!(canonical(BaseType::Double, PropertyValue::Double(f64::NAN)).is_err());
    }

    #[test]
    fn geopoint_is_range_checked() {
        let ok = PropertyValue::Geopoint {
            lat: 90.0,
            lon: -180.0,
        };
        assert!(canonical(BaseType::Geopoint, ok).is_ok());
        let bad = PropertyValue::Geopoint {
            lat: 90.5,
            lon: 0.0,
        };
        assert!(canonical(BaseType::Geopoint, bad).is_err());
    }

    #[test]
    fn array_elements_follow_items_and_may_not_be_null() {
        let def = array_of(BaseType::Decimal);
        let ok = PropertyValue::Array(vec![PropertyValue::Decimal("1".into())]);
        assert_eq!(
            ok.canonical(&def),
            Ok(PropertyValue::Array(vec![PropertyValue::Decimal(
                "1.00".into()
            )]))
        );
        let wrong = PropertyValue::Array(vec![PropertyValue::Int(1)]);
        assert!(wrong.canonical(&def).is_err());
        let null = PropertyValue::Array(vec![PropertyValue::Null]);
        assert!(null.canonical(&def).is_err());
    }

    #[test]
    fn null_is_refused_for_a_required_property_only() {
        let mut required = def(BaseType::String);
        required.required = true;
        assert!(PropertyValue::Null.check(&required).is_err());
        assert!(PropertyValue::Null.check(&def(BaseType::String)).is_ok());
    }

    #[test]
    fn json_round_trips_every_type() {
        let cases = [
            (def(BaseType::Integer), json!(5), PropertyValue::Int(5)),
            (
                def(BaseType::Boolean),
                json!(true),
                PropertyValue::Bool(true),
            ),
            (
                def(BaseType::Double),
                json!(1.5),
                PropertyValue::Double(1.5),
            ),
            (
                def(BaseType::Decimal),
                json!("12.50"),
                PropertyValue::Decimal("12.50".into()),
            ),
            (
                def(BaseType::Date),
                json!("2026-03-29"),
                PropertyValue::Date("2026-03-29".into()),
            ),
            (
                def(BaseType::Timestamp),
                json!("2026-03-29T00:30:00.000Z"),
                PropertyValue::Timestamp("2026-03-29T00:30:00.000Z".into()),
            ),
            (
                array_of(BaseType::String),
                json!(["a", "b"]),
                PropertyValue::Array(vec![
                    PropertyValue::String("a".into()),
                    PropertyValue::String("b".into()),
                ]),
            ),
            (
                def(BaseType::Geopoint),
                json!({ "lat": 52.5, "lon": 13.4 }),
                PropertyValue::Geopoint {
                    lat: 52.5,
                    lon: 13.4,
                },
            ),
        ];
        for (def, json, expected) in cases {
            let value = PropertyValue::from_json(&def, &json).expect("from_json");
            assert_eq!(value, expected);
            assert_eq!(value.to_json(), json);
        }
        // A decimal may be a JSON integer, never a JSON number with a fraction.
        assert_eq!(
            PropertyValue::from_json(&def(BaseType::Decimal), &json!(12)),
            Ok(PropertyValue::Decimal("12.00".into()))
        );
        assert!(PropertyValue::from_json(&def(BaseType::Decimal), &json!(12.5)).is_err());
        assert!(PropertyValue::from_json(&def(BaseType::Integer), &json!("5")).is_err());
        assert!(PropertyValue::from_json(&def(BaseType::Integer), &json!(5.5)).is_err());
    }

    #[test]
    fn decimal_json_numbers_must_be_exact_integers() {
        let wide = PropertyDef {
            precision: Some(38),
            scale: Some(10),
            ..def(BaseType::Decimal)
        };
        // serde_json parses a number with a fraction into an f64, which cannot hold 17 or more
        // significant digits.
        let fraction: Value = serde_json::from_str("12345678.1234567891").expect("json");
        let err = PropertyValue::from_json(&wide, &fraction).expect_err("a fractional number");
        assert!(err.0.contains("as a string"), "{err}");
        assert_eq!(
            PropertyValue::from_json(&wide, &json!("12345678.1234567891")),
            Ok(PropertyValue::Decimal("12345678.1234567891".into()))
        );
        assert_eq!(
            PropertyValue::from_json(&wide, &json!(12_345_678_901_234_567_890_u64)),
            Ok(PropertyValue::Decimal("12345678901234567890.0000000000".into()))
        );
        assert_eq!(
            PropertyValue::from_json(&wide, &json!(-7)),
            Ok(PropertyValue::Decimal("-7.0000000000".into()))
        );
        assert!(PropertyValue::from_json(&wide, &json!(1e2)).is_err());
        let array = PropertyDef {
            items: Some(BaseType::Decimal),
            ..wide
        };
        let array = PropertyDef {
            base_type: BaseType::Array,
            ..array
        };
        assert!(PropertyValue::from_json(&array, &json!([1, 2.5])).is_err());
        assert!(PropertyValue::from_json(&array, &json!([1, 2])).is_ok());
    }

    #[test]
    fn postgres_limits_are_validated() {
        assert!(canonical(BaseType::String, PropertyValue::String("a\u{0}".into())).is_err());
        let strings = array_of(BaseType::String);
        let nul = PropertyValue::Array(vec![PropertyValue::String("x\u{0}".into())]);
        assert!(nul.canonical(&strings).is_err());

        assert!(canonical(BaseType::Date, PropertyValue::Date("0000-01-01".into())).is_err());
        assert!(canonical(BaseType::Date, PropertyValue::Date("0001-01-01".into())).is_ok());

        for bad in ["0001-01-01T00:30:00+05:00", "9999-12-31T23:30:00-05:00"] {
            assert!(
                canonical(BaseType::Timestamp, PropertyValue::Timestamp(bad.into())).is_err(),
                "{bad}"
            );
        }
        assert!(
            canonical(
                BaseType::Timestamp,
                PropertyValue::Timestamp("0001-01-01T00:00:00Z".into())
            )
            .is_ok()
        );
    }

    fn text(base_type: BaseType, cell: &str) -> Result<PropertyValue, ValueError> {
        PropertyValue::from_text(&def(base_type), cell)
    }

    #[test]
    fn text_cells_convert_to_canonical_values() {
        assert_eq!(text(BaseType::Integer, "42"), Ok(PropertyValue::Int(42)));
        assert_eq!(text(BaseType::Long, "-7"), Ok(PropertyValue::Int(-7)));
        assert_eq!(
            text(BaseType::Double, "2.5"),
            Ok(PropertyValue::Double(2.5))
        );
        assert_eq!(
            text(BaseType::Float, "1.1"),
            Ok(PropertyValue::Double(f64::from(1.1_f32)))
        );
        assert_eq!(
            text(BaseType::Boolean, "true"),
            Ok(PropertyValue::Bool(true))
        );
        assert_eq!(
            text(BaseType::String, " a b "),
            Ok(PropertyValue::String(" a b ".into())),
            "a string is taken as it is"
        );
        assert_eq!(
            text(BaseType::Decimal, "1.5"),
            Ok(PropertyValue::Decimal("1.50".into()))
        );
        assert_eq!(
            text(BaseType::Date, "2024-02-29"),
            Ok(PropertyValue::Date("2024-02-29".into()))
        );
        assert_eq!(
            text(BaseType::Timestamp, "2026-03-29T02:30:00+02:00"),
            Ok(PropertyValue::Timestamp("2026-03-29T00:30:00.000Z".into()))
        );
        assert_eq!(
            text(BaseType::Geopoint, "50.03,8.57"),
            Ok(PropertyValue::Geopoint {
                lat: 50.03,
                lon: 8.57
            })
        );
        assert_eq!(
            PropertyValue::from_text(&array_of(BaseType::Decimal), "[\"1\", \"2.5\"]"),
            Ok(PropertyValue::Array(vec![
                PropertyValue::Decimal("1.00".into()),
                PropertyValue::Decimal("2.50".into()),
            ]))
        );
    }

    #[test]
    fn text_cells_that_do_not_fit_are_refused() {
        let bad = [
            (BaseType::Integer, "4.5"),
            (BaseType::Integer, "2147483648"),
            (BaseType::Integer, ""),
            (BaseType::Double, "NaN"),
            (BaseType::Double, "inf"),
            (BaseType::Boolean, "yes"),
            (BaseType::Boolean, "True"),
            (BaseType::Decimal, "1.555"),
            (BaseType::Decimal, "1e3"),
            (BaseType::Date, "29.02.2024"),
            (BaseType::Timestamp, "2026-03-29T00:30:00"),
            (BaseType::Geopoint, "91,0"),
            (BaseType::Geopoint, "0,181"),
            (BaseType::Geopoint, "1,2,3"),
            (BaseType::Geopoint, "12"),
            (BaseType::Geopoint, "a,b"),
        ];
        for (base, cell) in bad {
            assert!(text(base, cell).is_err(), "{base:?} {cell:?}");
        }
        assert!(PropertyValue::from_text(&array_of(BaseType::Integer), "[1, \"x\"]").is_err());
        assert!(PropertyValue::from_text(&array_of(BaseType::Integer), "1,2").is_err());
        assert!(PropertyValue::from_text(&array_of(BaseType::Integer), "[null]").is_err());
    }

    #[test]
    fn only_plain_values_become_predicate_scalars() {
        assert_eq!(PropertyValue::Int(3).to_scalar(), Scalar::Int(3));
        assert_eq!(
            PropertyValue::String("a".into()).to_scalar(),
            Scalar::String("a".into())
        );
        assert_eq!(
            PropertyValue::Decimal("1.00".into()).to_scalar(),
            Scalar::Null
        );
        assert_eq!(PropertyValue::Null.to_scalar(), Scalar::Null);
    }
}
