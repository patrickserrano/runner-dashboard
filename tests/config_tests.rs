use runner_mgr::github::RunnerScope;
use serial_test::serial;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[test]
#[serial]
fn test_config_save_and_load() {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path().join("runner-mgr");
    std::env::set_var("RUNNER_MGR_CONFIG_DIR", config_dir.to_str().unwrap());

    let config = runner_mgr::config::Config {
        github_pat: "ghp_test123".to_string(),
        github_user: "testuser".to_string(),
        runner_user: "github".to_string(),
        runner_os: "linux".to_string(),
        runner_arch: "x64".to_string(),
        instances_base: "/opt/github-runners".to_string(),
    };

    config.save().expect("save should succeed");

    // Verify file permissions
    let metadata = fs::metadata(runner_mgr::config::Config::config_file()).unwrap();
    let permissions = metadata.permissions();
    assert_eq!(
        permissions.mode() & 0o777,
        0o600,
        "config file should be readable only by owner"
    );

    let loaded = runner_mgr::config::Config::load().expect("load should succeed");
    assert_eq!(loaded.github_pat, "ghp_test123");
    assert_eq!(loaded.github_user, "testuser");
    assert_eq!(loaded.runner_user, "github");
    assert_eq!(loaded.runner_os, "linux");
    assert_eq!(loaded.runner_arch, "x64");
    assert_eq!(loaded.instances_base, "/opt/github-runners");

    std::env::remove_var("RUNNER_MGR_CONFIG_DIR");
}

#[test]
#[serial]
fn test_config_load_missing_file() {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path().join("nonexistent");
    std::env::set_var("RUNNER_MGR_CONFIG_DIR", config_dir.to_str().unwrap());

    let result = runner_mgr::config::Config::load();
    assert!(result.is_err(), "loading missing config should fail");

    let err_msg = format!("{:#}", result.unwrap_err());
    assert!(
        err_msg.contains("Not initialized"),
        "error should mention initialization: {err_msg}"
    );

    std::env::remove_var("RUNNER_MGR_CONFIG_DIR");
}

#[test]
#[serial]
fn test_config_dir_permissions() {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path().join("runner-mgr");
    std::env::set_var("RUNNER_MGR_CONFIG_DIR", config_dir.to_str().unwrap());

    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "linux".to_string(),
        runner_arch: "x64".to_string(),
        instances_base: "/opt/github-runners".to_string(),
    };

    config.save().unwrap();

    let dir_metadata = fs::metadata(&config_dir).unwrap();
    let dir_permissions = dir_metadata.permissions();
    assert_eq!(
        dir_permissions.mode() & 0o777,
        0o700,
        "config dir should be accessible only by owner"
    );

    std::env::remove_var("RUNNER_MGR_CONFIG_DIR");
}

#[test]
fn test_instance_dir_path() {
    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "linux".to_string(),
        runner_arch: "x64".to_string(),
        instances_base: "/opt/github-runners".to_string(),
    };

    let scope = RunnerScope::parse("myuser/myrepo").unwrap();
    let dir = config.instance_dir(&scope);
    assert_eq!(
        dir.to_str().unwrap(),
        "/opt/github-runners/instances/myuser__myrepo"
    );
}

#[test]
fn test_template_dir_path() {
    let config = runner_mgr::config::Config {
        github_pat: "ghp_test".to_string(),
        github_user: "user".to_string(),
        runner_user: "github".to_string(),
        runner_os: "linux".to_string(),
        runner_arch: "x64".to_string(),
        instances_base: "/opt/github-runners".to_string(),
    };

    let dir = config.template_dir();
    assert_eq!(dir.to_str().unwrap(), "/opt/github-runners/template");
}

#[test]
fn test_detect_os() {
    let os = runner_mgr::config::Config::detect_os();
    assert!(os == "linux" || os == "darwin");
}

#[test]
fn test_detect_arch() {
    let arch = runner_mgr::config::Config::detect_arch();
    assert!(arch == "x64" || arch == "arm64");
}
