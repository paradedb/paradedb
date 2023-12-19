fn main() {
    std::process::Command::new("sh")
        .arg("clickbench/paradedb/benchmark.sh")
        .status()
        .expect("Failed to execute benchmark script");
}
