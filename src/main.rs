use mdcat::{render, RenderOptions};
use std::{
    env,
    ffi::OsString,
    fs,
    io::{self, IsTerminal, Read, Write},
    process::ExitCode,
};

const HELP: &str = "Readable Markdown in your terminal.

Usage: mdcat [OPTIONS] [FILE|-]

Read FILE, or standard input when FILE is omitted or '-'.

Options:
  -w, --width N       Wrap prose at N columns (1–1000)
      --color MODE    auto, always, or never (default: auto)
      --plain         Disable styles and terminal hyperlinks
      --no-links      Disable clickable terminal hyperlinks
  -h, --help          Print help
  -V, --version       Print version
      --             Treat remaining arguments as a file name

Examples:
  mdcat README.md
  cat notes.md | mdcat --width 72
  mdcat --color always README.md | less -R

NO_COLOR disables automatic styling. TERM=dumb produces plain output.
Code lines and unbroken words are preserved even when wider than --width.
";

#[derive(Clone, Copy)]
enum Color {
    Auto,
    Always,
    Never,
}

struct Args {
    file: Option<OsString>,
    width: Option<usize>,
    color: Color,
    links: bool,
}

enum Action {
    Help,
    Version,
    Render(Args),
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Action, String> {
    let mut args = args.into_iter();
    let mut result = Args {
        file: None,
        width: None,
        color: Color::Auto,
        links: true,
    };
    let mut positional = false;
    while let Some(arg) = args.next() {
        let text = arg.to_str().unwrap_or("");
        if !positional {
            match text {
                "--help" | "-h" => return Ok(Action::Help),
                "--version" | "-V" | "-v" => return Ok(Action::Version),
                "--" => {
                    positional = true;
                    continue;
                }
                "--plain" => {
                    result.color = Color::Never;
                    result.links = false;
                    continue;
                }
                "--no-links" => {
                    result.links = false;
                    continue;
                }
                "--width" | "-w" | "--color" => {
                    let value = args
                        .next()
                        .ok_or_else(|| format!("{text} requires a value"))?;
                    set_option(
                        &mut result,
                        text,
                        value.to_str().ok_or("option value must be valid UTF-8")?,
                    )?;
                    continue;
                }
                _ => {
                    if let Some(value) = text.strip_prefix("--width=") {
                        set_option(&mut result, "--width", value)?;
                        continue;
                    }
                    if let Some(value) = text.strip_prefix("--color=") {
                        set_option(&mut result, "--color", value)?;
                        continue;
                    }
                    if text.starts_with('-') && text != "-" {
                        return Err(format!("unknown option '{text}'"));
                    }
                }
            }
        }
        if result.file.replace(arg).is_some() {
            return Err("expected at most one input file".into());
        }
    }
    Ok(Action::Render(result))
}

fn set_option(args: &mut Args, option: &str, value: &str) -> Result<(), String> {
    match option {
        "--width" | "-w" => {
            args.width = Some(
                value
                    .parse::<usize>()
                    .ok()
                    .filter(|w| (1..=1000).contains(w))
                    .ok_or("width must be an integer between 1 and 1000")?,
            );
        }
        "--color" => {
            args.color = match value {
                "auto" => Color::Auto,
                "always" => Color::Always,
                "never" => Color::Never,
                _ => return Err("color must be auto, always, or never".into()),
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn write_output(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()
}

fn run() -> Result<(), (u8, String)> {
    let action = parse(env::args_os().skip(1))
        .map_err(|message| (2, format!("{message}\nTry 'mdcat --help' for usage.")))?;
    let output = match action {
        Action::Help => format!("mdcat {}\n\n{HELP}", env!("CARGO_PKG_VERSION")),
        Action::Version => format!("mdcat {}\n", env!("CARGO_PKG_VERSION")),
        Action::Render(args) => {
            let markdown = match args.file {
                Some(path) if path != "-" => fs::read_to_string(&path)
                    .map_err(|e| (1, format!("cannot read '{}': {e}", path.to_string_lossy())))?,
                _ => {
                    if io::stdin().is_terminal() {
                        return Err((2, "provide a file or pipe Markdown on standard input\nTry 'mdcat --help' for usage.".into()));
                    }
                    let mut buffer = String::new();
                    io::stdin()
                        .read_to_string(&mut buffer)
                        .map_err(|e| (1, format!("cannot read stdin: {e}")))?;
                    buffer
                }
            };
            let terminal = io::stdout().is_terminal();
            let color = match args.color {
                Color::Always => true,
                Color::Never => false,
                Color::Auto => {
                    terminal
                        && env::var_os("NO_COLOR").filter(|v| !v.is_empty()).is_none()
                        && env::var_os("TERM").is_none_or(|v| v != "dumb")
                }
            };
            let width = args.width.unwrap_or_else(|| {
                if terminal {
                    crossterm::terminal::size()
                        .map(|(w, _)| usize::from(w).clamp(1, 100))
                        .unwrap_or(80)
                } else {
                    80
                }
            });
            render(
                &markdown,
                &RenderOptions {
                    width,
                    color,
                    hyperlinks: color && args.links,
                },
            )
        }
    };
    match write_output(&output) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err((1, format!("cannot write output: {error}"))),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            let _ = writeln!(io::stderr().lock(), "mdcat: {message}");
            ExitCode::from(code)
        }
    }
}
