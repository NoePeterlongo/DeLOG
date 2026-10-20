use std::path::{Path, PathBuf};

pub fn bundled_python_home(exe_path: &Path) -> PathBuf {
    exe_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("python")
}

/// `<home>/lib/pythonX.Y/site-packages` of the bundled runtime, discovered
/// from the directory listing so the pinned Python version stays
/// unstated here.
pub fn bundled_site_packages(home: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(home.join("lib")).ok()?;
    let python_dir = entries.filter_map(Result::ok).find(|entry| {
        let name = entry.file_name();
        entry.path().is_dir() && name.to_string_lossy().starts_with("python")
    })?;
    Some(python_dir.path().join("site-packages"))
}

pub fn init_bundled_python() {
    let exe = std::env::current_exe().expect("current_exe() failed");
    let home = bundled_python_home(&exe);
    if !home.is_dir() {
        eprintln!(
            "fatal: bundled Python runtime not found at {}\n\
             This install is incomplete - reinstall DeLOG.",
            home.display()
        );
        std::process::exit(1);
    }
    // The embedded interpreter honors PYTHONHOME, which locates the
    // standard library, but on this platform the site machinery never adds
    // the bundled site-packages to sys.path, leaving the pinned wheels
    // (numpy, scipy, ...) unreachable. Put them on the path explicitly;
    // setting PYTHONPATH (instead of clearing it) also shields the embedded
    // interpreter from any host PYTHONPATH.
    match bundled_site_packages(&home) {
        Some(site_packages) => {
            // SAFETY: called at the very top of main() while single-threaded,
            // before any other thread is spawned and before CPython initializes.
            unsafe {
                std::env::set_var("PYTHONHOME", &home);
                std::env::set_var("PYTHONPATH", &site_packages);
            }
        }
        // Without a recognizable site-packages layout, fall back to a
        // clean environment; the interpreter finds nothing either way.
        None => {
            // SAFETY: same single-threaded pre-init window as above.
            unsafe {
                std::env::set_var("PYTHONHOME", &home);
                std::env::remove_var("PYTHONPATH");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_is_python_dir_beside_exe() {
        let home = bundled_python_home(Path::new("/opt/delog/bin/delog"));
        assert_eq!(home, PathBuf::from("/opt/delog/bin/python"));
    }

    #[test]
    fn home_handles_bare_filename() {
        let home = bundled_python_home(Path::new("delog"));
        assert_eq!(home, PathBuf::from("python"));
    }

    #[test]
    fn site_packages_finds_the_python_lib_dir() {
        let root = std::env::temp_dir().join(format!(
            "delog-py-runtime-{}-{}",
            std::process::id(),
            line!()
        ));
        let site = root.join("python").join("lib").join("python3.12");
        std::fs::create_dir_all(&site).unwrap();
        assert_eq!(
            bundled_site_packages(&root.join("python")),
            Some(site.join("site-packages"))
        );
        assert_eq!(bundled_site_packages(&root.join("missing")), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
