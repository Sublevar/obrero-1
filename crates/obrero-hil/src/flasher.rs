use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Nombre del binario tal como lo deja `cargo build` para el target activo
/// en `firmware/.cargo/config.toml` (S3 por ahora — ver `firmware/dev.sh`).
const TARGET_DIR: &str = "target/xtensa-esp32s3-espidf/debug";
const BIN_NAME: &str = "obrero-1";

/// Compila el firmware y lo flashea por el puerto UART indicado.
///
/// Requiere correr en una shell donde ya se corrió `source ~/export-esp.sh`
/// (mismo requisito que `firmware/dev.sh`) para que `cargo`/`espflash`
/// resuelvan el toolchain xtensa.
pub fn build_and_flash(firmware_dir: &Path, uart_port: &str) -> Result<()> {
    let status = Command::new("cargo")
        .current_dir(firmware_dir)
        // Si este proceso ya corre bajo `cargo` del toolchain stable del
        // workspace raíz, rustup nos pasa RUSTUP_TOOLCHAIN=stable en el
        // entorno — y eso pisa el override de `firmware/rust-toolchain.toml`
        // (esp/xtensa). Sacarlo para que rustup vuelva a resolver por archivo.
        .env_remove("RUSTUP_TOOLCHAIN")
        .arg("build")
        .status()
        .context("no se pudo ejecutar `cargo build` — ¿se corrió `source ~/export-esp.sh`?")?;
    if !status.success() {
        bail!("`cargo build` del firmware falló");
    }

    let bin_path = firmware_dir.join(TARGET_DIR).join(BIN_NAME);
    if !bin_path.exists() {
        bail!(
            "no se encontró el binario compilado en {}",
            bin_path.display()
        );
    }

    let status = Command::new("espflash")
        .current_dir(firmware_dir)
        .args(["flash", "--port", uart_port])
        .arg(&bin_path)
        .status()
        .context("no se pudo ejecutar `espflash` — ¿está instalado? (`cargo install espflash`)")?;
    if !status.success() {
        bail!("`espflash flash` falló");
    }

    Ok(())
}
