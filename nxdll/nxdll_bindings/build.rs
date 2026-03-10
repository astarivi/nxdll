use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const EXCLUDED_LIBS: &[&str] = &["libnxdk_automount_d"];

fn run_command(program: &str, args: &[&str]) -> String {
    let output = Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|_| panic!("{program} failed to start; is it in your PATH?"));

    if !output.status.success() {
        panic!("{program} exited with an error");
    }

    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn extract_symbols(library: &Path) -> Vec<String> {
    let stdout = run_command("llvm-nm", &["-g", "--defined-only", library.to_str().unwrap()]);

    stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (_, sym_type, name) = (parts.next()?, parts.next()?, parts.next()?);
            ["T", "D", "B"].contains(&sym_type)
                .then(|| name.strip_prefix('_').unwrap_or(name).to_string())
        })
        .collect()
}

fn generate_bindings(nxdk_dir: &str) -> std::io::Result<()> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let manifest = Path::new(&manifest_dir);

    let mut libraries: Vec<PathBuf> = std::fs::read_dir(Path::new(nxdk_dir).join("lib"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map_or(false, |ext| ext == "lib"))
        .collect();

    libraries.push(Path::new(nxdk_dir).join("lib/xboxkrnl/libxboxkrnl.lib"));

    let mut mod_rs       = File::create(manifest.join("src/bindings/mod.rs"))?;
    let mut exports_mod  = File::create(manifest.join("src/exports/mod.rs"))?;
    let mut modules      = File::create(manifest.join("src/modules.rs"))?;

    writeln!(modules, "use nxdll_shared::loader::registry::PEExportedFunction;")?;
    writeln!(modules, "use alloc::{{boxed::Box, vec::Vec}};")?;
    writeln!(modules, "\npub fn get_emulated_modules() -> Vec<(Box<str>, Vec<PEExportedFunction>)> {{")?;
    writeln!(modules, "    let mut result: Vec<(Box<str>, Vec<PEExportedFunction>)> = Vec::new();")?;

    // Truncate Cargo.toml at the codegen marker
    let mut cargo_file = OpenOptions::new().read(true).write(true).open(manifest.join("Cargo.toml"))?;
    let mut cargo_contents = String::new();
    cargo_file.read_to_string(&mut cargo_contents)?;
    let marker = "# codegen Internal";
    let cut_pos = cargo_contents.find(marker).unwrap() + marker.len();
    cargo_file.set_len(cut_pos as u64)?;
    cargo_file.seek(SeekFrom::Start(cut_pos as u64))?;
    writeln!(cargo_file)?;

    for library in &libraries {
        let lib_name = library.file_stem().unwrap().to_str().unwrap().replace('+', "x");

        if EXCLUDED_LIBS.contains(&&*lib_name) {
            continue;
        }

        let symbols = extract_symbols(library);
        let lib_dir = manifest.join("lib");

        let mut bindings = File::create(manifest.join(format!("src/bindings/{lib_name}.rs")))?;
        let mut exports  = File::create(manifest.join(format!("src/exports/{lib_name}.rs")))?;
        let mut def      = File::create(lib_dir.join(format!("{lib_name}.def")))?;

        writeln!(bindings, "extern \"C\" {{")?;

        writeln!(exports, "use nxdll_shared::loader::registry::PEExportedFunction;")?;
        writeln!(exports, "use alloc::{{boxed::Box, vec::Vec}};")?;
        writeln!(exports, "use crate::bindings::{lib_name};")?;
        writeln!(exports, "\npub fn add_modules(to: &mut Vec<(Box<str>, Vec<PEExportedFunction>)>) {{")?;
        writeln!(exports, "    let mut target: Vec<PEExportedFunction> = Vec::new();")?;

        writeln!(def, "LIBRARY {lib_name}")?;
        writeln!(def, "EXPORTS")?;

        for (ordinal, symbol) in symbols.iter().enumerate().map(|(i, s)| (i + 1, s)) {
            writeln!(bindings, "    #[link_name = \"{symbol}\"]\n    pub fn sym{ordinal}();")?;
            writeln!(exports,
                     "    target.push(PEExportedFunction {{ name: None, ordinal: {ordinal}, addr: {lib_name}::sym{ordinal} as *const () as *const u8 }});"
            )?;
            writeln!(def, "    {symbol} @{ordinal}")?;
        }

        writeln!(bindings, "}}")?;
        writeln!(exports, "    to.push((Box::from(\"{lib_name}\"), target));\n}}")?;

        // Generate import lib
        run_command("llvm-dlltool", &[
            "-m", "i386",
            "-d", lib_dir.join(format!("{lib_name}.def")).to_str().unwrap(),
            "-l", lib_dir.join(format!("{lib_name}.lib")).to_str().unwrap(),
        ]);

        // Register feature in all three files
        for file in [&mut mod_rs, &mut exports_mod] {
            writeln!(file, "#[cfg(feature = \"{lib_name}\")]\npub mod {lib_name};")?;
        }
        writeln!(cargo_file, "{lib_name} = []")?;
        writeln!(modules, "    #[cfg(feature = \"{lib_name}\")]\n    crate::exports::{lib_name}::add_modules(&mut result);")?;
    }

    writeln!(modules, "    result\n}}")?;
    Ok(())
}

fn main() {
    let nxdk_dir = std::env::var("NXDK_DIR")
        .expect("NXDK_DIR environment variable is not set");
    generate_bindings(&nxdk_dir).unwrap();
}