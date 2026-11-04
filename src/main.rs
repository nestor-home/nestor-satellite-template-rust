//! `my-satellite run`: this device as a satellite of a Nestor server.
//!
//!     my-satellite run --room salon --pairing-uri "nestor+http://nestor.local:8471/?code=K7QF2M9X"
//!     my-satellite run --core http://nestor.local:8471 --room salon
//!
//! The first run pairs: with the pairing URI or enrollment code it is given (flags, or
//! `NESTOR_PAIRING_URI` / `NESTOR_PAIRING_CODE`), else it says a device code on its speaker and logs
//! it, to type in Nestor. The token is then kept in `~/.config/my-satellite/<id>.json`, and later
//! runs reconnect with it.

mod light;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use nestor_satellite::devices::{Microphone, Speaker};
use nestor_satellite::{
    Device, Enrollment, EnrollmentCode, IDENTIFY, Language, Manifest, PairingUri, Satellite, Software, TokenStore,
};
use tracing_subscriber::EnvFilter;

use crate::light::Light;

const NAME: &str = "my-satellite";
/// TODO(rename): what your device is, declared to Nestor (a slug).
const MODEL: &str = "my-satellite";

/// A Nestor satellite.
#[derive(Parser)]
#[command(name = NAME, version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Pair if needed, then stay connected.
    Run(Run),
}

#[derive(clap::Args)]
struct Run {
    /// The server's voice address, e.g. http://nestor.local:8471 (default: the pairing URI's).
    #[arg(long, env = "NESTOR_CORE")]
    core: Option<String>,
    /// Pair with this pairing URI, made in Nestor: nestor+http://host:port/?code=K7QF2M9X.
    #[arg(long, env = "NESTOR_PAIRING_URI", conflicts_with = "pairing_code")]
    pairing_uri: Option<PairingUri>,
    /// Pair with this enrollment code (K7QF-2M9X), the server given by --core.
    #[arg(long, env = "NESTOR_PAIRING_CODE")]
    pairing_code: Option<EnrollmentCode>,
    /// The room the device is in (a slug: salon, cuisine…).
    #[arg(long)]
    room: String,
    /// This device's id (default: the host name, as a slug).
    #[arg(long)]
    id: Option<String>,
    /// Where the token is kept (default: ~/.config/my-satellite/<id>.json).
    #[arg(long)]
    token_file: Option<PathBuf>,
    /// The language it speaks to people: en or fr (default: from LANG, else en).
    #[arg(long, value_parser = parse_language)]
    language: Option<Language>,
    /// No sound card: send silence and drop the core's voice (conformance runs, CI).
    #[arg(long)]
    null_audio: bool,
    /// Log more, down to each step of the link (`RUST_LOG` takes precedence).
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let Command::Run(run) = Cli::parse().command;
    init_logging(run.verbose);
    let language = run.language.unwrap_or_else(Language::from_env);
    match satellite(run, language) {
        Ok(satellite) => match satellite.run_until(shutdown()).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => fail(&language.describe(&error)),
        },
        Err(error) => fail(&error),
    }
}

