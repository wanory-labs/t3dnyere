use serde_json::{Value, json};
use std::{
    env,
    fs::File,
    io::{Read, Write},
    path::Path,
};
use t3dnyere::{
    MAX_BYTES, Model, Result,
    codec::MAGIC,
    parse_document, protocol,
    render::{self, Camera},
};

fn read_input(path: &str) -> Result<Vec<u8>> {
    let input: Box<dyn Read> = if path == "-" {
        Box::new(std::io::stdin())
    } else {
        Box::new(File::open(path).map_err(|e| e.to_string())?)
    };
    let mut bytes = Vec::new();
    input
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("input exceeds 2 MiB".into());
    }
    Ok(bytes)
}
fn load(path: &str) -> Result<Model> {
    let bytes = read_input(path)?;
    if bytes.starts_with(MAGIC) {
        Model::from_bytes(&bytes)
    } else {
        parse_document(&bytes)?.compile()
    }
}
fn create(path: &str, bytes: &[u8]) -> Result<()> {
    // Never silently overwrite sources or previous assets.
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(Path::new(path))
        .map_err(|e| format!("cannot create {path}: {e}"))?;
    file.write_all(bytes).map_err(|e| e.to_string())
}
fn stats(model: &Model) -> Value {
    json!({"ok":true,"format":"t3d-v1","instructions":model.instruction_count(),"root_index":model.root_index(),"runtime_bytes":model.to_bytes().len()})
}
fn run(args: &[String]) -> Result<Value> {
    match args.first().map(String::as_str) {
        Some("schema") if args.len() == 1 => Ok(protocol::document_schema()),
        Some("validate" | "inspect") if args.len() == 2 => Ok(stats(&load(&args[1])?)),
        Some("compile") if args.len() == 3 => {
            let model = parse_document(&read_input(&args[1])?)?.compile()?;
            create(&args[2], &model.to_bytes())?;
            let mut out = stats(&model); out["output"] = json!(args[2]); Ok(out)
        }
        Some("sample") if args.len() == 5 => {
            let model = load(&args[1])?;
            let mut point = [0.0; 3];
            for i in 0..3 { point[i] = args[i+2].parse().map_err(|_| "coordinates must be numbers")?; }
            let field = model.sample(point)?;
            Ok(json!({"ok":true,"point":point,"field":field,"inside":field < 0.0}))
        }
        Some("preview") if args.len() == 3 || args.len() == 4 => {
            let size = if args.len() == 4 { args[3].parse::<u32>().map_err(|_| "size must be an integer")? } else { 128 };
            let model = load(&args[1])?;
            let image = render::preview(&model,size,size,Camera::default())?;
            create(&args[2], &image)?;
            Ok(json!({"ok":true,"output":args[2],"width":size,"height":size,"bytes":image.len()}))
        }
        _ => Err("usage: t3dnyere schema | validate INPUT | inspect INPUT | compile SOURCE OUTPUT.t3d | sample INPUT X Y Z | preview INPUT OUTPUT.ppm [SIZE] | mcp; INPUT can be JSON/T3D or - for stdin; outputs must not exist".into()),
    }
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args == ["mcp"] {
        if let Err(message) = protocol::serve(std::io::stdin().lock(), std::io::stdout().lock()) {
            eprintln!("{message}");
            std::process::exit(1);
        }
        return;
    }
    if args == ["--help"] || args.is_empty() {
        println!(
            "t3dnyere — headless procedural solid modeling\nCommands (JSON output):\n  schema\n  validate INPUT\n  inspect INPUT\n  compile SOURCE OUTPUT.t3d\n  sample INPUT X Y Z\n  preview INPUT OUTPUT.ppm [SIZE]\n  mcp\nINPUT: JSON/T3D path or - for stdin. Outputs never overwrite existing files."
        );
        return;
    }
    match run(&args) {
        Ok(value) => println!("{value}"),
        Err(message) => {
            println!("{}", json!({"ok":false,"error":message}));
            std::process::exit(1);
        }
    }
}
