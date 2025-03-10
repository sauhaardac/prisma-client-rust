use std::{
    fs::{self, File},
    io::{stderr, stdin, BufRead, BufReader, Write},
    path::Path,
    sync::Arc,
};

use dmmf::from_precomputed_parts;
use query_core::schema;

use crate::{
    args::GenerateArgs, dmmf::EngineDMMF, jsonrpc, utils::rustfmt, GenerateFn, GeneratorError,
};

pub struct GeneratorMetadata {
    generate_fn: GenerateFn,
    name: &'static str,
    default_output: &'static str,
}

impl GeneratorMetadata {
    pub fn new(generate_fn: GenerateFn, name: &'static str, default_output: &'static str) -> Self {
        Self {
            generate_fn,
            name,
            default_output,
        }
    }

    pub fn run(self) {
        loop {
            let mut content = String::new();
            BufReader::new(stdin())
                .read_line(&mut content)
                .expect("Failed to read engine output");

            let input: jsonrpc::Request =
                serde_json::from_str(&content).expect("Failed to marshal jsonrpc input");

            let data = match input.method.as_str() {
                "getManifest" => jsonrpc::ResponseData::Result(
                    serde_json::to_value(jsonrpc::ManifestResponse {
                        manifest: jsonrpc::Manifest {
                            default_output: self.default_output.to_string(),
                            pretty_name: self.name.to_string(),
                            ..Default::default()
                        },
                    })
                    .expect("Failed to convert manifest to json"), // literally will never fail
                ),
                "generate" => {
                    let params_str = input.params.to_string();

                    let deserializer = &mut serde_json::Deserializer::from_str(&params_str);

                    let dmmf = serde_path_to_error::deserialize(deserializer)
                        .expect("Failed to deserialize DMMF from Prisma engines");

                    match self.generate(dmmf) {
                        Ok(_) => jsonrpc::ResponseData::Result(serde_json::Value::Null),
                        Err(e) => jsonrpc::ResponseData::Error {
                            code: 0,
                            message: e.to_string(),
                        },
                    }
                }
                method => jsonrpc::ResponseData::Error {
                    code: 0,
                    message: format!("{} cannot handle method {}", self.name, method),
                },
            };

            let response = jsonrpc::Response {
                jsonrpc: "2.0".to_string(),
                id: input.id,
                data,
            };

            let mut bytes =
                serde_json::to_vec(&response).expect("Failed to marshal json data for reply");

            bytes.push(b'\n');

            stderr()
                .by_ref()
                .write(bytes.as_ref())
                .expect("Failed to write output to stderr for Prisma engines");

            if input.method.as_str() == "generate" {
                break;
            }
        }
    }

    fn generate(&self, engine_dmmf: EngineDMMF) -> Result<(), GeneratorError> {
        let schema = Arc::new(
            psl::parse_schema(engine_dmmf.datamodel.as_str())
                .expect("Datamodel is invalid after being verified by CLI?!"),
        );
        let query_schema = Arc::new(schema::build(schema.clone(), true));
        let dmmf = from_precomputed_parts(&query_schema);

        let output_str = engine_dmmf.generator.output.get_value();
        let output_path = Path::new(&output_str);

        let config = engine_dmmf.generator.config.clone();

        let mut file = create_generated_file(output_path)?;

        let mut generated_str = format!("// Code generated by {}. DO NOT EDIT\n\n", self.name);

        generated_str +=
            &(self.generate_fn)(GenerateArgs::new(&schema, &dmmf, engine_dmmf), config)?;

        file.write(generated_str.as_bytes())
            .map_err(GeneratorError::FileWrite)?;

        rustfmt(output_path);

        Ok(())
    }
}

fn create_generated_file(path: &Path) -> Result<File, GeneratorError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(GeneratorError::FileCreate)?;
    }

    File::create(&path).map_err(GeneratorError::FileCreate)
}
