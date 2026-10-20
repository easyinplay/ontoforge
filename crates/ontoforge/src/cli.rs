//! Command line definition.

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};
use ontoforge_storage::migrate::RenameDecl;

use crate::render::{ColorChoice, Format};

#[derive(Debug, Parser)]
#[command(
    name = "ontoforge",
    version,
    about = "Validate and lint ontology-as-code"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check an ontology directory for structural errors.
    Validate(ValidateArgs),
    /// Validate, then run the lint rules.
    Lint(LintArgs),
    /// Work with the JSON Schema of the definition format.
    Schema(SchemaArgs),
    /// Bring the edit store's tables in line with the definitions.
    Reload(ReloadArgs),
    /// Review (`--plan`) and run (`--apply`) a destructive definition change that `reload` refuses.
    Migrate(MigrateArgs),
    /// Review (`plan`) and write (`apply`) the roles and role bindings of a scope directory.
    Scope(ScopeArgs),
    /// Issue a development token signed with a local key (accepted only by a server started
    /// with `--dev`).
    DevToken(DevTokenArgs),
}

#[derive(Debug, Args)]
pub struct SchemaArgs {
    #[command(subcommand)]
    pub command: SchemaCommand,
}

#[derive(Debug, Subcommand)]
pub enum SchemaCommand {
    /// Write the definition and manifest JSON Schemas for editors and LLMs.
    Export {
        /// Directory to write the schema files into; created when missing.
        #[arg(long, default_value = "schemas")]
        out_dir: Utf8PathBuf,
    },
}

#[derive(Debug, Args)]
pub struct LintArgs {
    /// Ontology directory containing `ontology.yaml`.
    pub dir: Utf8PathBuf,
    /// Lint configuration file; replaces `<DIR>/ontoforge.toml`.
    #[arg(long)]
    pub config: Option<Utf8PathBuf>,
    #[command(flatten)]
    pub profile: ProfileArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Ontology directory containing `ontology.yaml`.
    pub dir: Utf8PathBuf,
    /// Also check the bindings of the selected profile against the engine.
    #[arg(long)]
    pub online: bool,
    /// Query timeout for the online check, in milliseconds (at least 1).
    #[arg(
        long,
        value_name = "MS",
        requires = "online",
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    pub timeout: Option<u64>,
    /// Report duplicate and NULL primary keys of every bound object type (full table scan;
    /// explicit only).
    #[arg(long, requires = "online")]
    pub check_pk: bool,
    /// Server configuration with the Trino connection and the profile to engine mapping.
    #[arg(long, value_name = "FILE", requires = "online")]
    pub server_config: Option<Utf8PathBuf>,
    #[command(flatten)]
    pub profile: ProfileArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct ReloadArgs {
    /// Ontology directory containing `ontology.yaml`.
    pub dir: Utf8PathBuf,
    /// Server configuration with the `[store]` section.
    #[arg(long, value_name = "FILE")]
    pub server_config: Utf8PathBuf,
    /// Print the DDL that would run and change nothing.
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub profile: ProfileArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
#[command(group(clap::ArgGroup::new("mode").required(true).args(["plan", "apply"])))]
pub struct MigrateArgs {
    /// Ontology directory containing `ontology.yaml`.
    pub dir: Utf8PathBuf,
    /// Server configuration with the `[store]` section.
    #[arg(long, value_name = "FILE")]
    pub server_config: Utf8PathBuf,
    /// Print the steps of the migration and their hash; change nothing.
    #[arg(long)]
    pub plan: bool,
    /// Run the migration whose hash is `--plan-hash`. The plan is made again first and the run
    /// is refused if its hash differs.
    #[arg(long, requires = "plan_hash")]
    pub apply: bool,
    /// The `plan hash` that `--plan` printed.
    #[arg(long, value_name = "HASH", requires = "apply")]
    pub plan_hash: Option<String>,
    /// Declare a rename, repeatable: `Type.old=new` for a property, `OldType=NewType` for an
    /// object type. The data moves to the new name; without a declaration the old column is
    /// archived and the new one starts empty.
    #[arg(
        long,
        value_name = "DECL",
        value_parser = |text: &str| text.parse::<RenameDecl>()
    )]
    pub rename: Vec<RenameDecl>,
    /// Plan or run the removal of every archived column and table instead of a definition change.
    /// This deletes data.
    #[arg(long, conflicts_with = "rename")]
    pub drop_archived: bool,
    #[command(flatten)]
    pub profile: ProfileArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct ScopeArgs {
    #[command(subcommand)]
    pub command: ScopeCommand,
}

#[derive(Debug, Subcommand)]
pub enum ScopeCommand {
    /// Print what `apply` would change in the edit store, and the hash of that plan.
    Plan(ScopePlanArgs),
    /// Write the scope directory into the edit store if its plan still has `--plan-hash`.
    Apply(ScopeApplyArgs),
}

/// What `scope plan` and `scope apply` have in common.
#[derive(Debug, Args)]
pub struct ScopeSourceArgs {
    /// Scope directory with the `Role` and `RoleBinding` documents.
    pub dir: Utf8PathBuf,
    /// Ontology directory the scope is checked against (and the edit store has loaded).
    #[arg(long, value_name = "DIR")]
    pub ontology: Utf8PathBuf,
    /// Server configuration with the `[store]` section and `[auth.principal_attrs]`.
    #[arg(long, value_name = "FILE")]
    pub server_config: Utf8PathBuf,
    #[command(flatten)]
    pub profile: ProfileArgs,
    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Debug, Args)]
