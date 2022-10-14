fn main() -> eyre::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(cirrus::main())?;
    Ok(())
}
