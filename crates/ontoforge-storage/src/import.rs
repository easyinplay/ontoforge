//! Bulk import of editOnly data: a CSV or JSON file becomes system edits (STORE-06, D-11).
//!
//! The import is not a second write path. Every batch is one [`Commit`] by the reserved actor
//! `system:import`, so it is validated, fenced against the active definition version, logged and
//! snapshotted exactly like any other commit (ADR-0026). Only editOnly object types and editOnly
//! many-to-many links can be imported: business data of a source backed type belongs in the lake
//! (ADR-0016), and an import that could write it would be a general write door around the Actions.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::Read;

use camino::Utf8Path;
use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::{
    Backing, Cardinality, LinkTypeName, ObjectType, ObjectTypeName, PropertyDef, PropertyName,
};
use sea_query::{Alias, Expr, ExprTrait, Query};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::commit::{
    Actor, Commit, Edit, EditError, LinkOp, ObjectOp, SystemActor, bind, cell_json, link_backing,
    primary_key_def, read_expr,
};
use crate::error::StoreError;
use crate::physical::{ColumnType, LinkTable, ObjectTable};
use crate::sql_exit::{self, Cell, CellKind};
use crate::store::{ActiveDefinition, EditStore, WriteTx};
use crate::value::PropertyValue;

/// Rows per commit when the caller does not choose.
pub const DEFAULT_BATCH_SIZE: usize = 1_000;
/// The largest batch a caller may ask for.
pub const MAX_BATCH_SIZE: usize = 10_000;
/// Rows a file may hold when the caller does not choose.
pub const DEFAULT_MAX_ROWS: usize = 1_000_000;
/// Bytes a file may have when the caller does not choose: 256 MiB.
pub const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// The most problems one import reports; the rest of a bad file is not worth listing.
const MAX_PROBLEMS: usize = 100;
/// Keys per query when the existing rows are looked up.
const LOOKUP_CHUNK: usize = 500;
/// The longest cell text a message quotes.
const SHOWN_CELL: usize = 40;

/// What a file holds the rows of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportTarget {
    /// The objects of an editOnly object type; the columns are its properties.
    Object(ObjectTypeName),
    /// The edges of an editOnly many-to-many link; the columns are `from` and `to`.
    Link(LinkTypeName),
}

/// What to do with a row whose object (or edge) already has an edit row. A tombstoned object and
/// a removed edge are never revived, whatever the mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnExisting {
    /// Refuse the whole file and name the keys: the import only creates.
    #[default]
    Error,
    /// Leave existing rows alone; an interrupted import can be run again.
    Skip,
    /// Set the cells of the file on a live object.
    Update,
}

/// Limits and choices of one import.
#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub on_existing: OnExisting,
    /// Rows per commit, `1..=`[`MAX_BATCH_SIZE`].
    pub batch_size: usize,
    /// More rows than this refuse the file.
    pub max_rows: usize,
    /// A larger file is refused unread.
    pub max_bytes: u64,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            on_existing: OnExisting::Error,
            batch_size: DEFAULT_BATCH_SIZE,
            max_rows: DEFAULT_MAX_ROWS,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }
}

/// What an import did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// Data rows in the file.
    pub rows: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    /// The `seq` of each commit, in order.
    pub commits: Vec<i64>,
}

/// Why an import stopped.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The file or its rows are not acceptable (ONT-V27, with file and line). Nothing was written.
    #[error("{} problem(s) in the import file", .0.len())]
    Data(Vec<Diagnostic>),
    /// The target is not an editOnly object type or an editOnly many-to-many link. Nothing was
    /// written.
    #[error("{0}")]
    NotEditOnly(String),
    /// A batch failed after `committed` batches were written. The commits stay; running the import
    /// again with [`OnExisting::Skip`] writes the rest.
    #[error(
        "batch {failed_batch} failed after {} committed batch(es): {error}",
        committed.len()
    )]
    Partial {
        committed: Vec<i64>,
        /// The index of the failed batch, counted from 0.
        failed_batch: usize,
        error: Box<EditError>,
    },
    #[error(transparent)]
    Edit(EditError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Imports the rows of `file` into the edit store.
///
/// The whole file is read, converted and checked before anything is written; a file with a
/// problem writes nothing. The rows are then committed in batches of `opts.batch_size`, one
/// commit each, so a failure after the first batch leaves the earlier batches in place
/// ([`ImportError::Partial`]).
pub async fn import(
    store: &EditStore,
    target: ImportTarget,
    file: &Utf8Path,
    opts: &ImportOptions,
) -> Result<ImportReport, ImportError> {
    if !(1..=MAX_BATCH_SIZE).contains(&opts.batch_size) {
        let mut problems = Problems::new(file);
        problems.add(
            0,
            format!("the batch size must be between 1 and {MAX_BATCH_SIZE}"),
        );
        return Err(problems.into_error());
    }
    let def = store.active_definition().await.ok_or_else(|| {
        StoreError::Config("no definition is loaded; run reload first".to_owned())
    })?;
    match target {
        ImportTarget::Object(name) => import_objects(store, &def, &name, file, opts).await,
        ImportTarget::Link(link) => import_links(store, &def, &link, file, opts).await,
    }
}

// ---------------------------------------------------------------------------------------------
// Problems and places
// ---------------------------------------------------------------------------------------------

/// Where a row is in its file: a CSV line, or the position of an item of a JSON array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Place {
    n: u32,
    json: bool,
}

