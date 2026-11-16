//! Bundled CLI launcher.
//!
//! The desktop bundle ships `dshr` next to `dshr-desktop`, but the app bundle
//! path is not on `PATH`. `<home>/bin/dshr` is a thin launcher that runs that
//! binary with this home and profile, so the CLI works from a terminal — and
//! while the desktop application is closed — exactly like the bundled command
//! runtime of the reference desktop shell.

use std::path::{Path, PathBuf};

/// `<home>/bin/dshr` (`dshr.cmd` on Windows).
pub fn shim_path(home: &Path) -> PathBuf {
    home.join("bin")
        .join(if cfg!(windows) { "dshr.cmd" } else { "dshr" })
}

/// Writes (or refreshes) the launcher so a moved bundle is picked up at startup.
pub fn write_shim(home: &Path, profile: &str) -> Result<PathBuf, String> {
    let binary = crate::find_binary().map_err(|error| error.to_string())?;
    write_shim_for(home, profile, &binary)
}

fn write_shim_for(home: &Path, profile: &str, binary: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(home.join("bin")).map_err(|error| error.to_string())?;
    let path = shim_path(home);
    let text = if cfg!(windows) {
        format!(
            "@echo off\r\n\"{}\" --home \"{}\" --profile \"{}\" %*\r\n",
            binary.display(),
            home.display(),
            profile
        )
    } else {
        format!(
            "#!/bin/sh\nexec \"{}\" --home \"{}\" --profile \"{}\" \"$@\"\n",
            binary.display(),
            home.display(),
            profile
        )
    };
    std::fs::write(&path, text).map_err(|error| format!("write {}: {error}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("chmod {}: {error}", path.display()))?;
    }
    Ok(path)
}

/// Writes the launcher and links it into `~/.local/bin` when nothing else owns
/// that name. Returns the path that should be reported to the user.
pub fn install(home: &Path, profile: &str) -> Result<PathBuf, String> {
    let shim = write_shim(home, profile)?;
    let linked = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local").join("bin"))
        .and_then(|directory| link_into(&directory, &shim));
    Ok(linked.unwrap_or(shim))
}

fn link_into(directory: &Path, shim: &Path) -> Option<PathBuf> {
    std::fs::create_dir_all(directory).ok()?;
    let name = if cfg!(windows) { "dshr.cmd" } else { "dshr" };
    let link = directory.join(name);
    match std::fs::read_link(&link) {
        Ok(target) if target == shim => return Some(link),
        // A user-installed dshr (file or another link) always wins.
        Ok(_) => return None,
        Err(_) if link.exists() => return None,
        Err(_) => {}
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(shim, &link).ok()?;
        Some(link)
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home() -> PathBuf {
        std::env::temp_dir().join(format!("dshr-cli-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn writes_a_launcher_bound_to_the_home_and_profile() {
        let home = temp_home();
        let path = write_shim_for(&home, "work", Path::new("/usr/local/bin/dshr")).expect("shim");
        assert_eq!(
            path,
            home.join("bin")
                .join(if cfg!(windows) { "dshr.cmd" } else { "dshr" })
        );
        let text = std::fs::read_to_string(&path).expect("shim text");
        assert!(text.contains("--home"), "{text}");
        assert!(text.contains("--profile \"work\""), "{text}");
        assert!(text.contains("/usr/local/bin/dshr"), "{text}");
        assert!(text.contains(&home.display().to_string()), "{text}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "launcher is executable");
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(unix)]
    #[test]
    fn links_the_launcher_and_leaves_foreign_files_alone() {
        let home = temp_home();
        let shim =
            write_shim_for(&home, "default", Path::new("/usr/local/bin/dshr")).expect("shim");
        let bin = home.join(".local").join("bin");
        assert_eq!(
            link_into(&bin, &shim).as_deref(),
            Some(bin.join("dshr").as_path())
        );
        assert_eq!(
            link_into(&bin, &shim).as_deref(),
            Some(bin.join("dshr").as_path())
        );

        let other_home = temp_home();
        let other_shim =
            write_shim_for(&other_home, "default", Path::new("/opt/dshr")).expect("other shim");
        assert!(
            link_into(&bin, &other_shim).is_none(),
            "foreign link is kept"
        );

        let foreign = temp_home();
        std::fs::create_dir_all(foreign.join(".local").join("bin")).expect("dir");
        std::fs::write(foreign.join(".local/bin/dshr"), "#!/bin/sh\n").expect("file");
        assert!(link_into(&foreign.join(".local/bin"), &shim).is_none());
        assert_eq!(
            std::fs::read_to_string(foreign.join(".local/bin/dshr")).expect("kept"),
            "#!/bin/sh\n"
        );
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&other_home);
        let _ = std::fs::remove_dir_all(&foreign);
    }
}
