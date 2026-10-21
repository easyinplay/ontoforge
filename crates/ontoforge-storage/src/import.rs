//! Bulk import of editOnly data: a CSV or JSON file becomes system edits (STORE-06, D-11).
//!
//! The import is not a second write path. Every batch is one [`Commit`] by the reserved actor
//! `system:import`, so it is validated, fenced against the active definition version, logged and
//! snapshotted exactly like any other commit (ADR-0026). Only editOnly object types and editOnly
//! many-to-many links can be imported: business data of a source backed type belongs in the lake
//! (ADR-0016), and an import that could write it would be a general write door around the Actions.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;

use camino::Utf8Path;
use ontoforge_schema::diag::{Diagnostic, Severity, codes};
use ontoforge_schema::ir::{
    Backing, LinkTypeName, ObjectType, ObjectTypeName, PropertyDef, PropertyName,
};
use sea_query::{Alias, Expr, ExprTrait, Query};
use serde_json::json;

use crate::commit::{
    Actor, Commit, Edit, EditError, ObjectOp, SystemActor, bind, cell_json, primary_key_def,
    read_expr,
};
use crate::error::StoreError;
use crate::physical::ObjectTable;
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
        ImportTarget::Link(link) => Err(ImportError::NotEditOnly(format!(
            "link `{}` cannot be imported yet",
            link.as_str()
        ))),
    }
}

/// One accepted data row.
struct Row {
    /// The line of the file (CSV), `0` when the format has no lines.
    line: u32,
    pk: PropertyValue,
    values: BTreeMap<PropertyName, PropertyValue>,
}

/// What the problems of a file are collected in.
struct Problems {
    file: String,
    found: Vec<Diagnostic>,
    /// More problems were found than are listed.
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

    fn is_empty(&self) -> bool {
        self.found.is_empty()
    }

    fn full(&self) -> bool {
        self.found.len() >= MAX_PROBLEMS
    }

    fn into_error(mut self) -> ImportError {
        if self.truncated {
            self.found.push(Diagnostic::without_location(
                codes::V27,
                Severity::Info,
                format!("more problems were found; only the first {MAX_PROBLEMS} are listed"),
                self.file.clone(),
            ));
        }
        ImportError::Data(self.found)
    }
}

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
    let shape = Shape::new(ty)?;

    let mut problems = Problems::new(file);
    let rows = match format_of(file) {
        Some(Format::Csv) => read_csv(&shape, file, opts, &mut problems),
        Some(Format::Json) => {
            problems.add(0, "a JSON file cannot be imported yet");
            Vec::new()
        }
        None => {
            problems.add(0, "the file must be a .csv or a .json file");
            Vec::new()
        }
    };
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

    if opts.on_existing != OnExisting::Error {
        problems.add(0, "only `--on-existing error` can be used yet");
        return Err(problems.into_error());
    }
    let existing = existing_objects(store, ty, table, &rows).await?;
    for row in &rows {
        if existing.contains_key(&key_of(&row.pk)) {
            problems.add(
                row.line,
                format!(
                    "object `{}` already exists; the import only creates objects",
                    show_key(&row.pk)
                ),
            );
        } else if row.values.is_empty() {
            problems.add(
                row.line,
                format!(
                    "object `{}` has no value besides its primary key, and an object cannot be \
                     created from a key alone",
                    show_key(&row.pk)
                ),
            );
        }
    }
    if !problems.is_empty() {
        return Err(problems.into_error());
    }

    let edits = rows
        .into_iter()
        .map(|row| Edit::Object {
            type_name: type_name.clone(),
            pk: row.pk,
            op: ObjectOp::Set(row.values),
        })
        .collect();
    report.created = report.rows;
    commit_in_batches(store, def.def_version, file, edits, opts, &mut report).await?;
    Ok(report)
}

