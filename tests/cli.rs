use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mdcat"));
    command.env_remove("NO_COLOR").env("TERM", "xterm-256color");
    command
}

fn run(args: &[&str], input: &[u8]) -> Output {
    let mut child = command()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).unwrap()
}
fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).unwrap()
}

#[test]
fn help_and_version_use_stdout() {
    for flag in ["--help", "-h", "--version", "-V", "-v"] {
        let output = run(&[flag], b"");
        assert!(output.status.success());
        assert!(stderr(&output).is_empty());
        assert!(stdout(&output).starts_with("mdcat 0.1.0"));
    }
}

#[test]
fn reads_stdin_implicitly_or_with_dash() {
    for args in [vec![], vec!["-"], vec!["--", "-"]] {
        let output = run(&args, b"**Hello**\n");
        assert!(output.status.success());
        assert_eq!(stdout(&output), "Hello\n");
        assert!(stderr(&output).is_empty());
    }
}

#[test]
fn reads_files_and_option_terminator() {
    for args in [vec!["examples/nested.md"], vec!["--", "examples/nested.md"]] {
        let output = run(&args, b"");
        assert!(output.status.success());
        assert!(stdout(&output).contains("Nested structures"));
    }
}

#[test]
fn reads_unicode_and_dash_prefixed_file_names() {
    let directory = std::env::temp_dir().join(format!("mdcat-names-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    for name in ["notes-café-東京.md", "-notes.md"] {
        let file = directory.join(name);
        std::fs::write(&file, "**Hello**.").unwrap();
        let output = command()
            .current_dir(&directory)
            .args(["--", name])
            .output()
            .unwrap();
        std::fs::remove_file(&file).unwrap();
        assert!(output.status.success());
        assert_eq!(stdout(&output), "Hello.\n");
    }
    std::fs::remove_dir(&directory).unwrap();
}

#[test]
fn width_accepts_separate_and_equals_forms() {
    for args in [vec!["--width", "10"], vec!["-w", "10"], vec!["--width=10"]] {
        let output = run(&args, b"one two three four");
        assert!(output.status.success());
        assert_eq!(stdout(&output), "one two\nthree four\n");
    }
}

#[test]
fn invalid_arguments_fail_with_usage_errors() {
    for args in [
        vec!["--wat"],
        vec!["--width"],
        vec!["--width=0"],
        vec!["--width=1001"],
        vec!["--width=abc"],
        vec!["--width=-1"],
        vec!["--color"],
        vec!["--color=maybe"],
        vec!["one.md", "two.md"],
    ] {
        let output = run(&args, b"");
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(stdout(&output).is_empty());
        assert!(stderr(&output).starts_with("mdcat: "));
        assert!(stderr(&output).contains("--help"));
    }
}

#[test]
fn missing_files_and_invalid_utf8_have_clear_errors() {
    let output = run(&["tests/does-not-exist.md"], b"");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("cannot read 'tests/does-not-exist.md'"));
    let output = run(&[], &[0xff, 0xfe]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("cannot read stdin"));
    assert!(!stderr(&output).contains("panicked"));
}

#[test]
fn redirected_output_is_plain_by_default() {
    let output = run(&[], b"# Title\n\n[site](https://example.com)");
    assert!(output.status.success());
    assert!(!stdout(&output).contains('\x1b'));
    assert!(stdout(&output).contains("site (https://example.com)"));
}

#[test]
fn explicit_style_and_link_policies_work() {
    let input = b"**bold** [site](https://example.com)";
    let output = run(&["--color=always"], input);
    assert!(stdout(&output).contains("\x1b[1m"));
    assert!(stdout(&output).contains("\x1b]8;;https://example.com"));
    for args in [
        vec!["--plain"],
        vec!["--color=never"],
        vec!["--color", "always", "--plain"],
    ] {
        assert!(!stdout(&run(&args, input)).contains('\x1b'));
    }
    let output = run(&["--color=always", "--no-links"], input);
    assert!(stdout(&output).contains("\x1b[1m"));
    assert!(!stdout(&output).contains("\x1b]8"));
}

#[test]
fn empty_input_succeeds_without_output() {
    let output = run(&[], b"");
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[cfg(unix)]
#[test]
fn closed_output_pipe_exits_cleanly() {
    let mut child = command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"# A heading\n\nSome Markdown.")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}

// macOS filesystems require valid Unicode names; Linux permits arbitrary bytes.
#[cfg(target_os = "linux")]
#[test]
fn accepts_non_utf8_file_names() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let directory = std::env::temp_dir().join(format!("mdcat-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join(OsString::from_vec(b"notes-\xff.md".to_vec()));
    std::fs::write(&file, "Hello.").unwrap();
    let output = command().arg(&file).output().unwrap();
    std::fs::remove_file(&file).unwrap();
    std::fs::remove_dir(&directory).unwrap();
    assert!(output.status.success());
    assert_eq!(stdout(&output), "Hello.\n");
}
