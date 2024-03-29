use glob::glob;
use naga::{
    valid::{Capabilities, ValidationFlags, Validator},
    ShaderStage,
};
use std::{collections::HashMap, env, fs, path::PathBuf};

fn main() {
    let src_dir = &PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/");
    let out_dir = &PathBuf::from(env::var("OUT_DIR").unwrap());

    // Find already compiled
    env::set_current_dir(out_dir).unwrap();
    let mut compiled = CompiledShaders::load();

    // Collect all shaders recursively within /src/ without prefix
    env::set_current_dir(src_dir).unwrap();
    let shaders = vec![
        glob("**/*.vert").unwrap(),
        glob("**/*.frag").unwrap(),
        glob("**/*.comp").unwrap(),
    ]
    .into_iter()
    .flatten()
    .filter_map(|glob_result| {
        let shader = ShaderData::load(glob_result.unwrap());
        match compiled.has_new_checksum(&shader) {
            true => Some(shader),
            false => None,
        }
    })
    .collect::<Vec<ShaderData>>();

    // This can't be parallelized. The [shaderc::Compiler] is not thread safe.
    env::set_current_dir(out_dir).unwrap();
    let mut parser = naga::front::glsl::Parser::default();
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::empty());

    for shader in shaders {
        let name = shader.path.to_str().unwrap();
        println!("cargo:warning=Compiling shader {name}");

        let module = parser
            .parse(
                &naga::front::glsl::Options {
                    stage: shader.kind,
                    defines: naga::FastHashMap::default(),
                },
                &shader.source,
            )
            .unwrap();
        let compiled = naga::back::wgsl::write_string(
            &module,
            &validator.validate(&module).unwrap(),
            naga::back::wgsl::WriterFlags::empty(),
        )
        .unwrap();
        let extension = match shader.kind {
            ShaderStage::Vertex => "vert",
            ShaderStage::Fragment => "frag",
            ShaderStage::Compute => "comp",
        };
        fs::write(
            shader.path.with_extension(format!("{extension}.wgsl")),
            compiled.as_bytes(),
        )
        .unwrap();
    }

    // Remember compiled
    compiled.store();
}

struct ShaderData {
    source: String,
    path: PathBuf,
    kind: ShaderStage,
}

impl ShaderData {
    pub fn load(path: PathBuf) -> Self {
        assert!(path.is_relative());
        assert!(path.is_file());

        let extension = path
            .extension()
            .expect("File has no extension")
            .to_str()
            .expect("Extension cannot be converted to &str");
        let kind = match extension {
            "vert" => ShaderStage::Vertex,
            "frag" => ShaderStage::Fragment,
            "comp" => ShaderStage::Compute,
            _ => panic!("Unsupported shader: {}", path.display()),
        };

        let source = fs::read_to_string(path.clone()).unwrap();
        Self { source, path, kind }
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
                .lines()
                .filter_map(|line| {
                    let mut words = line.split(' ');
                    let filename = PathBuf::from(words.next()?);
                    let hash = String::from(words.next()?);
                    Some((filename, hash))
                })
                .collect(),
        )
    }
    pub fn store(self) {
        let entries: Vec<String> = self
            .0
            .into_iter()
            .map(|(path, digest)| format!("{} {digest}", path.to_str().unwrap()))
            .collect();
        fs::write("shader_checksums.txt", entries.join("\n")).unwrap();
    }
    pub fn has_new_checksum(&mut self, shader: &ShaderData) -> bool {
        let digest = format!("{:?}", blake3::hash(shader.source.as_bytes()));
        if let Some(old_digest) = self.0.get(&shader.path) {
            if *old_digest == digest {
                return false;
            }
        }
        self.0.insert(shader.path.clone(), digest);
        true
    }
}
