//! Binary entry point — a thin wrapper over [`snip_lib::cli`].

fn main() -> anyhow::Result<()> {
    snip_lib::cli::run()
}