pub struct ScopePlanArgs {
    #[command(flatten)]
    pub source: ScopeSourceArgs,
}

#[derive(Debug, Args)]
pub struct ScopeApplyArgs {
    #[command(flatten)]
    pub source: ScopeSourceArgs,
    /// The `plan hash` that `scope plan` printed. The plan is made again first and the run is
    /// refused if its hash differs.
    #[arg(long, value_name = "HASH")]
    pub plan_hash: String,
}

#[derive(Debug, Args)]
pub struct DevTokenArgs {
    /// Server configuration with `[auth.principal_attrs]` and, optionally, `[auth.dev]`.
    #[arg(long, value_name = "FILE")]
    pub server_config: Utf8PathBuf,
    /// Subject of the token; the principal id becomes `ontoforge-dev|<SUB>`.
    #[arg(long)]
    pub sub: String,
    /// Comma separated group names.
    #[arg(long, value_name = "A,B")]
    pub groups: Option<String>,
    /// A principal attribute as `name=value`, repeatable. A list attribute takes comma
    /// separated elements. The name must be declared in `[auth.principal_attrs]`.
    #[arg(long, value_name = "NAME=VALUE")]
    pub attr: Vec<String>,
    /// Lifetime as a number and a unit: `m`, `h` or `d` (at most 30d).
    #[arg(long, value_name = "TTL", default_value = "8h")]
    pub ttl: String,
    /// Signing key file, created on first use. Defaults to `[auth.dev] key_file`.
    #[arg(long, value_name = "FILE")]
    pub key_file: Option<Utf8PathBuf>,
}

/// Which deployment profile to check the source bindings of.
#[derive(Debug, Args)]
pub struct ProfileArgs {
    /// Profile whose source bindings apply: the directory `bindings/<NAME>/`. Overrides
    /// `defaultProfile` in `ontology.yaml`.
    #[arg(long, env = "ONTOFORGE_PROFILE", value_name = "NAME")]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
pub struct OutputArgs {
    /// Diagnostic output format.
    #[arg(long, value_enum, default_value_t = Format::Human)]
    pub format: Format,
    /// When to use colors in human output.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
    /// Exit with code 1 when warnings are present.
    #[arg(long)]
    pub deny_warnings: bool,
}