impl Place {
    /// The line a diagnostic carries; a JSON item has none.
    fn line(self) -> u32 {
        if self.json { 0 } else { self.n }
    }

    fn label(self) -> String {
        let what = if self.json { "item" } else { "line" };
        format!("{what} {}", self.n)
    }
}

/// What the problems of a file are collected in.
struct Problems {
    file: String,
    found: Vec<Diagnostic>,
    /// The check stopped, or dropped problems, because the list is full.
    truncated: bool,
}

impl Problems {
    fn new(file: &Utf8Path) -> Self {
        Self {
            file: file.as_str().to_owned(),
            found: Vec::new(),
            truncated: false,
        }
    }

    /// Records a problem at `line` (`0`: no line). Beyond [`MAX_PROBLEMS`] it only notes that
    /// there were more.
    fn add(&mut self, line: u32, message: impl Into<String>) {
        if self.found.len() >= MAX_PROBLEMS {
            self.truncated = true;
            return;
        }
        let mut found =
            Diagnostic::without_location(codes::V27, Severity::Error, message, self.file.clone());
        found.line = line;
        self.found.push(found);
    }

    /// Records a problem of the row at `place`. A JSON item has no line, so its position is
    /// part of the message.
    fn at(&mut self, place: Place, message: impl Into<String>) {
        let message = message.into();
        if place.json {
            self.add(0, format!("{}: {message}", place.label()));
        } else {
            self.add(place.line(), message);
        }
    }

    fn is_empty(&self) -> bool {
        self.found.is_empty()
    }

    /// Whether the list is full; if the caller stops reading because of it, rows are left
    /// unchecked.
    fn stop_reading(&mut self) -> bool {
        let full = self.found.len() >= MAX_PROBLEMS;
        self.truncated |= full;
        full
    }

    fn into_error(mut self) -> ImportError {
        if self.truncated {
            self.found.push(Diagnostic::without_location(
                codes::V27,
                Severity::Info,
                format!("more problems may follow; only the first {MAX_PROBLEMS} are listed"),
                self.file.clone(),
            ));
        }
        ImportError::Data(self.found)
    }
}

/// A cell text for a message: short, so a file with a huge cell cannot flood the output.
fn shown(cell: &str) -> String {
    let mut text: String = cell.chars().take(SHOWN_CELL).collect();
    if cell.chars().count() > SHOWN_CELL {
        text.push('…');
    }
    text
}

/// A primary key as it is written in a message.
fn show_key(pk: &PropertyValue) -> String {
    match pk.to_json() {
        Value::String(text) => shown(&text),
        other => shown(&other.to_string()),
    }
}

/// The identity of a primary key: its canonical JSON text.
fn key_of(pk: &PropertyValue) -> String {
    pk.to_json().to_string()
}

/// The identity of an edge.
fn edge_key(a: &PropertyValue, b: &PropertyValue) -> String {
    json!([a.to_json(), b.to_json()]).to_string()
}

/// The first place each key was seen at, so that a repeated key is refused.
#[derive(Default)]
struct FirstSeen(HashMap<String, Place>);

impl FirstSeen {
    /// `false`, with a problem, when `key` appeared before.
    fn admit(
        &mut self,
        key: String,
        shown_key: &str,
        place: Place,
        problems: &mut Problems,
    ) -> bool {
        if let Some(first) = self.0.get(&key) {
            problems.at(
                place,
                format!(
                    "`{shown_key}` already appears on {} of the file",
                    first.label()
                ),
            );
            return false;
        }
        self.0.insert(key, place);
        true
    }
}

// ---------------------------------------------------------------------------------------------
// Reading a file
// ---------------------------------------------------------------------------------------------

/// What the file format is, from the extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Csv,
    Json,
}

fn format_of(file: &Utf8Path) -> Option<Format> {
    match file.extension().map(str::to_ascii_lowercase).as_deref() {
        Some("csv") => Some(Format::Csv),
        Some("json") => Some(Format::Json),
        _ => None,
    }
}

/// Turns the records or items of a file into rows of one target.
trait RowReader {
    type Row;

    /// Looks at the header line of a CSV file. `false` when no row can be read.
    fn header(&mut self, headers: &csv::StringRecord, problems: &mut Problems) -> bool;

    /// One record of a CSV file. `None` when the row has a problem (already recorded).
    fn record(
        &mut self,
        record: &csv::StringRecord,
        place: Place,
        problems: &mut Problems,
    ) -> Option<Self::Row>;

    /// One item of a JSON file. `None` when the row has a problem (already recorded).
    fn item(&mut self, item: &Value, place: Place, problems: &mut Problems) -> Option<Self::Row>;
}

/// Reads and converts the rows of `file` with `reader`; the problems go to `problems`.
fn read_rows<R: RowReader>(
    reader: &mut R,
    file: &Utf8Path,
    opts: &ImportOptions,
    problems: &mut Problems,
) -> (Vec<R::Row>, String) {
    match format_of(file) {
        Some(Format::Csv) => read_csv(reader, file, opts, problems),
        Some(Format::Json) => read_json(reader, file, opts, problems),
        None => {
            problems.add(0, "the file must be a .csv or a .json file");
            (Vec::new(), String::new())
        }
    }
}