/// Commits `edits` in batches of `opts.batch_size`, each as a commit of the actor
/// `system:import`.
async fn commit_in_batches(
    store: &EditStore,
    def_version: i64,
    file: &Utf8Path,
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
            params: json!({ "file": file_name, "batch": batch_index, "rows": batch.len() }),
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

// ---------------------------------------------------------------------------------------------
// Reading a file
// ---------------------------------------------------------------------------------------------

/// A cell text for a message: short, so a file with a huge cell cannot flood the output.
fn shown(cell: &str) -> String {
    let mut text: String = cell.chars().take(SHOWN_CELL).collect();
    if cell.chars().count() > SHOWN_CELL {
        text.push('…');
    }
    text
}

/// The primary key as it is written in a message.
fn show_key(pk: &PropertyValue) -> String {
    match pk {
        PropertyValue::String(text) => shown(text),
        other => shown(&other.to_json().to_string()),
    }
}

/// The identity of a primary key: its canonical JSON text.
fn key_of(pk: &PropertyValue) -> String {
    pk.to_json().to_string()
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

/// What a column of the header is.
enum Slot<'a> {
    Pk(&'a PropertyDef),
    Prop(&'a PropertyName, &'a PropertyDef),
    Ignored,
}

/// The columns of a CSV file, checked against the object type.
struct Header<'a> {
    slots: Vec<Slot<'a>>,
    /// Required properties that have no column; reported once, not for every row.
    missing: BTreeSet<&'a PropertyName>,
}

/// Checks the header line: unknown and repeated columns, the primary key and the required
/// columns. `None` when no row can be told from another (no primary key column).
fn read_header<'a>(
    shape: &Shape<'a>,
    headers: &csv::StringRecord,
    problems: &mut Problems,
) -> Option<Header<'a>> {
    let pk_name = &shape.ty.primary_key;
    let mut slots = Vec::with_capacity(headers.len());
    let mut seen_columns = BTreeSet::new();
    for column in headers {
        if !seen_columns.insert(column.to_owned()) {
            problems.add(1, format!("the column `{}` appears twice", shown(column)));
            slots.push(Slot::Ignored);
            continue;
        }
        match shape.ty.properties.get_key_value(column) {
            Some((name, _)) if name == pk_name => slots.push(Slot::Pk(shape.pk_def)),
            Some((name, def)) => slots.push(Slot::Prop(name, def)),
            None => {
                let known: Vec<&str> = shape
                    .ty
                    .properties
                    .keys()
                    .map(PropertyName::as_str)
                    .collect();
                problems.add(
                    1,
                    format!(
                        "unknown column `{}`; the properties of `{}` are {}",
                        shown(column),
                        shape.ty.api_name.as_str(),
                        known.join(", ")
                    ),
                );
                slots.push(Slot::Ignored);
            }
        }
    }
    let has_pk = slots.iter().any(|slot| matches!(slot, Slot::Pk(_)));
    if !has_pk {
        problems.add(
            1,
            format!("the primary key column `{}` is missing", pk_name.as_str()),
        );
    }
    let mut missing = BTreeSet::new();
    for name in &shape.required {
        if !seen_columns.contains(name.as_str()) {
            problems.add(
                1,
                format!("the required column `{}` is missing", name.as_str()),
            );
            missing.insert(*name);
        }
    }
    has_pk.then_some(Header { slots, missing })
}

/// What the cells of one record came to.
struct Candidate {
    pk: Option<PropertyValue>,
    values: BTreeMap<PropertyName, PropertyValue>,
    /// No cell had a problem.
    ok: bool,
}

