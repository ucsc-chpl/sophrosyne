use core::panic;
use std::process::Command;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::str;


fn main() {
    // Check if 'clspv' is availible in PATH.
    if Command::new("clspv").arg("--version").status().is_err() {
        panic!("clspv not found in PATH. Please install clspv.");
    }

    let cl_src_dir = "./shaders/cl";
    let spv_output_dir = "./shaders/spv";

    // Create spv ouput dir if it doesn't exist.
    fs::create_dir_all(spv_output_dir).expect("Failed to create spv output dir.");

    writeln!(io::stderr(), "Error message").unwrap();

    // Iterate over all OpenCL files in the directory.
    for entry in fs::read_dir(cl_src_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().unwrap() == "cl" {
            let spv_file_name = path.file_stem().unwrap().to_str().unwrap().to_owned() + ".spv";
            let spv_path = Path::new(spv_output_dir).join(spv_file_name);

            // Check if the .spv file neesd to be (re)compiled.
            if should_compile(&path, &spv_path) {
                // Compile the OpenCL file to SPIR-V.
                let output = Command::new("clspv")
                    .arg("-cl-std=CL2.0")
                    .arg("-inline-entry-points")
                    .arg(&path)
                    .arg("-o")
                    .arg(&spv_path)
                    .output()
                    .expect("Failed to execute clspv");

                if !output.status.success() {
                     // Write error message to stderr and print stderr of the command
                    writeln!(io::stderr(), "clspv failed to execute").unwrap();
                    writeln!(io::stderr(), "clspv Error Output: {}", str::from_utf8(&output.stderr).unwrap()).unwrap();
                    std::process::exit(1);
                }                
                
                // Tells cargo to rerun this build script if the shader file changes.
                println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
            }
        }
    }
}

fn should_compile(src_path: &Path, spv_path: &Path) -> bool {
    match (src_path.metadata(), spv_path.metadata()) {
        (Ok(src_meta), Ok(spv_meta)) => {
            // Recompile if the source is newer than the compiled file
            src_meta.modified().unwrap() > spv_meta.modified().unwrap()
        }
        _ => true, // Compile if either file is missing or on error
    }
}