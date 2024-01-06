fn main() {
    std::process::Command::new("sh")
        .arg("../benchmarks/benchmark-paradedb.sh")
        .arg("-t")
        .arg("pgrx")
        .status()
        .expect("Failed to execute benchmark script");
}