/// Opens `file` when it is a regular file of at most `max_bytes`; otherwise records why not.
fn open_limited(file: &Utf8Path, max_bytes: u64, problems: &mut Problems) -> Option<std::fs::File> {
    let meta = match std::fs::metadata(file.as_std_path()) {
        Ok(meta) => meta,
        Err(error) => {
            problems.add(0, format!("cannot read the file: {}", error.kind()));
            return None;
        }
    };
    if !meta.is_file() {
        problems.add(0, "the path is not a regular file");
        return None;
    }
    if meta.len() > max_bytes {
        problems.add(
            0,
            format!("the file is larger than the limit of {max_bytes} bytes"),
        );
        return None;
    }
    match std::fs::File::open(file.as_std_path()) {
        Ok(handle) => Some(handle),
        Err(error) => {
            problems.add(0, format!("cannot read the file: {}", error.kind()));
            None
        }
    }
}

fn line_of(position: Option<&csv::Position>) -> u32 {
    position.map_or(0, |p| u32::try_from(p.line()).unwrap_or(u32::MAX))
}

/// Reads the rows of a CSV file, streaming. The first line names the columns.
fn read_csv<R: RowReader>(
    reader: &mut R,
    file: &Utf8Path,
    opts: &ImportOptions,
    problems: &mut Problems,
) -> (Vec<R::Row>, String) {
    let Some(handle) = open_limited(file, opts.max_bytes, problems) else {
        return (Vec::new(), String::new());
    };
    let mut hashed = HashingReader {
        inner: handle.take(opts.max_bytes.saturating_add(1)),
        digest: Sha256::new(),
    };
    // The csv reader skips a byte order mark in front of the header.
    let mut csv = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(&mut hashed);
    let headers = match csv.headers() {
        Ok(headers) => headers.clone(),
        Err(error) => {
            problems.add(
                line_of(error.position()),
                format!("cannot read the header: {error}"),
            );
            return (Vec::new(), String::new());
        }
    };
    if headers.is_empty() {
        problems.add(1, "the file has no header line");
        return (Vec::new(), String::new());
    }
    if !reader.header(&headers, problems) {
        return (Vec::new(), String::new());
    }

    let mut rows = Vec::new();
    let mut data_rows = 0_usize;
    for record in csv.records() {
        if problems.stop_reading() {
            break;
        }
        let record = match record {
            Ok(record) => record,
            Err(error) => {
                let io_failure = matches!(error.kind(), csv::ErrorKind::Io(_));
                problems.add(line_of(error.position()), error.to_string());
                if io_failure {
                    break;
                }
                continue;
            }
        };
        let place = Place {
            n: line_of(record.position()),
            json: false,
        };
        if record.position().is_some_and(|p| p.byte() > opts.max_bytes) {
            problems.add(
                0,
                format!(
                    "the file is larger than the limit of {} bytes",
                    opts.max_bytes
                ),
            );
            break;
        }
        data_rows += 1;
        if data_rows > opts.max_rows {
            problems.at(
                place,
                format!("the file has more than {} rows", opts.max_rows),
            );
            break;
        }
        rows.extend(reader.record(&record, place, problems));
    }
    drop(csv);
    (rows, format!("{:x}", hashed.digest.finalize()))
}

struct HashingReader<R> {
    inner: R,
    digest: Sha256,
}

impl<R: Read> Read for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let count = self.inner.read(buf)?;
        self.digest.update(&buf[..count]);
        Ok(count)
    }
}

/// Reads the rows of a JSON file: an array of objects, one per row.
fn read_json<R: RowReader>(
    reader: &mut R,
    file: &Utf8Path,
    opts: &ImportOptions,
    problems: &mut Problems,
) -> (Vec<R::Row>, String) {
    let Some(handle) = open_limited(file, opts.max_bytes, problems) else {
        return (Vec::new(), String::new());
    };
    let mut bytes = Vec::new();
    if let Err(error) = handle
        .take(opts.max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
    {
        problems.add(0, format!("cannot read the file: {}", error.kind()));
        return (Vec::new(), String::new());
    }
    if u64::try_from(bytes.len()).is_ok_and(|len| len > opts.max_bytes) {
        problems.add(
            0,
            format!(
                "the file is larger than the limit of {} bytes",
                opts.max_bytes
            ),
        );
        return (Vec::new(), String::new());
    }
    let body = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let items = match serde_json::from_slice::<Value>(body) {
        Ok(Value::Array(items)) => items,
        Ok(_) => {
            problems.add(
                0,
                "the top level of a JSON import file must be an array of objects",
            );
            return (Vec::new(), String::new());
        }
        Err(error) => {
            problems.add(
                u32::try_from(error.line()).unwrap_or(u32::MAX),
                format!("not valid JSON: {error}"),
            );
            return (Vec::new(), String::new());
        }
    };
    if items.len() > opts.max_rows {
        problems.add(0, format!("the file has more than {} rows", opts.max_rows));
        return (Vec::new(), String::new());
    }
    let mut rows = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        if problems.stop_reading() {
            break;
        }
        let place = Place {
            n: u32::try_from(index + 1).unwrap_or(u32::MAX),
            json: true,
        };
        rows.extend(reader.item(item, place, problems));
    }
    (rows, format!("{:x}", Sha256::digest(&bytes)))
}

