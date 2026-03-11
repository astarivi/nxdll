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
                .then(|| name.to_string())
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

        writeln!(exports, "use nxdll_shared::loader::registry::PEExportedFunction;")?;
        writeln!(exports, "use alloc::{{boxed::Box, vec::Vec}};")?;
        writeln!(exports, "use crate::bindings::{lib_name};")?;
        writeln!(exports, "\npub fn add_modules(to: &mut Vec<(Box<str>, Vec<PEExportedFunction>)>) {{")?;
        writeln!(exports, "    let mut target: Vec<PEExportedFunction> = Vec::new();")?;

        writeln!(def, "LIBRARY {lib_name}")?;
        writeln!(def, "EXPORTS")?;

        for (ordinal, symbol) in symbols.iter().enumerate().map(|(i, s)| (i + 1, s)) {

            if symbol.starts_with(".weak") {
                println!("Skipping weak symbol: {}", symbol);
                continue;
            }

            let has_at_suffix = symbol.contains('@') &&
                symbol.chars().rev()
                    .take_while(|c| c.is_ascii_digit())
                    .count() > 0;

            let (convention, sym, arg_size) = if symbol.starts_with('_') && has_at_suffix {
                // stdcall: _Name@N
                let stripped = symbol.strip_prefix('_').unwrap();
                let at_pos = stripped.rfind('@').unwrap();
                let size: u32 = stripped[at_pos + 1..].parse().unwrap_or(0);
                let name = &stripped[..at_pos];
                ("stdcall", name, Some(size))
            } else if symbol.starts_with('@') && has_at_suffix {
                // fastcall: @Name@N
                let stripped = symbol.strip_prefix('@').unwrap();
                let at_pos = stripped.rfind('@').unwrap();
                let size: u32 = stripped[at_pos + 1..].parse().unwrap_or(0);
                let name = &stripped[..at_pos];
                ("fastcall", name, Some(size))

            } else if symbol.starts_with('?') {
                // thiscall: MSVC mangled ?method@Class@@...
                // Extract arg size from the @N suffix if present, else None
                let arg_size = if has_at_suffix {
                    let at_pos = symbol.rfind('@').unwrap();
                    symbol[at_pos + 1..].parse().ok()
                } else {
                    None
                };
                ("thiscall", symbol.as_str(), arg_size)

            } else if symbol.starts_with('_') {
                // cdecl: _Name
                let name = symbol.strip_prefix('_').unwrap();
                ("C", name, None)

            } else {
                // No decoration at all — treat as C/cdecl
                ("C", symbol.as_str(), None)
            };

            let dummy_args = match arg_size {
                Some(size) => {
                    let count = size / 4;
                    (0..count)
                        .map(|i| format!("_arg{}: u32", i))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
                None => String::new(),
            };
            writeln!(bindings, "extern \"{convention}\" {{")?;
            writeln!(bindings, "    #[link_name = \"{sym}\"]\n    pub fn sym{ordinal}({dummy_args});")?;
            writeln!(bindings, "}}")?;
            writeln!(exports,
                     "    target.push(PEExportedFunction {{ name: None, ordinal: {ordinal}, addr: {lib_name}::sym{ordinal} as *const () as *const u8 }});"
            )?;
            writeln!(def, "    {symbol} @{ordinal}")?;
        }

        writeln!(exports, "    to.push((Box::from(\"{lib_name}.dll\"), target));\n}}")?;

        // Generate import lib
        run_command("llvm-dlltool", &[
            "-m", "i386",
            "-d", lib_dir.join(format!("{lib_name}.def")).to_str().unwrap(),
            "-l", lib_dir.join(format!("{lib_name}.lib")).to_str().unwrap(),
            "--no-leading-underscore"
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