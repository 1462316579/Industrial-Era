//! The game's app bundle on macOS. Started as a bare executable, the game shows in the Dock and
//! in Cmd+Tab as "industrial_era-client" with the generic executable icon (winit's window icon does
//! nothing there). So the launcher starts it from a small IndustrialEra.app inside its version folder:
//! the Info.plist names it and the icon is the launcher's own. The programs inside are hard links
//! to the version's, so the bundle takes no disk space and the release's signature is untouched.
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// The bundle's folder in a version folder.
pub const APP: &str = "IndustrialEra.app";

/// The game, then industrial_era-ctl, which the game looks for beside itself (Copy Connection Details).
const PROGRAMS: [&str; 2] = ["industrial_era-client", "industrial_era-ctl"];

const ICON: &[u8] = include_bytes!("../../packaging/icons/industrial_era.icns");

/// Its own identifier, so macOS keeps it apart from the launcher (dev.industrial_era.launcher); the
/// games category lets macOS turn Game Mode on when it plays full screen.
const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDisplayName</key>
	<string>IndustrialEra</string>
	<key>CFBundleExecutable</key>
	<string>industrial_era-client</string>
	<key>CFBundleIconFile</key>
	<string>industrial_era</string>
	<key>CFBundleIdentifier</key>
	<string>dev.industrial_era.game</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>IndustrialEra</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>LSApplicationCategoryType</key>
	<string>public.app-category.games</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>NSMicrophoneUsageDescription</key>
	<string>__MIC_DESCRIPTION__</string>
</dict>
</plist>
"#;

/// Where `ensure` fills in the localized `NSMicrophoneUsageDescription` (native mods can open the
/// microphone for voice chat; without the key macOS would kill the game instead of asking).
const MIC_DESCRIPTION: &str = "__MIC_DESCRIPTION__";

/// Brings `version`'s IndustrialEra.app up to date and returns the game's program inside it. A program
/// replaced in the version folder (a reinstall, a build copied over) is linked again.
pub fn ensure(version: &Path) -> io::Result<PathBuf> {
    let contents = version.join(APP).join("Contents");
    let programs = contents.join("MacOS");
    fs::create_dir_all(&programs)?;
    fs::create_dir_all(contents.join("Resources"))?;
    // Filled here so the prompt follows the launcher's language: the game reads the bundle as
    // it starts, and the plist is not a place the game can localize.
    let plist = INFO_PLIST.replacen(
        MIC_DESCRIPTION,
        &crate::shortcut::xml(crate::i18n::tr(
            "Native mods can use your microphone for voice chat when you enable it.",
        )),
        1,
    );
    write_if_changed(&contents.join("Info.plist"), plist.as_bytes())?;
    write_if_changed(&contents.join("Resources/industrial_era.icns"), ICON)?;
    for (i, name) in PROGRAMS.into_iter().enumerate() {
        let (from, to) = (version.join(name), programs.join(name));
        // Releases before industrial_era-ctl shipped have only the game.
        if i > 0 && !from.exists() {
            continue;
        }
        if !same_file(&from, &to) {
            let _ = fs::remove_file(&to);
            fs::hard_link(&from, &to)?;
        }
    }
    Ok(programs.join(PROGRAMS[0]))
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if fs::read(path).ok().as_deref() != Some(bytes) {
        fs::write(path, bytes)?;
    }
    Ok(())
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::metadata(a), fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundle_links_the_version_programs_and_follows_a_replaced_one() {
        let version =
            std::env::temp_dir().join(format!("industrial_era-game-app-{}", std::process::id()));
        let _ = fs::remove_dir_all(&version);
        fs::create_dir_all(&version).unwrap();
        fs::write(version.join("industrial_era-client"), "game 1").unwrap();
        fs::write(version.join("industrial_era-ctl"), "ctl").unwrap();

        let program = ensure(&version).unwrap();
        assert_eq!(
            program,
            version.join("IndustrialEra.app/Contents/MacOS/industrial_era-client")
        );
        assert!(same_file(&program, &version.join("industrial_era-client")));
        assert!(same_file(
            &version.join("IndustrialEra.app/Contents/MacOS/industrial_era-ctl"),
            &version.join("industrial_era-ctl")
        ));
        let plist =
            fs::read_to_string(version.join("IndustrialEra.app/Contents/Info.plist")).unwrap();
        assert!(plist.contains("<string>IndustrialEra</string>"));
        // The mic prompt macOS needs before a native mod may open the microphone: a key with a
        // nonempty localized value, not the placeholder the template ships with.
        let (_, rest) = plist
            .split_once("<key>NSMicrophoneUsageDescription</key>")
            .expect("Info.plist has NSMicrophoneUsageDescription");
        let value = rest
            .split_once("</string>")
            .expect("the key has a string value")
            .0
            .split_once("<string>")
            .expect("an opening tag")
            .1;
        assert!(!value.is_empty());
        assert!(!plist.contains("__MIC_DESCRIPTION__"));
        assert_eq!(
            fs::read(version.join("IndustrialEra.app/Contents/Resources/industrial_era.icns"))
                .unwrap(),
            ICON
        );

        // A new build copied over the version's game: the bundle runs it, not the old one.
        fs::remove_file(version.join("industrial_era-client")).unwrap();
        fs::write(version.join("industrial_era-client"), "game 2").unwrap();
        assert_eq!(
            fs::read_to_string(ensure(&version).unwrap()).unwrap(),
            "game 2"
        );
        fs::remove_dir_all(&version).unwrap();
    }

    #[test]
    fn a_version_without_industrial_era_ctl_still_gets_its_bundle() {
        let version = std::env::temp_dir().join(format!(
            "industrial_era-game-app-old-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&version);
        fs::create_dir_all(&version).unwrap();
        fs::write(version.join("industrial_era-client"), "game").unwrap();
        assert!(ensure(&version).unwrap().exists());
        assert!(!version
            .join("IndustrialEra.app/Contents/MacOS/industrial_era-ctl")
            .exists());
        fs::remove_dir_all(&version).unwrap();
    }

    #[test]
    fn a_version_without_the_game_has_no_bundle_program() {
        let version = std::env::temp_dir().join(format!(
            "industrial_era-game-app-none-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&version);
        fs::create_dir_all(&version).unwrap();
        assert!(ensure(&version).is_err());
        fs::remove_dir_all(&version).unwrap();
    }

    #[test]
    fn xml_escapes_the_characters_plist_strings_must_not_carry() {
        assert_eq!(
            crate::shortcut::xml(r#"a & b < c > "d""#),
            r#"a &amp; b &lt; c &gt; &quot;d&quot;"#
        );
    }
}
