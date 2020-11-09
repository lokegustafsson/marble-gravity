use anyhow::*;
use glob::glob;
use shaderc::{CompileOptions, Compiler, OptimizationLevel, ShaderKind};
use std::{
    collections::HashMap,
    env::{self, set_current_dir},
    fs,
    path::PathBuf,
};

fn main() -> Result<()> {
    let root_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src_dir = &root_dir.clone().join("src/");
    let spirv_dir = &root_dir.join("target/");

    // Find already compiled
    set_current_dir(spirv_dir)?;
    let mut compiled = CompiledShaders::load();

    // Collect all shaders recursively within /src/ without prefix
    set_current_dir(src_dir)?;
    let shaders = vec![glob("**/*.vert")?, glob("**/*.frag")?, glob("**/*.comp")?]
        .into_iter()
        .flatten()
        .map(|glob_result| ShaderData::load(glob_result?))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|shader| compiled.has_new_checksum(shader))
        .collect::<Vec<ShaderData>>();

    // This can't be parallelized. The [shaderc::Compiler] is not thread safe.
    set_current_dir(spirv_dir)?;
    let mut compiler = Compiler::new().context("Unable to create shader compiler")?;
    let mut compile_options =
        CompileOptions::new().context("Unable to create shader compile options")?;
    compile_options.set_optimization_level(OptimizationLevel::Performance);

    for shader in shaders {
        let name = shader.path.to_str().unwrap();
        println!("cargo:warning=Compiling shader {}", name);

        let compiled = compiler.compile_into_spirv(
            &shader.source,
            shader.kind,
            &name,
            "main",
            Some(&compile_options),
        )?;
        let extension = match shader.kind {
            ShaderKind::Vertex => "vert",
            ShaderKind::Fragment => "frag",
            ShaderKind::Compute => "comp",
            _ => panic!("Shader {:?} unsupported"),
        };
        fs::write(
            shader.path.with_extension(format!("{}.spv", extension)),
            compiled.as_binary_u8(),
        )?;
    }

    // Remember compiled
    compiled.store();
    Ok(())
}

struct ShaderData {
    source: String,
    path: PathBuf,
    kind: ShaderKind,
}

impl ShaderData {
    pub fn load(path: PathBuf) -> Result<Self> {
        assert!(path.is_relative());
        assert!(path.is_file());

        let extension = path
            .extension()
            .context("File has no extension")?
            .to_str()
            .context("Extension cannot be converted to &str")?;
        let kind = match extension {
            "vert" => ShaderKind::Vertex,
            "frag" => ShaderKind::Fragment,
            "comp" => ShaderKind::Compute,
            _ => bail!("Unsupported shader: {}", path.display()),
        };

        let source = fs::read_to_string(path.clone())?;
        Ok(Self { source, path, kind })
    }
}

/**
Caches shader source file checksums, to avoid unnecessary recompilation.

Example `target/shader_checksums.txt` content:
```norun
shader.frag bf009481bd9bb7650dcdf903fafc896c
shader.vert 04181d9dc9d21e07dded377e96e6e61b
```
*/
struct CompiledShaders(HashMap<PathBuf, String>);

impl CompiledShaders {
    fn load() -> Self {
        let entries = match fs::read_to_string("shader_checksums.txt") {
            Ok(entries) => entries,
            Err(_) => return Self(Default::default()),
        };
        Self(
            entries
                .split('\n')
                .map(|line| {
                    let mut words = line.split(' ');
                    Some((PathBuf::from(words.nth(0)?), String::from(words.nth(0)?)))
                })
                .filter(Option::is_some)
                .map(Option::unwrap)
                .collect(),
        )
    }
    pub fn store(self) {
        let entries: Vec<String> = self
            .0
            .into_iter()
            .map(|(path, digest)| format!("{} {}", path.to_str().unwrap(), digest))
            .collect();
        fs::write("shader_checksums.txt", &entries.join("\n")).unwrap();
    }
    pub fn has_new_checksum(&mut self, shader: &ShaderData) -> bool {
        let digest = format!("{:?}", md5::compute(&shader.source));
        if let Some(old_digest) = self.0.get(&shader.path) {
            if *old_digest == digest {
                return false;
            }
        }
        self.0.insert(shader.path.clone(), digest);
        return true;
    }
}