fn satellite(run: Run, language: Language) -> Result<Satellite, String> {
    let id = run.id.unwrap_or_else(default_id);
    let device = Device::new(&id, MODEL, run.room).map_err(|e| e.to_string())?;
    // IDENTIFY is the optional capability this example shows (Light::identify): declare only what
    // the device really has. Audio and text follow from what the builder is given: the microphone
    // (`source`), the speaker (`sink`) and `text_display` for the assistant's words.
    // The software is this crate, named as it is installed: Nestor shows it and tells about updates.
    let software = Software::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
    let manifest = Manifest::new(device, [IDENTIFY]).map_err(|e| e.to_string())?.with_software(software);
    let tokens = TokenStore::new(run.token_file.unwrap_or_else(|| format!("~/.config/{NAME}/{id}.json").into()));
    let enrollment: Option<Enrollment> = run.pairing_uri.map(Into::into).or(run.pairing_code.map(Into::into));
    let core = match (run.core, enrollment.as_ref().and_then(Enrollment::core)) {
        (Some(core), _) => core,
        (None, Some(core)) => core.to_owned(),
        (None, None) => return Err("give the server's address (--core, NESTOR_CORE) or a pairing URI".to_owned()),
    };
    // Without a screen, drop `text_display`: the device then only speaks.
    let mut builder =
        Satellite::builder(core, manifest, tokens).body(Light).text_display(light::show).language(language);
    if let Some(enrollment) = enrollment {
        builder = builder.enrollment(enrollment);
    }
    let built = if run.null_audio {
        builder.null_audio().build() // silence is sent, the core's voice is decoded and dropped
    } else {
        // TODO(hardware): the system's default microphone and speaker, through cpal. Pick others
        // with Microphone::named / Speaker::named, or implement AudioSource / AudioSink.
        builder.source(Microphone::new()).sink(Speaker::new()).build()
    };
    built.map_err(|error| language.describe(&error))
}

fn default_id() -> String {
    let host = gethostname::gethostname().to_string_lossy().to_lowercase();
    let first = host.split('.').next().unwrap_or_default();
    let slug: String =
        first.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '-' }).collect();
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "satellite" } else { slug };
    slug.chars().take(64).collect()
}

fn parse_language(tag: &str) -> Result<Language, String> {
    Language::parse(tag).ok_or_else(|| format!("{tag:?}: the languages are en and fr"))
}

fn init_logging(verbose: bool) {
    // webrtc-rs logs every unreachable candidate as an error: the SDK reports what matters. The
    // device code and the pairing steps are logged at info: they are how a headless device is paired.
    let default = if verbose { "off,nestor_satellite=debug" } else { "off,nestor_satellite=info" };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).init();
}

/// Ctrl-C, or SIGTERM (systemd, the conformance suite) on Unix.
async fn shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn fail(error: &dyn std::fmt::Display) -> ExitCode {
    eprintln!("{NAME}: {error}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use nestor_satellite::{AUDIO_CAPTURE, AUDIO_PLAYBACK, TEXT_DISPLAY};

    use super::*;

    fn run(args: &[&str]) -> Run {
        let Command::Run(run) = Cli::parse_from([NAME, "run"].iter().chain(args)).command;
        run
    }

    #[test]
    fn a_pairing_uri_gives_the_servers_address() {
        let satellite = satellite(
            run(&[
                "--room",
                "salon",
                "--null-audio",
                "--pairing-uri",
                "nestor+http://nestor.local:8471/?code=K7QF2M9X",
            ]),
            Language::En,
        );
        assert!(satellite.is_ok());
    }

    #[test]
    fn it_declares_the_light_under_its_model() {
        let args = ["--core", "http://nestor.local:8471", "--room", "salon", "--id", "enceinte", "--null-audio"];
        let satellite = satellite(run(&args), Language::En).unwrap();
        let manifest = satellite.manifest();
        assert_eq!(manifest.device().id(), "enceinte");
        assert_eq!(manifest.device().model(), MODEL);
        assert_eq!(manifest.software().name(), "my-satellite");
        assert_eq!(manifest.software().version(), env!("CARGO_PKG_VERSION"));
        for capability in [IDENTIFY, AUDIO_CAPTURE, AUDIO_PLAYBACK, TEXT_DISPLAY] {
            assert!(manifest.capabilities().iter().any(|c| c.name() == capability), "{capability}");
        }
    }

    #[test]
    fn without_the_servers_address_it_says_what_is_missing() {
        let error = satellite(run(&["--room", "salon", "--null-audio"]), Language::En).err().unwrap();
        assert!(error.contains("--core"), "{error}");
    }

    #[test]
    fn a_malformed_enrollment_code_is_refused_before_anything_runs() {
        let args = [NAME, "run", "--core", "http://c", "--room", "salon", "--pairing-code", "nope"];
        assert!(Cli::try_parse_from(args).is_err());
    }
}