/// Converts the cells of one record; an empty cell is "not set". Records every problem.
fn read_cells(
    shape: &Shape<'_>,
    header: &Header<'_>,
    record: &csv::StringRecord,
    line: u32,
    problems: &mut Problems,
) -> Candidate {
    let pk_name = shape.ty.primary_key.as_str();
    let mut found = Candidate {
        pk: None,
        values: BTreeMap::new(),
        ok: true,
    };
    let mut failed: BTreeSet<&PropertyName> = BTreeSet::new();
    for (slot, cell) in header.slots.iter().zip(record.iter()) {
        match slot {
            Slot::Ignored => {}
            Slot::Pk(_) if cell.is_empty() => {
                problems.add(line, format!("the primary key `{pk_name}` is empty"));
                found.ok = false;
            }
            Slot::Pk(def) => match PropertyValue::from_text(def, cell) {
                Ok(value) => found.pk = Some(value),
                Err(error) => {
                    problems.add(
                        line,
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
                    problems.add(
                        line,
                        format!("column `{}`: `{}`: {error}", name.as_str(), shown(cell)),
                    );
                    failed.insert(name);
                    found.ok = false;
                }
            },
        }
    }
    for name in &shape.required {
        if !found.values.contains_key(*name)
            && !failed.contains(*name)
            && !header.missing.contains(*name)
        {
            problems.add(
                line,
                format!("the required property `{}` is empty", name.as_str()),
            );
            found.ok = false;
        }
    }
    found
}

/// Remembers the primary keys of a file and refuses a second row with the same key.
#[derive(Default)]
struct FirstSeen(HashMap<String, u32>);

impl FirstSeen {
    /// `false`, with a problem, when `pk` appeared before.
    fn admit(&mut self, pk: &PropertyValue, line: u32, problems: &mut Problems) -> bool {
        if let Some(first) = self.0.get(&key_of(pk)) {
            problems.add(
                line,
                format!(
                    "the primary key `{}` already appears on line {first}",
                    show_key(pk)
                ),
            );
            return false;
        }
        self.0.insert(key_of(pk), line);
        true
    }
}

/// Reads the rows of a CSV file. The first line names the columns; an empty cell is "not set".
fn read_csv(
    shape: &Shape<'_>,
    file: &Utf8Path,
    opts: &ImportOptions,
    problems: &mut Problems,
) -> Vec<Row> {
    let Some(handle) = open_limited(file, opts.max_bytes, problems) else {
        return Vec::new();
    };
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(handle.take(opts.max_bytes.saturating_add(1)));
    let headers = match reader.headers() {
        Ok(headers) => headers.clone(),
        Err(error) => {
            problems.add(
                line_of(error.position()),
                format!("cannot read the header: {error}"),
            );
            return Vec::new();
        }
    };
    if headers.is_empty() {
        problems.add(1, "the file has no header line");
        return Vec::new();
    }
    let Some(header) = read_header(shape, &headers, problems) else {
        return Vec::new();
    };

    let mut rows = Vec::new();
    let mut data_rows = 0_usize;
    let mut seen = FirstSeen::default();
    for record in reader.records() {
        if problems.full() {
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
        let line = line_of(record.position());
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
            problems.add(
                line,
                format!("the file has more than {} rows", opts.max_rows),
            );
            break;
        }
        let Candidate { pk, values, ok } = read_cells(shape, &header, &record, line, problems);
        let Some(pk) = pk else { continue };
        if seen.admit(&pk, line, problems) && ok {
            rows.push(Row { line, pk, values });
        }
    }
    rows
}

// ---------------------------------------------------------------------------------------------
// What exists already
// ---------------------------------------------------------------------------------------------

fn col(name: &str) -> Alias {
    Alias::new(name)
}

/// The objects of `rows` that have an edit row, by key, with whether they are tombstoned.
async fn existing_objects(
    store: &EditStore,
    ty: &ObjectType,
    table: &ObjectTable,
    rows: &[Row],
) -> Result<HashMap<String, bool>, ImportError> {
    let pk_def = primary_key_def(ty).map_err(ImportError::Edit)?;
    let mut tx = store.begin_write().await?;
    let found = lookup_objects(&mut tx, table, pk_def, rows).await;
    sql_exit::rollback(tx).await;
    Ok(found?)
}

async fn lookup_objects(
    tx: &mut WriteTx<'_>,
    table: &ObjectTable,
    pk_def: &PropertyDef,
    rows: &[Row],
) -> Result<HashMap<String, bool>, StoreError> {
    let backend = tx.backend();
    let schema = tx.schema_edit().map(str::to_owned);
    let (pk_read, pk_kind) = read_expr(&table.pk.column, &table.pk.ty, backend);
    let mut found = HashMap::new();
    for chunk in rows.chunks(LOOKUP_CHUNK) {
        let select = Query::select()
            .expr(pk_read.clone())
            .column(col("of_version"))
            .column(col("of_deleted"))
            .from(sql_exit::edit_table(schema.as_deref(), &table.table))
            .and_where(
                Expr::col(col(&table.pk.column))
                    .is_in(chunk.iter().map(|row| bind(&row.pk, &table.pk.ty, backend))),
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
            let json = cell_json(pk.clone(), &table.pk.ty)?;
            let pk = PropertyValue::from_json(pk_def, &json).map_err(|error| {
                StoreError::Corrupt(format!("a stored primary key is invalid: {error}"))
            })?;
            found.insert(key_of(&pk), *deleted);
        }
    }
    Ok(found)
}