// ---------------------------------------------------------------------------------------------
// Object rows
// ---------------------------------------------------------------------------------------------

/// One accepted object row.
struct ObjectRow {
    place: Place,
    pk: PropertyValue,
    values: BTreeMap<PropertyName, PropertyValue>,
}

/// The object type an import fills, with what a row is checked against.
struct Shape<'a> {
    ty: &'a ObjectType,
    pk_def: &'a PropertyDef,
    /// The required properties other than the primary key.
    required: Vec<&'a PropertyName>,
}

impl<'a> Shape<'a> {
    fn new(ty: &'a ObjectType) -> Result<Self, ImportError> {
        let pk_def = primary_key_def(ty).map_err(ImportError::Edit)?;
        let required = ty
            .properties
            .iter()
            .filter(|(name, def)| def.required && **name != ty.primary_key)
            .map(|(name, _)| name)
            .collect();
        Ok(Self {
            ty,
            pk_def,
            required,
        })
    }
}

/// What a column of a CSV header is.
enum Slot<'a> {
    Pk(&'a PropertyDef),
    Prop(&'a PropertyName, &'a PropertyDef),
    Ignored,
}

/// What the cells or keys of one row came to.
struct Candidate<'a> {
    pk: Option<PropertyValue>,
    values: BTreeMap<PropertyName, PropertyValue>,
    /// Properties whose value had a problem, so they are not reported as missing as well.
    failed: BTreeSet<&'a PropertyName>,
    /// No cell had a problem.
    ok: bool,
}

impl Candidate<'_> {
    fn new() -> Self {
        Self {
            pk: None,
            values: BTreeMap::new(),
            failed: BTreeSet::new(),
            ok: true,
        }
    }
}

struct ObjectReader<'a> {
    shape: Shape<'a>,
    slots: Vec<Slot<'a>>,
    /// Required properties without a column; reported once with the header, not for every row.
    missing: BTreeSet<&'a PropertyName>,
    seen: FirstSeen,
}

impl<'a> ObjectReader<'a> {
    fn new(shape: Shape<'a>) -> Self {
        Self {
            shape,
            slots: Vec::new(),
            missing: BTreeSet::new(),
            seen: FirstSeen::default(),
        }
    }

    /// Reports the required properties a row lacks, then admits the row if it is good.
    fn finish(
        &mut self,
        found: Candidate<'a>,
        place: Place,
        problems: &mut Problems,
    ) -> Option<ObjectRow> {
        let Candidate {
            pk,
            values,
            failed,
            mut ok,
        } = found;
        for name in &self.shape.required {
            if !values.contains_key(*name)
                && !failed.contains(*name)
                && !self.missing.contains(*name)
            {
                problems.at(
                    place,
                    format!("the required property `{}` is empty", name.as_str()),
                );
                ok = false;
            }
        }
        let pk = pk?;
        let shown_key = show_key(&pk);
        let fresh = self.seen.admit(key_of(&pk), &shown_key, place, problems);
        (ok && fresh).then_some(ObjectRow { place, pk, values })
    }
}

