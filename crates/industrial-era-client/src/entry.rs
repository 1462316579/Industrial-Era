//! Startup shared by the gameplay library and its executable.
use crate::{crash, game, logging, ram_limit};

/// Start the game. The executable supplies the allocator.
pub fn main() {
    start(|| {});
}

/// Running `after_log` once the log is open (Android's start-up notes, its
/// platform set-up that logs).
pub fn start(after_log: impl FnOnce()) {
    industrial_era_threads::configure_allocator();
    industrial_era_client::telemetry::mark_start();
    industrial_era_client::spans::startup::init(std::time::Instant::now());
    #[cfg(not(target_os = "android"))]
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The process has no command line: `industrial-era-args.txt`'s, read by `android::main`.
    #[cfg(target_os = "android")]
    let args: Vec<String> = crate::android::args();
    let game_args = args.iter().take_while(|a| *a != "--dedicated");
    let help = game_args
        .clone()
        .any(|a| a == "--help" || a == "-h" || a == "--launch-api");
    // --home before the log opens in it.
    if let Some(dir) = game_args.clone().skip_while(|a| *a != "--home").nth(1) {
        logging::set_game_home(std::path::absolute(dir).unwrap_or_else(|_| dir.into()));
    }
    // The dedicated server logs in its own folder (--game-dir's), as vanilla's does; the game in its home.
    let (home, side, thread) = if args.iter().any(|a| a == "--dedicated") {
        let folder = game_args
            .clone()
            .skip_while(|a| *a != "--game-dir" && *a != "--gameDir")
            .nth(1)
            .and_then(|dir| std::path::absolute(dir).ok())
            .unwrap_or_else(|| ".".into());
        (folder, "server", "Server thread")
    } else {
        // --player-dir too (a game's, not a server's): a player of a shared game folder keeps
        // its log and crash reports in its own; crash telemetry stays with the home's consent.
        if let Some(dir) = game_args
            .clone()
            .skip_while(|a| *a != "--player-dir")
            .nth(1)
        {
            logging::set_player_dir(std::path::absolute(dir).unwrap_or_else(|_| dir.into()));
        }
        (logging::game_home(), "client", "Render thread")
    };
    if !help {
        logging::init(
            logging::player_dir_override().unwrap_or(home.as_path()),
            thread,
        );
        // The log's first line names the build, so a bug report that carries the log says which one it was.
        log::info!("Industrial Era {}", crash::version());
        crash::install(home.clone(), side);
        ram_limit::start(&home, side);
    }
    after_log();
    raise_open_file_limit();
    if let Err(error) = game::run_with() {
        // A bad command line is the caller's mistake, not a crash: no report.
        let usage = error.is::<game::UsageError>();
        #[cfg(target_os = "android")]
        if !usage {
            crate::diagnostics::fatal_error(&error.to_string());
        }
        log::error!("Industrial Era: {error}");
        logging::flush();
        // The report's path is the last line on stderr (crash::SAVED_LINE).
        if !help && !usage {
            crash::report_error(&home, side, &*error);
        }
        // Started from Explorer, the console window closes with the game:
        // keep the message on screen until it is read.
        #[cfg(windows)]
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            eprintln!("Press Enter to close.");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        // The activity ends first, and the process without the C library's exit handlers.
        #[cfg(target_os = "android")]
        crate::android::finish_and_exit();
        #[cfg(not(target_os = "android"))]
        std::process::exit(if usage {
            game::exit_code::USAGE
        } else {
            game::exit_code::ERROR
        });
    }
    logging::flush();
}

/// The JVM raises its open-file limit to the hard limit as it starts
/// (`-XX:+MaxFDLimit`, on by default), so a vanilla server may hold as many
/// sockets and region files as the host allows, not the soft default (often
/// 1024 in a container). The same here; macOS caps it at `OPEN_MAX`, as the
/// JVM does there.
fn raise_open_file_limit() {
    #[cfg(unix)]
    {
        use rustix::process::{Resource, Rlimit, getrlimit, setrlimit};
        let limit = getrlimit(Resource::Nofile);
        let hard = if cfg!(target_os = "macos") {
            Some(limit.maximum.map_or(10_240, |h| h.min(10_240)))
        } else {
            limit.maximum
        };
        let raised = match (limit.current, hard) {
            (Some(soft), Some(hard)) => hard > soft,
            (Some(_), None) => true,
            (None, _) => false,
        };
        if raised {
            let to = Rlimit {
                current: hard,
                maximum: limit.maximum,
            };
            match setrlimit(Resource::Nofile, to) {
                Ok(()) => log::info!(
                    "Open file limit raised from {:?} to {hard:?}",
                    limit.current
                ),
                Err(e) => log::warn!("Open file limit stays at {:?}: {e}", limit.current),
            }
        }
    }
}
