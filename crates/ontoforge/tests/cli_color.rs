//! Phase 1 color environment contracts through the real CLI process.

use std::process::Command;

const INVALID: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ontologies/invalid/v04-unknown-field"
);

#[test]
fn color_environment_controls_human_output_and_never_colors_json() {
    let cases: &[(&str, &[(&str, &str)], &str, bool)] = &[
        ("forced color", &[("CLICOLOR_FORCE", "1")], "human", true),
        (
            "empty NO_COLOR does not suppress forced color",
            &[("NO_COLOR", ""), ("CLICOLOR_FORCE", "1")],
            "human",
            true,
        ),
        (
            "NO_COLOR overrides forced color",
            &[("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")],
            "human",
            false,
        ),
        (
            "CLICOLOR zero suppresses color",
            &[("CLICOLOR", "0")],
            "human",
            false,
        ),
        (
            "dumb terminal suppresses color",
            &[("TERM", "dumb")],
            "human",
            false,
        ),
        (
            "JSON ignores forced color",
            &[("CLICOLOR_FORCE", "1")],
            "json",
            false,
        ),
    ];

    for &(name, env, format, colored) in cases {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ontoforge"));
        command.args(["validate", INVALID, "--color", "auto", "--format", format]);
        for key in [
            "NO_COLOR",
            "CLICOLOR",
            "CLICOLOR_FORCE",
            "FORCE_COLOR",
            "TERM",
            "TERM_PROGRAM",
            "COLORTERM",
            "CI",
            "ONTOFORGE_PROFILE",
        ] {
            command.env_remove(key);
        }
        let output = command.envs(env.iter().copied()).output().expect("run CLI");
        assert_eq!(output.status.code(), Some(1), "{name}: {output:?}");
        let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
        assert!(stdout.contains("ONT-V04"), "{name}: {stdout:?}");
        assert_eq!(stdout.contains('\u{1b}'), colored, "{name}: {stdout:?}");
        if format == "json" {
            let diagnostics: Vec<serde_json::Value> =
                serde_json::from_str(&stdout).expect("JSON diagnostics");
            assert_eq!(diagnostics.len(), 1, "{name}");
            assert_eq!(diagnostics[0]["code"], "ONT-V04", "{name}");
            assert_eq!(diagnostics[0]["file"], "objects.yaml", "{name}");
        }
    }
}
