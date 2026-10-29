//! Persistent, embedded development runtime for `serve --dev`.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};

use camino::{Utf8Path, Utf8PathBuf};
use ontoforge_auth::scope::{ScopeInputs, attr_types, compile_scope};
use ontoforge_engine::AnyEngine;
use ontoforge_engine::config::load_server_config;
use ontoforge_engine::sqlite::{SqliteEngine, fixture::plan_fixture_tables};
use ontoforge_schema::{load_dir, load_scope_dir};
use ontoforge_storage::reload::{ReloadOptions, reload};
use ontoforge_storage::store::EditStore;

use crate::cli::DevCliOptions;
use crate::dev_assets;

const MARKER: &str = "ontoforge-dev-v1";
const MARKER_FILE: &str = ".ontoforge-dev-marker";
const LOCK_FILE: &str = ".ontoforge-dev.lock";
const CONFIG_FILE: &str = "server.toml";
const AUTH_KEY: &str = "dev-token.key";
const QUERY_KEY: &str = "query-signing.key";
const EDIT_DB: &str = "editstore.db";

pub(crate) struct DevState {
    pub(crate) root: Utf8PathBuf,
    pub(crate) config: Utf8PathBuf,
    _lock: DevLock,
}

pub(crate) fn prepare(options: &DevCliOptions) -> Result<DevState, String> {
    let root = absolute_root(options.data_dir.as_ref())?;
    fs::create_dir_all(root.as_std_path())
        .map_err(|e| format!("cannot create dev directory `{root}`: {e}"))?;
    let root = Utf8PathBuf::from_path_buf(
        fs::canonicalize(root.as_std_path())
            .map_err(|e| format!("cannot resolve dev directory `{root}`: {e}"))?,
    )
    .map_err(|_| "dev directory is not valid UTF-8".to_owned())?;
    reject_symlink(&root)?;

    let marker = root.join(MARKER_FILE);
    let initialized = marker.exists();
    if options.reset {
        let expected = marker_text(&root);
        if !initialized
            || reject_symlink(&marker).is_err()
            || fs::read_to_string(&marker).ok().as_deref() != Some(expected.as_str())
        {
            return Err("--reset requires an initialized ontoforge dev directory".to_owned());
        }
    } else if !initialized {
        let has_entries = fs::read_dir(root.as_std_path())
            .map_err(|e| format!("cannot inspect dev directory `{root}`: {e}"))?
            .next()
            .is_some();
        if has_entries {
            return Err(format!(
                "refusing non-empty directory without `{MARKER_FILE}`: `{root}`"
            ));
        }
    }

    let lock = DevLock::acquire(&root)?;
    if options.reset {
        remove_known_assets(&root)?;
    }
    write_marker(&root)?;
    dev_assets::write_to(&root).map_err(|e| format!("cannot write embedded dev assets: {e}"))?;
    ensure_key(&root.join(AUTH_KEY))?;
    ensure_key(&root.join(QUERY_KEY))?;
    let config = root.join(CONFIG_FILE);
    write_config(&config)?;
    eprintln!("dev data directory: {root}");
    eprintln!("dev config: {config}");
    Ok(DevState {
        root,
        config,
        _lock: lock,
    })
}

pub(crate) async fn seed_store(state: &DevState, store: &EditStore) -> Result<(), String> {
    if store.active_definition().await.is_some() {
        return Ok(());
    }
    let loaded = load_dir(&dev_assets::ontology_root(&state.root));
    let ontology = loaded
        .ontology
        .ok_or_else(|| "embedded dev ontology failed validation".to_owned())?;
    let (config, _) = load_server_config(&state.config)
        .map_err(|_| "generated dev configuration failed validation".to_owned())?;
    let auth = config
        .auth
        .clone()
        .unwrap_or_else(|| ontoforge_engine::config::AuthConfig {
            oidc: Vec::new(),
            principal_attrs: std::collections::BTreeMap::new(),
            dev: None,
        });
    reload(store, &ontology, &ReloadOptions::default())
        .await
        .map_err(|e| format!("cannot seed embedded dev definition: {e}"))?;
    let scope = load_scope_dir(&dev_assets::scope_root(&state.root))
        .scope
        .ok_or_else(|| "embedded dev scope failed validation".to_owned())?;
    let compiled = compile_scope(&ScopeInputs {
        ontology: &ontology,
        profile: None,
        attrs: &attr_types(Some(&auth)),
        scope: &scope,
        sources: None,
    })
    .map_err(|errors| format!("cannot seed embedded dev scope: {errors:?}"))?;
    let props = compiled.row_filter_properties();
    let plan = store
        .scope_plan(&scope, &props)
        .await
        .map_err(|e| format!("cannot plan embedded dev scope: {e}"))?;
    store
        .scope_apply(&scope, &props, &plan.hash, "serve --dev")
        .await
        .map_err(|e| format!("cannot apply embedded dev scope: {e}"))?;
    Ok(())
}

