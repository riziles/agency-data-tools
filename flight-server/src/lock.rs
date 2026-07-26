use std::fs::{self, File, OpenOptions};
use std::io;

use fs2::FileExt;

/// Advisory file lock that the OS releases on process exit (crash-safe).
#[allow(dead_code)]
pub struct Lock {
    _file: File,
}

impl Lock {
    /// Take a shared lock — allows readers, blocks writers. Use for flight-server.
    pub fn read(data_dir: &std::path::PathBuf) -> io::Result<Self> {
        let lock_path = data_dir.join(".lock");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)?;
        file.try_lock_shared().map_err(|_| {
            let pid = fs::read_to_string(&lock_path).unwrap_or_default();
            io::Error::new(
                io::ErrorKind::WouldBlock,
                format!(
                    "add-quarter is running (lock held by PID {}) — stop it first",
                    pid.trim()
                ),
            )
        })?;
        // Write PID after lock is acquired
        {
            use std::io::Write;
            file.set_len(0)?; // clear any old PID
            write!(file, "{}", std::process::id())?;
            file.flush()?;
        }
        eprintln!("[lock] shared lock acquired on {}", lock_path.display());
        Ok(Lock { _file: file })
    }

    /// Take an exclusive lock — blocks readers and writers. Use for add-quarter.
    /// Also writes our PID so readers can report what's blocking.
    pub fn write(data_dir: &std::path::PathBuf) -> io::Result<Self> {
        let lock_path = data_dir.join(".lock");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true) // clear old PID
            .read(true)
            .write(true)
            .open(&lock_path)?;
        match file.try_lock_exclusive() {
            Ok(()) => {
                use std::io::Write;
                file.set_len(0)?;
                write!(file, "{}", std::process::id())?;
                file.flush()?;
                eprintln!("[lock] exclusive lock acquired on {}", lock_path.display());
                Ok(Lock { _file: file })
            }
            Err(_) => {
                let pid = fs::read_to_string(&lock_path).unwrap_or_default();
                Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    format!(
                        "Flight SQL server is running (lock held by PID {}) — stop it before adding data",
                        pid.trim()
                    ),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_blocks_exclusive() {
        let dir = std::path::PathBuf::from("/tmp/lock-test-1");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::remove_file(dir.join(".lock"));

        let shared = Lock::read(&dir).expect("shared lock");
        let result = Lock::write(&dir);
        assert!(result.is_err(), "exclusive should be blocked");
        drop(shared);
        let _excl = Lock::write(&dir).expect("exclusive after shared dropped");
    }
}
