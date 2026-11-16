//! Single-instance guard for the desktop shell.
//!
//! A second application process against the same home must not start a second
//! server: two shells would race for `desktop.env`, `desktop-profile`, the
//! safe-mode marker and the update helpers. The first process holds an
//! exclusive lock on `<home>/desktop.lock` for its whole lifetime; later launches
//! hand their folder argument to the running process through
//! `<home>/desktop-pending-folder` and exit.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

/// Holds the exclusive home lock until the process exits.
pub struct InstanceLock {
    _file: File,
}

impl InstanceLock {
    /// `Ok(None)` means another process owns this home.
    pub fn acquire(home: &Path) -> io::Result<Option<Self>> {
        std::fs::create_dir_all(home)?;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(lock_path(home))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self { _file: file })),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(error)) => Err(error),
        }
    }

    /// Retries for `timeout` so a restart can hand over from the exiting process.
    pub fn acquire_within(home: &Path, timeout: std::time::Duration) -> io::Result<Option<Self>> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(lock) = Self::acquire(home)? {
                return Ok(Some(lock));
            }
            if std::time::Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

fn lock_path(home: &Path) -> PathBuf {
    home.join("desktop.lock")
}

fn pending_path(home: &Path) -> PathBuf {
    home.join("desktop-pending-folder")
}

/// Records the folder a second launch asked for; `true` when something was written.
pub fn forward_launch(home: &Path, folder: Option<&Path>) -> io::Result<bool> {
    let Some(folder) = folder else {
        return Ok(false);
    };
    std::fs::write(pending_path(home), format!("{}\n", folder.display()))?;
    Ok(true)
}

/// Reads and consumes a folder forwarded by a second launch.
pub fn take_pending_folder(home: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(pending_path(home)).ok()?;
    let _ = std::fs::remove_file(pending_path(home));
    let path = PathBuf::from(text.trim());
    path.is_dir().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_home() -> PathBuf {
        std::env::temp_dir().join(format!("dshr-instance-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn second_instance_cannot_own_the_same_home() {
        let home = temp_home();
        let first = InstanceLock::acquire(&home).expect("first").expect("lock");
        assert!(InstanceLock::acquire(&home).expect("probe").is_none());
        drop(first);
        assert!(InstanceLock::acquire(&home).expect("after drop").is_some());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn waits_for_a_restart_to_hand_over_the_lock() {
        let home = temp_home();
        let first = InstanceLock::acquire(&home).expect("first").expect("lock");
        let started = std::time::Instant::now();
        assert!(
            InstanceLock::acquire_within(&home, std::time::Duration::from_millis(250))
                .expect("probe")
                .is_none()
        );
        assert!(started.elapsed() >= std::time::Duration::from_millis(250));

        let waiter_home = home.clone();
        let handover = std::thread::spawn(move || {
            InstanceLock::acquire_within(&waiter_home, std::time::Duration::from_secs(5))
                .expect("wait")
                .is_some()
        });
        std::thread::sleep(std::time::Duration::from_millis(200));
        drop(first);
        assert!(handover.join().expect("join"), "lock handed to the waiter");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn separate_homes_do_not_block_each_other() {
        let one = temp_home();
        let two = temp_home();
        let _first = InstanceLock::acquire(&one).expect("one").expect("lock");
        let _second = InstanceLock::acquire(&two).expect("two").expect("lock");
        let _ = std::fs::remove_dir_all(&one);
        let _ = std::fs::remove_dir_all(&two);
    }

    #[test]
    fn forwards_a_folder_to_the_running_instance() {
        let home = temp_home();
        let folder = home.join("workspace");
        std::fs::create_dir_all(&folder).expect("workspace");
        assert!(!forward_launch(&home, None).expect("no folder"));
        assert!(forward_launch(&home, Some(&folder)).expect("forward"));
        assert_eq!(
            take_pending_folder(&home).as_deref(),
            Some(folder.as_path())
        );
        assert!(take_pending_folder(&home).is_none(), "consumed once");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn ignores_a_forwarded_path_that_is_gone() {
        let home = temp_home();
        let folder = home.join("vanished");
        std::fs::create_dir_all(&home).expect("home");
        std::fs::write(pending_path(&home), format!("{}\n", folder.display())).expect("write");
        assert!(take_pending_folder(&home).is_none());
        let _ = std::fs::remove_dir_all(&home);
    }
}
