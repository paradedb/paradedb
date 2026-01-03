use std::error::Error;
use vergen::Emitter;
use vergen_git2::Git2Builder;

fn main() -> Result<(), Box<dyn Error>> {
    let short_sha = false;
    let git_instructions = Git2Builder::default()
        .sha(short_sha) // Emit VERGEN_GIT_SHA
        .commit_timestamp(true) // Maybe also the commit timestamp, etc.
        .build()?;

    Emitter::default()
        .add_instructions(&git_instructions)?
        .emit()?;

    Ok(())
}
