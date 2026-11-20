//! "Khởi động cùng macOS": a per-user LaunchAgent that opens VNKey at login.
//!
//! A LaunchAgent rather than `SMAppService`: it works on every macOS VNKey
//! supports (10.15+, `SMAppService` needs 13), needs no extra framework, and
//! is one file the user can see and delete. launchd reads it at login, so
//! writing or removing it changes nothing in the running session.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const LABEL: &str = "com.vnkey.app";

fn agent_path(home: &Path) -> PathBuf {
    home.join("Library/LaunchAgents").join(format!("{LABEL}.plist"))
}

/// The login command: through LaunchServices for a bundle, so VNKey starts
/// as if double-clicked; the bare executable for an unbundled dev build.
pub fn launch_command(app_path: &Path, exe: &Path) -> Vec<String> {
    if app_path.extension().is_some_and(|ext| ext == "app") {
        vec!["/usr/bin/open".into(), app_path.display().to_string()]
    } else {
        vec![exe.display().to_string()]
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn plist(args: &[String]) -> String {
    let args: String = args.iter().map(|arg| format!("\t\t<string>{}</string>\n", xml_escape(arg))).collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>Label</key>\n\
         \t<string>{LABEL}</string>\n\
         \t<key>ProgramArguments</key>\n\
         \t<array>\n\
         {args}\
         \t</array>\n\
         \t<key>RunAtLoad</key>\n\
         \t<true/>\n\
         \t<key>LimitLoadToSessionType</key>\n\
         \t<string>Aqua</string>\n\
         </dict>\n\
         </plist>\n"
    )
}

/// Write or remove the agent so it matches `enabled`, pointing at the copy of
/// VNKey that is running now (so moving the app fixes itself on next launch).
pub fn sync(enabled: bool, args: &[String]) -> io::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory"))?;
    sync_in(&home, enabled, args)
}

fn sync_in(home: &Path, enabled: bool, args: &[String]) -> io::Result<()> {
    let path = agent_path(home);
    if !enabled {
        return match fs::remove_file(&path) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        };
    }
    let content = plist(args);
    // Rewriting an identical file still makes macOS post a "background item
    // added" notice, so only write a change.
    if fs::read_to_string(&path).ok().as_deref() == Some(content.as_str()) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_launches_through_open_dev_build_runs_the_executable() {
        let exe = Path::new("/Applications/VNKey.app/Contents/MacOS/vnkey");
        assert_eq!(
            launch_command(Path::new("/Applications/VNKey.app"), exe),
            vec!["/usr/bin/open", "/Applications/VNKey.app"]
        );
        let dev = Path::new("/repo/target/debug/vnkey");
        assert_eq!(launch_command(Path::new("/repo/target/debug"), dev), vec!["/repo/target/debug/vnkey"]);
    }

    #[test]
    fn plist_escapes_paths_and_runs_at_login() {
        let text = plist(&["/usr/bin/open".into(), "/Users/a&b/VNKey <dev>.app".into()]);
        assert!(text.contains("<string>/Users/a&amp;b/VNKey &lt;dev&gt;.app</string>"));
        assert!(text.contains("<key>RunAtLoad</key>\n\t<true/>"));
        assert!(text.contains(&format!("<string>{LABEL}</string>")));
    }

    #[test]
    fn sync_writes_then_removes_the_agent() {
        let home = std::env::temp_dir().join(format!("vnkey-autostart-test-{}", std::process::id()));
        let args = vec!["/usr/bin/open".to_string(), "/Applications/VNKey.app".to_string()];

        sync_in(&home, true, &args).unwrap();
        assert_eq!(fs::read_to_string(agent_path(&home)).unwrap(), plist(&args));

        sync_in(&home, false, &args).unwrap();
        assert!(!agent_path(&home).exists());
        // Already absent is not an error.
        sync_in(&home, false, &args).unwrap();

        fs::remove_dir_all(&home).unwrap();
    }
}
