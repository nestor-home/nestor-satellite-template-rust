# Nestor satellite template, Rust

A working [Nestor](https://nestorhome.dev) satellite to start your own device from: a microphone,
a speaker and a status light, on the [`nestor-satellite`](https://crates.io/crates/nestor-satellite)
crate. It pairs with a Nestor server, streams audio both ways, shows the assistant's attitude and
blinks when a person looks for it. Its CI runs the server's conformance suite on every change.

The light is printed: replace it with your hardware, and keep the rest.

## Use this template

On GitHub, choose **Use this template**, then **Create a new repository**. Or with the GitHub CLI:

```bash
gh repo create my-satellite --template nestor-home/nestor-satellite-template-rust --public --clone
cd my-satellite
cargo build
```

It needs Rust 1.91 or later, a C compiler and `cmake` (libopus is built from source). On Linux,
the microphone and the speaker also need ALSA: `sudo apt install libasound2-dev pkg-config`.

## Rename it

Everything to rename is spelled `my-satellite`:

```bash
grep -rn --exclude-dir=target my-satellite .
```

| What                  | Where                                                                  |
| --------------------- | ---------------------------------------------------------------------- |
| Crate name, authors   | `Cargo.toml` (`name`, `authors`, `description`): the binary's name too |
| Model id              | `MODEL` in `src/main.rs`: what your device is, as a slug               |
| Token folder, command | `NAME` in `src/main.rs`                                                |
| Conformance test      | `CARGO_BIN_EXE_my-satellite` in `tests/conformance.rs`                 |
| Release               | `package-name` in `release-please-config.json`                         |

Then `cargo build` (it updates `Cargo.lock`) and run the tests.

## Run it against your Nestor

On the Nestor server, make an enrollment code (or in the admin: **Satellites**, then **Add**):

```bash
nestor satellites add --room salon --name "Living room speaker"
```

It prints a pairing URI. On the device:

```bash
cargo run -- run --room salon --pairing-uri 'nestor+http://nestor.local:8471/?code=K7QF2M9X'
```

The first run pairs, then keeps its token in `~/.config/my-satellite/<id>.json`; later runs only
need the server's address:

```bash
cargo run -- run --core http://nestor.local:8471 --room salon
```

Run like this without a pairing URI, an unpaired device says a 6-digit code on its speaker and logs
it: type it in the admin, or `nestor satellites approve <code>` on the server.
`NESTOR_PAIRING_URI` works instead of `--pairing-uri`; `--null-audio` runs without a sound card;
`cargo run -- run --help` lists the rest. On the device itself, `cargo install --path .` installs
the `my-satellite` command.

## Plug your hardware

Look for `TODO(hardware)`:

- `src/light.rs`: the body. `behavior` (idle, listening, thinking, speaking) and `identify` print;
  drive your LEDs, screen or motors there. Calls must return at once: keep the state and animate
  from a task of your own.
- `src/main.rs`, `satellite()`: the default microphone and speaker. Pick others with
  `Microphone::named` and `Speaker::named`, or implement `AudioSource` / `AudioSink`.
- `src/main.rs`, `satellite()`: the manifest. Declare only what the device has.

| Capability                                     | Body method or SDK call                                   |
| ---------------------------------------------- | --------------------------------------------------------- |
| `IDENTIFY` (`device.identify`)                 | `identify()`                                              |
| `EXPRESSION` (`expression.emotion`)            | `expression(expression, intensity)`                       |
| `HEAD_MOTION` (`motion.head`)                  | `beat()`                                                  |
| `SPEECH_ACTIVITY` (`audio.speech_activity`)    | `satellite.observer().speech_activity(active)`            |
| `PERSON_DETECTION` (`vision.person_detection`) | `satellite.observer().person_detected(count, confidence)` |

A screen shows the device code with `.display(…)`; a keypad takes enrollment codes with
`.accepts_enrollment(…)` and `satellite.enroller()`. See the
[Rust SDK](https://nestorhome.dev/docs/sdks/rust/) and its API on
[docs.rs](https://docs.rs/nestor-satellite).

## Test

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cargo test
```

`cargo test` includes the server's
[conformance suite](https://nestorhome.dev/docs/satellites/conformance/), run through uvx
([uv](https://docs.astral.sh/uv/) must be installed, else the test is skipped) at the version in
`.nestor-version`: it launches `my-satellite run --null-audio` once per scenario and checks
pairing, refused codes, identify, acks, audio and reconnection. CI runs both on every pull request.

## Release

Pull request titles follow [Conventional Commits](https://www.conventionalcommits.org/)
(`feat: …`, `fix: …`), checked by `pr-title.yml`. From them, release-please keeps a release pull
request open; merging it tags the version and writes the changelog. Set your first version in
`Cargo.toml` and `.release-please-manifest.json`.

`release.yml` runs release-please through nestor-home's shared workflow, which only works in
nestor-home. In your repository:

1. In `release.yml`, replace the `release` job with the commented one under it. Under
   **Settings**, **Actions**, **General**, allow GitHub Actions to create pull requests.
2. In `Cargo.toml`, remove `publish = false` and add your `repository`.
3. On crates.io, publish a first version by hand (`cargo publish`), then in the crate's settings
   add a [trusted publisher](https://github.com/rust-lang/crates-io-auth-action): your repository, workflow
   `release.yml`, environment `crates-io`.
4. In your repository, create the environment `crates-io` (**Settings**, **Environments**), then
   uncomment the `crates-io` job. No token is stored: crates.io trusts the workflow.

## Documentation

- [Satellites](https://nestorhome.dev/docs/satellites/): capabilities and pairing.
- [Protocol](https://nestorhome.dev/docs/satellites/protocol/): the standard satellites follow.
- [Rust SDK](https://nestorhome.dev/docs/sdks/rust/): every option of the builder.
- [Conformance](https://nestorhome.dev/docs/satellites/conformance/).

## License

MIT