impl RowReader for ObjectReader<'_> {
    type Row = ObjectRow;

    fn header(&mut self, headers: &csv::StringRecord, problems: &mut Problems) -> bool {
        let ty = self.shape.ty;
        let mut seen_columns = BTreeSet::new();
        for column in headers {
            if !seen_columns.insert(column.to_owned()) {
                problems.add(1, format!("the column `{}` appears twice", shown(column)));
                self.slots.push(Slot::Ignored);
                continue;
            }
            match ty.properties.get_key_value(column) {
                Some((name, _)) if *name == ty.primary_key => {
                    self.slots.push(Slot::Pk(self.shape.pk_def));
                }
                Some((name, def)) => self.slots.push(Slot::Prop(name, def)),
                None => {
                    let known: Vec<&str> = ty.properties.keys().map(PropertyName::as_str).collect();
                    problems.add(
                        1,
                        format!(
                            "unknown column `{}`; the properties of `{}` are {}",
                            shown(column),
                            ty.api_name.as_str(),
                            known.join(", ")
                        ),
                    );
                    self.slots.push(Slot::Ignored);
                }
            }
        }
        let has_pk = self.slots.iter().any(|slot| matches!(slot, Slot::Pk(_)));
        if !has_pk {
            problems.add(
                1,
                format!(
                    "the primary key column `{}` is missing",
                    ty.primary_key.as_str()
                ),
            );
        }
        for name in &self.shape.required {
            if !seen_columns.contains(name.as_str()) {
                problems.add(
                    1,
                    format!("the required column `{}` is missing", name.as_str()),
                );
                self.missing.insert(*name);
            }
        }
        // Without the key no row can be told from another.
        has_pk
    }

    fn record(
        &mut self,
        record: &csv::StringRecord,
        place: Place,
        problems: &mut Problems,
    ) -> Option<ObjectRow> {
        let pk_name = self.shape.ty.primary_key.as_str();
        let mut found = Candidate::new();
        for (slot, cell) in self.slots.iter().zip(record.iter()) {
            match slot {
                Slot::Ignored => {}
                Slot::Pk(_) if cell.is_empty() => {
                    problems.at(place, format!("the primary key `{pk_name}` is empty"));
                    found.ok = false;
                }
                Slot::Pk(def) => match PropertyValue::from_text(def, cell) {
                    Ok(value) => found.pk = Some(value),
                    Err(error) => {
                        problems.at(
                            place,
                            format!("primary key `{pk_name}`: `{}`: {error}", shown(cell)),
                        );
                        found.ok = false;
                    }
                },
                Slot::Prop(_, _) if cell.is_empty() => {}
                Slot::Prop(name, def) => match PropertyValue::from_text(def, cell) {
                    Ok(value) => {
                        found.values.insert((*name).clone(), value);
                    }
                    Err(error) => {
                        problems.at(
                            place,
                            format!("column `{}`: `{}`: {error}", name.as_str(), shown(cell)),
                        );
                        found.failed.insert(*name);
                        found.ok = false;
                    }
                },
            }
        }
        self.finish(found, place, problems)
    }

    fn item(&mut self, item: &Value, place: Place, problems: &mut Problems) -> Option<ObjectRow> {
        let Some(map) = item.as_object() else {
            problems.at(place, "expected an object");
            return None;
        };
        let ty = self.shape.ty;
        let mut found = Candidate::new();
        let mut pk_failed = false;
        for (key, value) in map {
            let Some((name, def)) = ty.properties.get_key_value(key.as_str()) else {
                problems.at(place, format!("unknown property `{}`", shown(key)));
                found.ok = false;
                continue;
            };
            if value.is_null() {
                continue;
            }
            match PropertyValue::from_json(def, value) {
                Ok(parsed) if *name == ty.primary_key => found.pk = Some(parsed),
                Ok(parsed) => {
                    found.values.insert(name.clone(), parsed);
                }
                Err(error) => {
                    problems.at(place, format!("property `{}`: {error}", name.as_str()));
                    pk_failed |= *name == ty.primary_key;
                    found.failed.insert(name);
                    found.ok = false;
                }
            }
        }
        if found.pk.is_none() && !pk_failed {
            problems.at(
                place,
                format!("the primary key `{}` is missing", ty.primary_key.as_str()),
            );
            found.ok = false;
        }
        self.finish(found, place, problems)
    }
}

async fn import_objects(
    store: &EditStore,
    def: &ActiveDefinition,
    name: &ObjectTypeName,
    file: &Utf8Path,
    opts: &ImportOptions,
) -> Result<ImportReport, ImportError> {
    let unknown = || ImportError::Edit(EditError::UnknownType(name.as_str().to_owned()));
    let (type_name, ty) = def
        .ontology
        .object_types
        .get_key_value(name)
        .ok_or_else(unknown)?;
    if ty.backing != Backing::EditOnly {
        return Err(ImportError::NotEditOnly(format!(
            "object type `{}` is source backed; only editOnly object types can be imported",
            type_name.as_str()
        )));
    }
    let table = def.physical.object_table(type_name).ok_or_else(unknown)?;

    let mut problems = Problems::new(file);
    let mut reader = ObjectReader::new(Shape::new(ty)?);
    let (rows, source_digest) = read_rows(&mut reader, file, opts, &mut problems);
    if !problems.is_empty() {
        return Err(problems.into_error());
    }
    let mut report = ImportReport {
        rows: rows.len(),
        ..ImportReport::default()
    };
    if rows.is_empty() {
        return Ok(report);
    }

    let existing = existing_objects(store, ty, table, &rows).await?;
    let mut edits = Vec::with_capacity(rows.len());
    for row in rows {
        let shown_key = show_key(&row.pk);
        let decision = decide(
            opts.on_existing,
            existing.get(&key_of(&row.pk)).copied(),
            !row.values.is_empty(),
            &format!("object `{shown_key}`"),
        );
        match decision {
            Err(message) => problems.at(row.place, message),
            Ok(Decision::Skip) => report.skipped += 1,
            Ok(decision) => {
                match decision {
                    Decision::Create => report.created += 1,
                    _ => report.updated += 1,
                }
                edits.push(Edit::Object {
                    type_name: type_name.clone(),
                    pk: row.pk,
                    op: ObjectOp::Set(row.values),
                });
            }
        }
    }
    if !problems.is_empty() {
        return Err(problems.into_error());
    }
    commit_in_batches(
        store,
        def.def_version,
        file,
        &source_digest,
        edits,
        opts,
        &mut report,
    )
    .await?;
    Ok(report)
}

// ---------------------------------------------------------------------------------------------
// Edge rows
// ---------------------------------------------------------------------------------------------

/// One accepted edge row.
struct EdgeRow {
    place: Place,
    a: PropertyValue,
    b: PropertyValue,
}

/// The two ends of an editOnly many-to-many link: the key definitions of the `from` and the `to`
/// object type.
struct EdgeReader<'a> {
    from_def: &'a PropertyDef,
    to_def: &'a PropertyDef,
    /// The column of `from` and of `to` in a CSV file.
    columns: [Option<usize>; 2],
    seen: FirstSeen,
}

