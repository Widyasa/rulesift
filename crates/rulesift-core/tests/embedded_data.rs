//! `data/` (packaged with the crate) must match the repo's canonical
//! `rules/builtin/` and `schemas/report.v1.json`, and each crate carries a
//! copy of the license texts (#109). Resync with
//! `RULESIFT_BLESS=1 cargo test -p rulesift-core --test embedded_data`.

use std::path::{Path, PathBuf};

fn pairs() -> Vec<(PathBuf, PathBuf)> {
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = krate.join("../..");
    let mut out = vec![(
        repo.join("schemas/report.v1.json"),
        krate.join("data/report.v1.json"),
    )];
    for e in std::fs::read_dir(repo.join("rules/builtin")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "yaml") {
            out.push((
                p.clone(),
                krate.join("data/rules").join(p.file_name().unwrap()),
            ));
        }
    }
    // license texts packaged in each crate (#109)
    for name in ["LICENSE-MIT", "LICENSE-APACHE"] {
        for c in ["rulesift-core", "rulesift"] {
            out.push((repo.join(name), repo.join("crates").join(c).join(name)));
        }
    }
    out
}

#[test]
fn packaged_data_matches_repo() {
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    if !krate.join("../../rules/builtin").is_dir() {
        return; // building from the crates.io package: data/ is the source
    }
    let bless = std::env::var_os("RULESIFT_BLESS").is_some();
    for (canon, copy) in pairs() {
        let want = std::fs::read(&canon).unwrap();
        if bless {
            std::fs::write(&copy, &want).unwrap();
        }
        assert!(
            std::fs::read(&copy).ok().as_deref() == Some(want.as_slice()),
            "{} is out of sync with {}; run `RULESIFT_BLESS=1 cargo test -p rulesift-core --test embedded_data`",
            copy.display(),
            canon.display()
        );
    }
    let copies = std::fs::read_dir(krate.join("data/rules")).unwrap().count();
    assert_eq!(copies + 1 + 4, pairs().len(), "stale files in data/rules");
}
