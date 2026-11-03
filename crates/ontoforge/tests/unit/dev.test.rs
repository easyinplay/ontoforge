use super::*;

fn canonical_root(directory: &tempfile::TempDir) -> Utf8PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    }
    Utf8PathBuf::from_path_buf(fs::canonicalize(directory.path()).unwrap()).unwrap()
}

#[test]
fn lock_acquisition_rechecks_new_entries_before_initialization() {
    let directory = tempfile::tempdir().unwrap();
    let root = canonical_root(&directory);
    validate_directory(&root, false).unwrap();
    fs::write(root.join("user-data.txt"), "preserve").unwrap();
    assert!(prepare_locked(root.clone(), false).is_err());
    assert_eq!(
        fs::read_to_string(root.join("user-data.txt")).unwrap(),
        "preserve"
    );
    assert!(!root.join(MARKER_FILE).exists());
    assert!(!root.join(AUTH_KEY).exists());
}

#[test]
fn lock_acquisition_rechecks_marker_before_reset_deletes_assets() {
    let directory = tempfile::tempdir().unwrap();
    let root = canonical_root(&directory);
    drop(
        prepare(&DevCliOptions {
            data_dir: Some(root.clone()),
            reset: false,
        })
        .unwrap(),
    );
    let before = fs::read(root.join(AUTH_KEY)).unwrap();
    validate_directory(&root, true).unwrap();
    fs::write(root.join(MARKER_FILE), "foreign-directory").unwrap();
    assert!(prepare_locked(root.clone(), true).is_err());
    assert_eq!(fs::read(root.join(AUTH_KEY)).unwrap(), before);
    assert_eq!(
        fs::read_to_string(root.join(MARKER_FILE)).unwrap(),
        "foreign-directory"
    );
}
