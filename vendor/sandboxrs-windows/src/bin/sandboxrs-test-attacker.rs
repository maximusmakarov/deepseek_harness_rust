use std::fs;
use std::io::Write;
use std::process::Command;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::ReadFile;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    CreateProcessW, ResumeThread, CREATE_BREAKAWAY_FROM_JOB, CREATE_NEW_PROCESS_GROUP,
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, DETACHED_PROCESS,
    PROCESS_INFORMATION, STARTUPINFOW,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("read") if args.len() == 2 => {
            let path = &args[1];
            match fs::read(path) {
                Ok(bytes) => {
                    std::io::stdout().write_all(&bytes).unwrap();
                    std::process::exit(0);
                }
                Err(err) => {
                    eprintln!("read failed: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("write") if args.len() == 2 => match fs::write(&args[1], b"attacker") {
            Ok(()) => std::process::exit(0),
            Err(err) => {
                eprintln!("write failed: {err}");
                std::process::exit(1);
            }
        },
        Some("delete") if args.len() == 2 => match fs::remove_file(&args[1]) {
            Ok(()) => std::process::exit(0),
            Err(err) => {
                eprintln!("delete failed: {err}");
                std::process::exit(1);
            }
        },
        Some("move") if args.len() == 3 => match fs::rename(&args[1], &args[2]) {
            Ok(()) => std::process::exit(0),
            Err(err) => {
                eprintln!("move failed: {err}");
                std::process::exit(1);
            }
        },
        Some("link") if args.len() == 3 => match fs::hard_link(&args[1], &args[2]) {
            Ok(()) => std::process::exit(0),
            Err(err) => {
                eprintln!("link failed: {err}");
                std::process::exit(1);
            }
        },
        Some("spawn-read") if args.len() == 2 => forward(&["read", &args[1]]),
        Some("spawn-write") if args.len() == 2 => forward(&["write", &args[1]]),
        Some("spawn-delete") if args.len() == 2 => forward(&["delete", &args[1]]),
        Some("spawn-move") if args.len() == 3 => forward(&["move", &args[1], &args[2]]),
        Some("grandchild-read") if args.len() == 2 => forward(&["spawn-read", &args[1]]),
        Some("grandchild-write") if args.len() == 2 => forward(&["spawn-write", &args[1]]),
        Some("grandchild-delete") if args.len() == 2 => forward(&["spawn-delete", &args[1]]),
        Some("grandchild-move") if args.len() == 3 => forward(&["spawn-move", &args[1], &args[2]]),
        Some("great-grandchild-read") if args.len() == 2 => forward(&["grandchild-read", &args[1]]),
        Some("great-grandchild-write") if args.len() == 2 => {
            forward(&["grandchild-write", &args[1]])
        }
        Some("great-grandchild-delete") if args.len() == 2 => {
            forward(&["grandchild-delete", &args[1]])
        }
        Some("env") if args.len() == 2 => match std::env::var(&args[1]) {
            Ok(value) => {
                println!("{value}");
                std::process::exit(0);
            }
            Err(_) => {
                eprintln!("env missing");
                std::process::exit(1);
            }
        },
        Some("sleep") if args.len() <= 2 => {
            let seconds = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(30);
            std::thread::sleep(std::time::Duration::from_secs(seconds));
            std::process::exit(0);
        }
        Some("allocate-memory") if args.len() <= 2 => {
            let bytes: usize = args
                .get(1)
                .and_then(|v| v.parse::<usize>().ok())
                .map(|mb| mb * 1024 * 1024)
                .unwrap_or(128 * 1024 * 1024);
            let mut block = vec![0u8; bytes];
            block[0] = 1;
            block[bytes - 1] = 2;
            std::thread::sleep(std::time::Duration::from_secs(5));
            std::process::exit(0);
        }
        Some("spawn-many") if args.len() <= 2 => {
            let count: u32 = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(5);
            let mut children = Vec::new();
            for _ in 0..count {
                children.push(
                    Command::new(std::env::current_exe().unwrap())
                        .arg("sleep")
                        .arg("5")
                        .spawn()
                        .unwrap(),
                );
            }
            for mut child in children {
                let _ = child.wait();
            }
            std::process::exit(0);
        }
        Some("spawn-child-sleep") if args.len() == 2 => {
            let mut child = Command::new(std::env::current_exe().unwrap())
                .arg("sleep")
                .arg("120")
                .spawn()
                .unwrap();
            fs::write(&args[1], child.id().to_string()).unwrap();
            let _ = child.wait();
            std::process::exit(0);
        }
        Some("spawn-breakaway") if args.len() == 3 => {
            #[cfg(windows)]
            {
                spawn_breakaway(&args[1], &args[2]);
            }
            #[cfg(not(windows))]
            {
                let _ = (&args[1], &args[2]);
                std::process::exit(2);
            }
        }
        Some("read-handle") if args.len() == 2 => {
            #[cfg(windows)]
            {
                read_handle(&args[1]);
            }
            #[cfg(not(windows))]
            {
                let _ = &args[1];
                std::process::exit(2);
            }
        }
        _ => {
            eprintln!(
                "usage: attacker read|write|delete|move|link ... | spawn-*|grandchild-*|great-grandchild-* | env NAME | sleep [S] | allocate-memory [MB] | spawn-many [N] | spawn-child-sleep PIDFILE | spawn-breakaway MODE PIDFILE | read-handle HEX"
            );
            std::process::exit(2);
        }
    }
}

fn forward(args: &[&str]) -> ! {
    let status = Command::new(std::env::current_exe().unwrap())
        .args(args)
        .status()
        .unwrap();
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(windows)]
fn spawn_breakaway(mode: &str, pidfile: &str) -> ! {
    let mut flags = CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT;
    match mode {
        "detached" => flags |= DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP,
        "breakaway" => flags |= CREATE_BREAKAWAY_FROM_JOB,
        "suspended" => flags |= CREATE_SUSPENDED,
        _ => std::process::exit(2),
    }
    let exe = std::env::current_exe().unwrap();
    let mut command_line: Vec<u16> = format!("\"{}\" sleep 120", exe.display())
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let exe_wide: Vec<u16> = exe
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: All buffers are valid and outlive the call; no stdio handles are
    // passed so no inherited-handle list is needed.
    let ok = unsafe {
        CreateProcessW(
            exe_wide.as_ptr(),
            command_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            flags,
            std::ptr::null(),
            std::ptr::null(),
            &si,
            &mut pi,
        )
    };
    if ok == 0 {
        eprintln!("breakaway spawn failed: {}", unsafe { GetLastError() });
        std::process::exit(1);
    }
    if mode == "suspended" {
        // SAFETY: pi.hThread is the just-created suspended thread handle.
        unsafe {
            ResumeThread(pi.hThread);
        }
    }
    fs::write(pidfile, pi.dwProcessId.to_string()).unwrap();
    // SAFETY: The parent no longer needs the process/thread handles.
    unsafe {
        let _ = CloseHandle(pi.hProcess);
        let _ = CloseHandle(pi.hThread);
    }
    std::process::exit(0);
}

#[cfg(windows)]
fn read_handle(value: &str) -> ! {
    let raw = u64::from_str_radix(value, 16).unwrap_or(0);
    let handle = raw as HANDLE;
    let mut buffer = [0u8; 4096];
    let mut read = 0u32;
    // SAFETY: The handle was provided by the eval parent. If it was not
    // inherited, ReadFile fails, which is the expected sandbox result.
    let ok = unsafe {
        ReadFile(
            handle,
            buffer.as_mut_ptr().cast(),
            buffer.len() as u32,
            &mut read,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        eprintln!("read-handle failed: {}", unsafe { GetLastError() });
        std::process::exit(1);
    }
    std::io::stdout()
        .write_all(&buffer[..read as usize])
        .unwrap();
    std::process::exit(0);
}
