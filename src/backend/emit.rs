use std::{fs, path::Path};

use inkwell::module::Module;

use super::{BackendError, BackendResult};

pub fn verify_module<'ctx>(module: &Module<'ctx>) -> BackendResult<()> {
    module
        .verify()
        .map_err(|msg| BackendError::VerificationFailed(msg.to_string()))
}

pub fn emit_ir_to_string<'ctx>(module: &Module<'ctx>) -> String {
    module.print_to_string().to_string()
}

pub fn emit_ir_to_file<'ctx>(
    module: &Module<'ctx>,
    path: impl AsRef<Path>,
) -> BackendResult<()> {
    let ir = emit_ir_to_string(module);
    fs::write(path, ir)?;
    Ok(())
}