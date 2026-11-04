use super::*;
use std::collections::BTreeMap;

fn key(version: i64) -> Key {
    Key {
        version,
        profile: Some("dev".to_owned().try_into().unwrap()),
    }
}

fn report() -> DriftView {
    DriftView::Checked(DriftReport {
        profile: "dev".to_owned().try_into().unwrap(),
        objects: BTreeMap::from([(
            "Flight".to_owned().try_into().unwrap(),
            TargetHealth::Healthy { warnings: vec![] },
        )]),
        links: BTreeMap::new(),
    })
}

#[test]
fn exact_key_isolates_new_version_and_keeps_inflight_view() {
    let shared = Shared::default();
    assert!(matches!(*shared.capture(&key(1)), DriftView::Pending));
    shared.publish(key(1), report());
    let inflight = shared.capture(&key(1));
    assert!(matches!(*shared.capture(&key(2)), DriftView::Pending));
    shared.publish(key(1), DriftView::EngineFailed);
    assert!(matches!(*shared.capture(&key(2)), DriftView::Pending));
    assert_eq!(
        inflight.check(&BindingTarget::ObjectType(
            "Flight".to_owned().try_into().unwrap()
        )),
        Ok(())
    );
    shared.publish(key(2), DriftView::CheckFailed);
    assert!(matches!(*shared.capture(&key(1)), DriftView::Pending));
    let mut other_profile = key(2);
    other_profile.profile = Some("prod".to_owned().try_into().unwrap());
    assert!(matches!(
        *shared.capture(&other_profile),
        DriftView::Pending
    ));
}

#[test]
fn owner_exit_cannot_leave_future_versions_pending_forever() {
    let shared = Arc::new(Shared::default());
    shared.capture(&key(1));
    shared.publish(key(1), report());
    drop(TaskExit(Arc::clone(&shared)));
    assert!(matches!(*shared.capture(&key(1)), DriftView::CheckFailed));
    assert!(matches!(*shared.capture(&key(2)), DriftView::CheckFailed));
}
