//! AC10 (SHOULD): systemd unit installs at the consistent bin path
//! `/usr/local/bin/wm-presence` (no cargo-bin drift).

#[test]
fn test_systemd_unit_bin_path() {
    let unit_content = include_str!("../wm-presence.service");
    assert!(
        unit_content.contains("/usr/local/bin/wm-presence"),
        "systemd unit must reference /usr/local/bin/wm-presence; got:\n{unit_content}"
    );
}