pub(crate) async fn build_engine(state: &DevState) -> Result<AnyEngine, String> {
    let loaded = load_dir(&dev_assets::ontology_root(&state.root));
    let ontology = loaded
        .ontology
        .ok_or_else(|| "embedded dev ontology failed validation".to_owned())?;
    let profile = ontology
        .default_profile
        .as_ref()
        .ok_or_else(|| "embedded dev ontology has no default profile".to_owned())?;
    let plan = plan_fixture_tables(&ontology, &loaded.sources, profile)
        .map_err(|errors| format!("cannot plan embedded dev fixtures: {errors:?}"))?;
    SqliteEngine::from_fixtures(&dev_assets::ontology_root(&state.root), &plan.tables)
        .await
        .map(AnyEngine::Sqlite)
        .map_err(|errors| format!("cannot load embedded dev fixtures: {errors:?}"))
}

fn write_config(path: &Utf8Path) -> Result<(), String> {
    let text = format!(
        "[store]\nkind = \"sqlite\"\npath = \"{EDIT_DB}\"\n\n[auth.principal_attrs.homeAirports]\nclaim = \"airports\"\ntype = \"string_list\"\n\n[auth.dev]\nkey_file = \"{AUTH_KEY}\"\n\n[query]\nsigning_key_file = \"{QUERY_KEY}\"\n"
    );
    if !path.exists() {
        fs::write(path, text)
            .map_err(|e| format!("cannot write dev configuration `{path}`: {e}"))?;
    }
    Ok(())
}

fn ensure_key(path: &Utf8Path) -> Result<(), String> {
    if path.exists() {
        let len = fs::metadata(path)
            .map_err(|e| format!("cannot inspect dev key `{path}`: {e}"))?
            .len();
        if len < 32 {
            return Err(format!("dev key `{path}` must hold at least 32 bytes"));
        }
        return Ok(());
    }
    let mut key = [0_u8; 32];
    getrandom::fill(&mut key).map_err(|e| format!("cannot generate dev signing key: {e}"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path.as_std_path())
        .map_err(|e| format!("cannot create dev key `{path}`: {e}"))?;
    file.write_all(&key)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("cannot persist dev key `{path}`: {e}"))
}

fn remove_known_assets(root: &Utf8Path) -> Result<(), String> {
    for file in [
        MARKER_FILE,
        CONFIG_FILE,
        AUTH_KEY,
        QUERY_KEY,
        EDIT_DB,
        "editstore.db-shm",
        "editstore.db-wal",
    ] {
        let path = root.join(file);
        if path.exists() {
            reject_symlink(&path)?;
            fs::remove_file(path.as_std_path())
                .map_err(|e| format!("cannot reset dev asset `{path}`: {e}"))?;
        }
    }
    for dir in ["ontology", "scope"] {
        let path = root.join(dir);
        if path.exists() {
            reject_symlink(&path)?;
            fs::remove_dir_all(path.as_std_path())
                .map_err(|e| format!("cannot reset dev asset `{path}`: {e}"))?;
        }
    }
    Ok(())
}

fn write_marker(root: &Utf8Path) -> Result<(), String> {
    fs::write(root.join(MARKER_FILE), marker_text(root))
        .map_err(|e| format!("cannot write dev marker: {e}"))
}

fn marker_text(root: &Utf8Path) -> String {
    format!("{MARKER}\nroot={root}\n")
}

fn absolute_root(path: Option<&Utf8PathBuf>) -> Result<Utf8PathBuf, String> {
    let raw = path
        .cloned()
        .unwrap_or_else(|| Utf8PathBuf::from(".ontoforge/dev"));
    let path = if raw.is_absolute() {
        raw
    } else {
        let cwd = std::env::current_dir()
            .map_err(|e| format!("cannot resolve current directory: {e}"))?;
        Utf8PathBuf::from_path_buf(cwd.join(raw.as_std_path()))
            .map_err(|_| "current directory is not valid UTF-8".to_owned())?
    };
    if path.exists() {
        reject_symlink(&path)?;
    }
    Ok(path)
}

fn reject_symlink(path: &Utf8Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path.as_std_path())
        .map_err(|e| format!("cannot inspect dev path `{path}`: {e}"))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("refusing symlinked dev path `{path}`"));
    }
    Ok(())
}

struct DevLock {
    path: Utf8PathBuf,
    _file: File,
}

impl DevLock {
    fn acquire(root: &Utf8Path) -> Result<Self, String> {
        let path = root.join(LOCK_FILE);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.as_std_path())
        {
            Ok(mut file) => {
                writeln!(file, "{}", std::process::id())
                    .map_err(|e| format!("cannot write dev lock: {e}"))?;
                Ok(Self { path, _file: file })
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                let mut text = String::new();
                let _ =
                    File::open(path.as_std_path()).and_then(|mut f| f.read_to_string(&mut text));
                let pid = text.trim().parse::<u32>().ok();
                if pid.is_some_and(pid_is_running) {
                    return Err(format!("another ontoforge dev instance owns `{root}`"));
                }
                fs::remove_file(path.as_std_path())
                    .map_err(|e| format!("cannot recover stale dev lock `{path}`: {e}"))?;
                Self::acquire(root)
            }
            Err(error) => Err(format!("cannot acquire dev lock `{path}`: {error}")),
        }
    }
}

impl Drop for DevLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.path.as_std_path());
    }
}

fn pid_is_running(pid: u32) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(true)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(true)
    }
}