impl EdgeReader<'_> {
    fn finish(
        &mut self,
        a: Option<PropertyValue>,
        b: Option<PropertyValue>,
        place: Place,
        problems: &mut Problems,
    ) -> Option<EdgeRow> {
        let (a, b) = (a?, b?);
        let label = format!("{} to {}", show_key(&a), show_key(&b));
        let fresh = self.seen.admit(edge_key(&a, &b), &label, place, problems);
        fresh.then_some(EdgeRow { place, a, b })
    }

    fn end_from_text(
        def: &PropertyDef,
        which: &str,
        cell: &str,
        place: Place,
        problems: &mut Problems,
    ) -> Option<PropertyValue> {
        if cell.is_empty() {
            problems.at(place, format!("`{which}` is empty"));
            return None;
        }
        match PropertyValue::from_text(def, cell) {
            Ok(value) => Some(value),
            Err(error) => {
                problems.at(place, format!("`{which}`: `{}`: {error}", shown(cell)));
                None
            }
        }
    }

    fn end_from_json(
        def: &PropertyDef,
        which: &str,
        value: Option<&Value>,
        place: Place,
        problems: &mut Problems,
    ) -> Option<PropertyValue> {
        let Some(value) = value.filter(|value| !value.is_null()) else {
            problems.at(place, format!("`{which}` is missing"));
            return None;
        };
        match PropertyValue::from_json(def, value) {
            Ok(value) => Some(value),
            Err(error) => {
                problems.at(place, format!("`{which}`: {error}"));
                None
            }
        }
    }
}

impl RowReader for EdgeReader<'_> {
    type Row = EdgeRow;

    fn header(&mut self, headers: &csv::StringRecord, problems: &mut Problems) -> bool {
        for (index, column) in headers.iter().enumerate() {
            let slot = match column {
                "from" => &mut self.columns[0],
                "to" => &mut self.columns[1],
                other => {
                    problems.add(
                        1,
                        format!(
                            "unknown column `{}`; a link file has the columns `from` and `to`",
                            shown(other)
                        ),
                    );
                    continue;
                }
            };
            if slot.replace(index).is_some() {
                problems.add(1, format!("the column `{column}` appears twice"));
            }
        }
        for (which, slot) in ["from", "to"].into_iter().zip(self.columns) {
            if slot.is_none() {
                problems.add(1, format!("the column `{which}` is missing"));
            }
        }
        self.columns.iter().all(Option::is_some)
    }

    fn record(
        &mut self,
        record: &csv::StringRecord,
        place: Place,
        problems: &mut Problems,
    ) -> Option<EdgeRow> {
        let cell = |slot: Option<usize>| slot.and_then(|index| record.get(index)).unwrap_or("");
        let a = Self::end_from_text(
            self.from_def,
            "from",
            cell(self.columns[0]),
            place,
            problems,
        );
        let b = Self::end_from_text(self.to_def, "to", cell(self.columns[1]), place, problems);
        self.finish(a, b, place, problems)
    }

    fn item(&mut self, item: &Value, place: Place, problems: &mut Problems) -> Option<EdgeRow> {
        let Some(map) = item.as_object() else {
            problems.at(place, "expected an object");
            return None;
        };
        let mut clean = true;
        for key in map
            .keys()
            .filter(|key| !matches!(key.as_str(), "from" | "to"))
        {
            problems.at(
                place,
                format!(
                    "unknown key `{}`; a link item has the keys `from` and `to`",
                    shown(key)
                ),
            );
            clean = false;
        }
        let a = Self::end_from_json(self.from_def, "from", map.get("from"), place, problems);
        let b = Self::end_from_json(self.to_def, "to", map.get("to"), place, problems);
        let row = self.finish(a, b, place, problems);
        row.filter(|_| clean)
    }
}

