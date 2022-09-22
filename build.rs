extern crate bindgen;
extern crate git2;
extern crate regex;

use git2::{Repository, RemoteCallbacks};
use regex::Regex;
use std::env;
use std::fs::remove_dir_all;
use std::path::{Path, PathBuf};
use std::process::Command;

const LIBINJECTION_URL: &'static str = "git@github.com:libinjection/libinjection.git";
// const LIBINJECTION_URL: &'static str = "https://github.com/client9/libinjection";

// https://github.com/libinjection/libinjection.git
const BUILD_DIR_NAME: &'static str = "libinjection";

fn clone_libinjection(build_dir: &Path, version: &str) -> Option<()> {
    let ssh_key_path = "/Users/adrien/.ssh/no_p_id_rsa";

    // let ssh_key_path = env::var("GIT_SSH_KEY_PATH")
    // .expect("No GIT_SSH_KEY_PATH environment variable found");



    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        git2::Cred::ssh_key(
            username_from_url.unwrap(),
          None,
          std::path::Path::new(ssh_key_path),
          None,
        )
      });
    
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(callbacks);
    opts.download_tags(git2::AutotagOption::All);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(opts);
    builder.branch("main");

    match builder.clone(LIBINJECTION_URL, build_dir)  {

        Ok(repo) => {
            println!("{:?}",repo.path() );
            let rev = repo.revparse_single(version).ok()?;
            repo.set_head_detached(rev.id()).ok()
        }
        Err(e) => {
            println!("{:?}", e);
            None
        }
    }
   
}

fn run_make(rule: &str, cwd: &Path) -> bool {
    let output = Command::new("make")
        .arg(rule)
        .env("OUT_DIR", env::var("OUT_DIR").unwrap())
        .current_dir(cwd)
        .output()
        .unwrap();
    if output.status.success() {
        true
    } else {
        panic!("make error: {}", String::from_utf8_lossy(&output.stderr));
    }
}

fn fix_python_version() -> Option<()> {
    let output = Command::new("python").arg("-V").output().ok()?;
    let python_version = String::from_utf8_lossy(&output.stdout).to_string();
    if !Regex::new("Python 2.*")
        .ok()?
        .is_match(python_version.as_str())
    {
        let cwd = env::current_dir().ok()?;
        if !run_make("fix-python", cwd.as_path()) {
            return None;
        }
    }
    Some(())
}

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut build_parent_dir = out_path.join(BUILD_DIR_NAME);

    println!("{:?}",out_path );

    let _ = remove_dir_all(build_parent_dir.as_path());

    if clone_libinjection(build_parent_dir.as_path(), "v3.10.0").is_none() {
        panic!("unable to clone libinjection");
    }

    if fix_python_version().is_none() {
        panic!("unable to fix python version");
    }

    build_parent_dir.push("src");
    if !run_make("all", build_parent_dir.as_path()) {
        panic!("unable to make libinjection");
    }

    println!("cargo:rustc-link-lib=static=injection");
    println!("cargo:rustc-link-search={}", build_parent_dir.display());

    let h_path = build_parent_dir.join("libinjection.h");
    let bindings = bindgen::Builder::default()
        .header(h_path.to_str().unwrap())
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("unable to write bindings");
}
