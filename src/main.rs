fn main() -> eyre::Result<()> {
    #[cfg(feature = "restigo")]
    if std::env::var_os("__CIRRUS_INTERNAL_MODE_BUNDLED_RESTIC").is_some() {
        restigo::restic_main();
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(cirrus::main())?;
    Ok(())
}