async fn import_links(
    store: &EditStore,
    def: &ActiveDefinition,
    name: &LinkTypeName,
    file: &Utf8Path,
    opts: &ImportOptions,
) -> Result<ImportReport, ImportError> {
    let unknown = || ImportError::Edit(EditError::UnknownType(name.as_str().to_owned()));
    let (link_name, link) = def
        .ontology
        .link_types
        .get_key_value(name)
        .ok_or_else(unknown)?;
    if link.cardinality != Cardinality::ManyToMany {
        return Err(ImportError::NotEditOnly(format!(
            "link `{}` is not many-to-many; only a many-to-many link has rows of its own",
            link_name.as_str()
        )));
    }
    if link_backing(link.backing) != Backing::EditOnly {
        return Err(ImportError::NotEditOnly(format!(
            "link `{}` is source backed; only editOnly links can be imported",
            link_name.as_str()
        )));
    }
    let table = def.physical.link_table(link_name).ok_or_else(unknown)?;
    let end_def = |object: &ObjectTypeName| {
        def.ontology
            .object_types
            .get(object)
            .ok_or_else(|| {
                ImportError::Store(StoreError::Corrupt(format!(
                    "link `{}` refers to unknown type `{}`",
                    link_name.as_str(),
                    object.as_str()
                )))
            })
            .and_then(|ty| primary_key_def(ty).map_err(ImportError::Edit))
    };
    let from_def = end_def(&link.from.object_type)?;
    let to_def = end_def(&link.to.object_type)?;

    let mut problems = Problems::new(file);
    let mut reader = EdgeReader {
        from_def,
        to_def,
        columns: [None, None],
        seen: FirstSeen::default(),
    };
    let (rows, source_digest) = read_rows(&mut reader, file, opts, &mut problems);
    if !problems.is_empty() {
        return Err(problems.into_error());
    }
    let mut report = ImportReport {
        rows: rows.len(),
        ..ImportReport::default()
    };
    if rows.is_empty() {
        return Ok(report);
    }

    let existing = existing_edges(store, from_def, to_def, table, &rows).await?;
    let mut edits = Vec::with_capacity(rows.len());
    for row in rows {
        let label = format!("edge `{}` to `{}`", show_key(&row.a), show_key(&row.b));
        let decision = decide(
            opts.on_existing,
            existing.get(&edge_key(&row.a, &row.b)).copied(),
            // An edge has no cells of its own; an existing one is left as it is.
            true,
            &label,
        );
        match decision {
            Err(message) => problems.at(row.place, message),
            Ok(Decision::Create) => {
                report.created += 1;
                edits.push(Edit::Link {
                    link: link_name.clone(),
                    a: row.a,
                    b: row.b,
                    op: LinkOp::Add,
                });
            }
            Ok(_) => report.skipped += 1,
        }
    }
    if !problems.is_empty() {
        return Err(problems.into_error());
    }
    commit_in_batches(
        store,
        def.def_version,
        file,
        &source_digest,
        edits,
        opts,
        &mut report,
    )
    .await?;
    Ok(report)
}

// ---------------------------------------------------------------------------------------------
// Existing rows and the decision for each row
// ---------------------------------------------------------------------------------------------

/// What happens to one row of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Create,
    Update,
    Skip,
}

/// What `mode` makes of a row whose thing (`what`) has an edit row or not: `existing` is
/// `Some(deleted)` when it has one. `has_values` is whether the row carries anything to set. The
/// error is the problem text. A deleted object or removed edge is never brought back.
fn decide(
    mode: OnExisting,
    existing: Option<bool>,
    has_values: bool,
    what: &str,
) -> Result<Decision, String> {
    match (existing, mode) {
        (None, _) if has_values => Ok(Decision::Create),
        (None, _) => Err(format!(
            "{what} has no value besides its primary key, and an object cannot be created from a \
             key alone"
        )),
        (Some(deleted), OnExisting::Error) => Err(format!(
            "{what} already exists{}; the import only creates (use `--on-existing skip` or \
             `update`)",
            if deleted { " (deleted)" } else { "" }
        )),
        (Some(false), OnExisting::Update) if has_values => Ok(Decision::Update),
        (Some(true), OnExisting::Update) => Err(format!(
            "{what} was deleted and an import never brings it back"
        )),
        // Skipping, or an update with nothing to set.
        (Some(_), OnExisting::Skip | OnExisting::Update) => Ok(Decision::Skip),
    }
}

fn col(name: &str) -> Alias {
    Alias::new(name)
}

/// The objects of `rows` that have an edit row, by key, with whether they are tombstoned.
async fn existing_objects(
    store: &EditStore,
    ty: &ObjectType,
    table: &ObjectTable,
    rows: &[ObjectRow],
) -> Result<HashMap<String, bool>, ImportError> {
    let pk_def = primary_key_def(ty).map_err(ImportError::Edit)?;
    let mut tx = store.begin_write().await?;
    let keys: Vec<&PropertyValue> = rows.iter().map(|row| &row.pk).collect();
    let found = lookup_objects(&mut tx, table, pk_def, &keys).await;
    sql_exit::rollback(tx).await;
    Ok(found?)
}

async fn lookup_objects(
    tx: &mut WriteTx<'_>,
    table: &ObjectTable,
    pk_def: &PropertyDef,
    keys: &[&PropertyValue],
) -> Result<HashMap<String, bool>, StoreError> {
    let backend = tx.backend();
    let schema = tx.schema_edit().map(str::to_owned);
    let (pk_read, pk_kind) = read_expr(&table.pk.column, &table.pk.ty, backend);
    let mut found = HashMap::new();
    for chunk in keys.chunks(LOOKUP_CHUNK) {
        let select = Query::select()
            .expr(pk_read.clone())
            .column(col("of_version"))
            .column(col("of_deleted"))
            .from(sql_exit::edit_table(schema.as_deref(), &table.table))
            .and_where(
                Expr::col(col(&table.pk.column))
                    .is_in(chunk.iter().map(|pk| bind(pk, &table.pk.ty, backend))),
            )
            .to_owned();
        let built = sql_exit::build(&select, tx);
        let found_rows =
            sql_exit::fetch_all_cells(tx, built, &[pk_kind, CellKind::Int64, CellKind::Bool])
                .await?;
        for cells in found_rows {
            let [pk, Cell::Int(version), Cell::Bool(deleted)] = cells.as_slice() else {
                return Err(StoreError::Corrupt(
                    "an edit row has no key, version or flag".to_owned(),
                ));
            };
            if *version <= 0 {
                continue;
            }
            let pk = stored_key(pk, &table.pk.ty, pk_def)?;
            found.insert(key_of(&pk), *deleted);
        }
    }
    Ok(found)
}

