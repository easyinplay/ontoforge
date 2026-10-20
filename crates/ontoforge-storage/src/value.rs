//! Property values of the edit store and their one JSON encoding (D-10, D-11).
//!
//! `to_json` is the only encoding of a value in the edit log and in the history snapshots, and it
//! is the same encoding the typed edit tables hold (ADR-0024), so a replay needs no type
//! inference:
//!
//! - boolean, integers and finite doubles as JSON booleans and numbers,
//! - `decimal` as canonical decimal text with exactly the scale of the property (`"12.50"`),
//! - `date` as `YYYY-MM-DD`,
//! - `timestamp` as UTC with milliseconds, `YYYY-MM-DDTHH:MM:SS.fffZ`; input needs a UTC offset
//!   (`Z` or `+HH:MM`), is converted to UTC, and more than three fractional digits are refused
//!   instead of truncated,
//! - `array` as a JSON array of its elements, `geopoint` as `{"lat": .., "lon": ..}`.

use chrono::{DateTime, NaiveDate, SecondsFormat, Timelike, Utc};
use ontoforge_engine::value::split_decimal;
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
fn decimal(text: &str, precision: u32, scale: u32) -> Result<String, ValueError> {
    let Some(parts) = split_decimal(text) else {
        return fail("expected a decimal number such as -12.50");
    };
    let scale_len = usize::try_from(scale).unwrap_or(usize::MAX);
    if parts.fraction.len() > scale_len {
        return fail(format!(
            "more than {scale} fractional digits; a value is never rounded"
        ));
    }
    let integer = parts.integer.trim_start_matches('0');
    let integer_limit = usize::try_from(precision.saturating_sub(scale)).unwrap_or(0);
    if integer.len() > integer_limit {
        return fail(format!(
            "more than {integer_limit} integer digits (precision {precision}, scale {scale})"
        ));
    }
    let integer = if integer.is_empty() { "0" } else { integer };
    let is_zero = integer == "0" && parts.fraction.bytes().all(|b| b == b'0');
    let mut out = String::new();
    if parts.negative && !is_zero {
        out.push('-');
    }
    out.push_str(integer);
    if scale_len > 0 {
        out.push('.');
        out.push_str(parts.fraction);
        out.extend(std::iter::repeat_n('0', scale_len - parts.fraction.len()));
    }
    Ok(out)
}

/// A date written exactly as `YYYY-MM-DD`.
fn date(text: &str) -> Result<String, ValueError> {
    let b = text.as_bytes();
    let shaped = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !shaped {
        return fail("expected a date written as YYYY-MM-DD");
    }
    let number = |range: std::ops::Range<usize>| text[range].parse::<u32>().ok();
    let year = number(0..4).and_then(|y| i32::try_from(y).ok());
    match (year, number(5..7), number(8..10)) {
        (Some(y), Some(m), Some(d)) if NaiveDate::from_ymd_opt(y, m, d).is_some() => {
            Ok(text.to_owned())
        }
        _ => fail("not a calendar date"),
    }
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
            Value::Number(n) => Ok(V::Decimal(n.to_string())),
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
        // A client may send a decimal as a JSON number.
        assert_eq!(
            PropertyValue::from_json(&def(BaseType::Decimal), &json!(12.5)),
            Ok(PropertyValue::Decimal("12.50".into()))
        );
        assert!(PropertyValue::from_json(&def(BaseType::Integer), &json!("5")).is_err());
        assert!(PropertyValue::from_json(&def(BaseType::Integer), &json!(5.5)).is_err());
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
