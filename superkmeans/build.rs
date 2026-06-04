use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=cpp/superkmeans_bridge.cpp");
    println!("cargo:rerun-if-changed=third_party/superkmeans/include");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let object = out_dir.join("superkmeans_bridge.o");
    let library = out_dir.join("libsuperkmeans_bridge.a");
    let target = env::var("TARGET").unwrap_or_default();

    let eigen_include = find_include(
        "EIGEN3_INCLUDE_DIR",
        &[
            "/opt/homebrew/opt/eigen/include/eigen3",
            "/usr/local/include/eigen3",
            "/usr/include/eigen3",
        ],
        "Eigen/Dense",
    );
    let libomp_include = find_include(
        "LIBOMP_INCLUDE_DIR",
        &[
            "/opt/homebrew/opt/libomp/include",
            "/usr/local/opt/libomp/include",
            "/usr/include",
        ],
        "omp.h",
    );
    let libomp_lib = find_library_dir(
        "LIBOMP_LIB_DIR",
        &[
            "/opt/homebrew/opt/libomp/lib",
            "/usr/local/opt/libomp/lib",
            "/usr/lib",
        ],
        "libomp",
    );
    let fftw = find_fftw();

    let mut compile = Command::new(env::var("CXX").unwrap_or_else(|_| "clang++".to_string()));
    compile
        .arg("-std=c++17")
        .arg("-O3")
        .arg("-DNDEBUG")
        // This object is archived into a static lib that is linked into the
        // pg_search `.so`, so it must be position-independent. Without `-fPIC`
        // on Linux the `thread_local`s compile to the local-exec TLS model
        // (`R_X86_64_TPOFF32`), which the linker rejects with `-shared`.
        // (clang on macOS emits PIC by default, so this was latent there.)
        .arg("-fPIC")
        .arg("-ftls-model=global-dynamic")
        .arg("-c")
        .arg("cpp/superkmeans_bridge.cpp")
        .arg("-Ithird_party/superkmeans/include")
        .arg(format!("-I{}", eigen_include.display()))
        .arg(format!("-I{}", libomp_include.display()))
        .arg("-o")
        .arg(&object);
    if let Some(fftw) = &fftw {
        compile.arg("-DHAS_FFTW");
        compile.arg(format!("-I{}", fftw.include.display()));
    }
    if target.contains("apple") {
        compile.arg("-Xpreprocessor").arg("-fopenmp");
    } else {
        compile.arg("-fopenmp");
    }
    if env::var("SKMEANS_PORTABLE").map_or(true, |value| value != "ON") {
        let march = env::var("SKMEANS_MARCH").unwrap_or_else(|_| "native".to_string());
        if !march.is_empty() {
            compile.arg(format!("-march={march}"));
        }
    }
    run(compile);

    let mut ar = Command::new(env::var("AR").unwrap_or_else(|_| "ar".to_string()));
    ar.arg("crs").arg(&library).arg(&object);
    run(ar);

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-search=native={}", libomp_lib.display());
    println!("cargo:rustc-link-lib=static=superkmeans_bridge");
    println!("cargo:rustc-link-lib=dylib=omp");
    if let Some(fftw) = fftw {
        println!("cargo:rustc-link-search=native={}", fftw.lib.display());
        println!("cargo:rustc-link-lib=dylib={}", fftw.thread_lib);
        println!("cargo:rustc-link-lib=dylib=fftw3f");
    }
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=c++");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
        // BLAS (sgemm_). apt: libopenblas-dev. Override via env if a distro
        // ships a different provider (e.g. SKMEANS_BLAS_LIB=blas).
        let blas_lib = env::var("SKMEANS_BLAS_LIB").unwrap_or_else(|_| "openblas".to_string());
        if let Ok(dir) = env::var("SKMEANS_BLAS_LIB_DIR") {
            println!("cargo:rustc-link-search=native={dir}");
        }
        println!("cargo:rustc-link-lib=dylib={blas_lib}");
    }
}

struct FftwConfig {
    include: PathBuf,
    lib: PathBuf,
    thread_lib: &'static str,
}

fn find_include(env_name: &str, candidates: &[&str], required: &str) -> PathBuf {
    if let Ok(path) = env::var(env_name) {
        let path = PathBuf::from(path);
        if path.join(required).exists() {
            return path;
        }
        panic!("{env_name} does not contain {required}");
    }
    for candidate in candidates {
        let path = Path::new(candidate);
        if path.join(required).exists() {
            return path.to_path_buf();
        }
    }
    panic!("could not find include directory containing {required}; set {env_name}");
}

fn find_library_dir(env_name: &str, candidates: &[&str], stem: &str) -> PathBuf {
    if let Ok(path) = env::var(env_name) {
        let path = PathBuf::from(path);
        if has_library(&path, stem) {
            return path;
        }
        panic!("{env_name} does not contain {stem}");
    }
    for candidate in candidates {
        let path = Path::new(candidate);
        if has_library(path, stem) {
            return path.to_path_buf();
        }
    }
    panic!("could not find library directory containing {stem}; set {env_name}");
}

fn find_fftw() -> Option<FftwConfig> {
    if env::var("SKMEANS_SKIP_FFTW").is_ok_and(|value| value == "ON") {
        return None;
    }
    let include = find_include_optional(
        "FFTW_INCLUDE_DIR",
        &[
            "/opt/homebrew/include",
            "/usr/local/include",
            "/usr/include",
        ],
        "fftw3.h",
    )?;
    let lib = find_library_dir_optional(
        "FFTW_LIB_DIR",
        &["/opt/homebrew/lib", "/usr/local/lib", "/usr/lib"],
        "libfftw3f",
    )?;
    let thread_lib = if has_library(&lib, "libfftw3f_omp") {
        "fftw3f_omp"
    } else if has_library(&lib, "libfftw3f_threads") {
        "fftw3f_threads"
    } else {
        return None;
    };
    Some(FftwConfig {
        include,
        lib,
        thread_lib,
    })
}

fn find_include_optional(env_name: &str, candidates: &[&str], required: &str) -> Option<PathBuf> {
    if let Ok(path) = env::var(env_name) {
        let path = PathBuf::from(path);
        return path.join(required).exists().then_some(path);
    }
    candidates
        .iter()
        .map(Path::new)
        .find(|path| path.join(required).exists())
        .map(Path::to_path_buf)
}

fn find_library_dir_optional(env_name: &str, candidates: &[&str], stem: &str) -> Option<PathBuf> {
    if let Ok(path) = env::var(env_name) {
        let path = PathBuf::from(path);
        return has_library(&path, stem).then_some(path);
    }
    candidates
        .iter()
        .map(Path::new)
        .find(|path| has_library(path, stem))
        .map(Path::to_path_buf)
}

fn has_library(path: &Path, stem: &str) -> bool {
    [".dylib", ".so", ".a"]
        .iter()
        .any(|suffix| path.join(format!("{stem}{suffix}")).exists())
}

fn run(mut command: Command) {
    let status = command.status().expect("failed to spawn command");
    if !status.success() {
        panic!("command failed: {command:?}");
    }
}
