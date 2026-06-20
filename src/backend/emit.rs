use std::{fs, path::Path};

use inkwell::module::Module;

use super::BackendResult;

pub fn emit_ir_to_string<'ctx>(module: &Module<'ctx>) -> String {
    module.print_to_string().to_string()
}

pub fn emit_ir_to_file<'ctx>(module: &Module<'ctx>, path: impl AsRef<Path>) -> BackendResult<()> {
    let ir = emit_ir_to_string(module);
    fs::write(path, ir)?;
    Ok(())
}
