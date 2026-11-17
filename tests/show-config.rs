//! Config profile test: runs the application as a subprocess and asserts its
//! output for the `show-config` command

use std::{fs, io::Read, process::Command, sync::LazyLock};

use abscissa_core::testing::prelude::*;
use insta::assert_snapshot;
use rustic_testing::TestResult;

// Storing this value as a [`Lazy`] static ensures that all instances of
// the runner acquire a mutex when executing commands and inspecting
// exit statuses, serializing what would otherwise be multithreaded
// invocations as `cargo test` executes tests in parallel by default.
pub static LAZY_RUNNER: LazyLock<CmdRunner> = LazyLock::new(|| {
    let mut runner = CmdRunner::new(env!("CARGO_BIN_EXE_rustic"));
    runner.exclusive().capture_stdout();
    runner
});

fn cmd_runner() -> CmdRunner {
    LAZY_RUNNER.clone()
}

#[test]
fn test_show_config_passes() -> TestResult<()> {
    {
        let mut runner = cmd_runner();

        let mut cmd = runner.args(["show-config"]).run();

        let mut output = String::new();

        cmd.stdout().read_to_string(&mut output)?;

        assert_snapshot!(output);

        cmd.wait()?.expect_success();
    }

    Ok(())
}

#[test]
fn test_use_profiles_do_not_loop() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    let dir_path = dir.path();

    let rustic_dir_path = dir_path.join("config").join("rustic");
    fs::create_dir_all(&rustic_dir_path)?;

    let profile_a = rustic_dir_path.join("a.toml");
    fs::write(&profile_a, "[global]\nuse-profiles = [\"b\"]\n")?;

    let profile_b = rustic_dir_path.join("b.toml");
    fs::write(&profile_b, "[global]\nuse-profiles = [\"a\"]\n")?;

    let output = Command::new(env!("CARGO_BIN_EXE_rustic"))
        .env("HOME", dir_path)
        .env("XDG_CONFIG_HOME", dir_path.join("config"))
        .env("XDG_CACHE_HOME", dir_path.join("cache"))
        .env("RUSTIC_NO_PROGRESS", "true")
        .args(["-P", "a", "show-config"])
        .output()?;

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("a -> b -> a"));

    Ok(())
}