/// A key cell read back from the database, in canonical form.
fn stored_key(
    cell: &Cell,
    ty: &ColumnType,
    def: &PropertyDef,
) -> Result<PropertyValue, StoreError> {
    let json = cell_json(cell.clone(), ty)?;
    PropertyValue::from_json(def, &json)
        .map_err(|error| StoreError::Corrupt(format!("a stored key is invalid: {error}")))
}

/// The edges of `rows` that have an edit row, by key, with whether they were removed.
async fn existing_edges(
    store: &EditStore,
    from_def: &PropertyDef,
    to_def: &PropertyDef,
    table: &LinkTable,
    rows: &[EdgeRow],
) -> Result<HashMap<String, bool>, ImportError> {
    let mut tx = store.begin_write().await?;
    let found = lookup_edges(&mut tx, from_def, to_def, table, rows).await;
    sql_exit::rollback(tx).await;
    Ok(found?)
}

/// Looks the edges up by their two ends: a query takes every row whose `a` and `b` are among
/// those of a chunk of edges, which may be more rows than the edges asked for; only the pairs
/// asked for are looked at afterwards.
async fn lookup_edges(
    tx: &mut WriteTx<'_>,
    from_def: &PropertyDef,
    to_def: &PropertyDef,
    table: &LinkTable,
    rows: &[EdgeRow],
) -> Result<HashMap<String, bool>, StoreError> {
    let backend = tx.backend();
    let schema = tx.schema_edit().map(str::to_owned);
    let (a_read, a_kind) = read_expr(&table.a.column, &table.a.ty, backend);
    let (b_read, b_kind) = read_expr(&table.b.column, &table.b.ty, backend);
    let wanted: HashSet<String> = rows.iter().map(|row| edge_key(&row.a, &row.b)).collect();
    let mut found = HashMap::new();
    for chunk in rows.chunks(LOOKUP_CHUNK) {
        let ends = |pick: fn(&EdgeRow) -> &PropertyValue| -> Vec<&PropertyValue> {
            let distinct: BTreeMap<String, &PropertyValue> = chunk
                .iter()
                .map(|row| (key_of(pick(row)), pick(row)))
                .collect();
            distinct.into_values().collect()
        };
        let select = Query::select()
            .expr(a_read.clone())
            .expr(b_read.clone())
            .column(col("of_version"))
            .column(col("of_deleted"))
            .from(sql_exit::edit_table(schema.as_deref(), &table.table))
            .and_where(
                Expr::col(col(&table.a.column)).is_in(
                    ends(|row| &row.a)
                        .into_iter()
                        .map(|a| bind(a, &table.a.ty, backend)),
                ),
            )
            .and_where(
                Expr::col(col(&table.b.column)).is_in(
                    ends(|row| &row.b)
                        .into_iter()
                        .map(|b| bind(b, &table.b.ty, backend)),
                ),
            )
            .to_owned();
        let built = sql_exit::build(&select, tx);
        let kinds = [a_kind, b_kind, CellKind::Int64, CellKind::Bool];
        for cells in sql_exit::fetch_all_cells(tx, built, &kinds).await? {
            let [a, b, Cell::Int(version), Cell::Bool(deleted)] = cells.as_slice() else {
                return Err(StoreError::Corrupt(
                    "a link row has no keys, version or flag".to_owned(),
                ));
            };
            if *version <= 0 {
                continue;
            }
            let key = edge_key(
                &stored_key(a, &table.a.ty, from_def)?,
                &stored_key(b, &table.b.ty, to_def)?,
            );
            if wanted.contains(&key) {
                found.insert(key, *deleted);
            }
        }
    }
    Ok(found)
}

// ---------------------------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------------------------

/// Commits `edits` in batches of `opts.batch_size`, each as a commit of the actor
/// `system:import`.
async fn commit_in_batches(
    store: &EditStore,
    def_version: i64,
    file: &Utf8Path,
    source_digest: &str,
    edits: Vec<Edit>,
    opts: &ImportOptions,
    report: &mut ImportReport,
) -> Result<(), ImportError> {
    let file_name = file.file_name().unwrap_or(file.as_str());
    let mut edits = edits.into_iter();
    let mut batch_index = 0_usize;
    loop {
        let batch: Vec<Edit> = edits.by_ref().take(opts.batch_size).collect();
        if batch.is_empty() {
            return Ok(());
        }
        let commit = Commit {
            actor: Actor::System(SystemActor::Import),
            action: "system:import".to_owned(),
            def_version,
            params: json!({ "file": file_name, "source_digest": source_digest, "batch": batch_index, "rows": batch.len() }),
            edits: batch,
        };
        match store.commit(commit).await {
            Ok(receipt) => report.commits.push(receipt.seq),
            Err(error) => {
                return Err(ImportError::Partial {
                    committed: std::mem::take(&mut report.commits),
                    failed_batch: batch_index,
                    error: Box::new(error),
                });
            }
        }
        batch_index += 1;
    }
}
