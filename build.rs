use serde_reflection::{ContainerFormat, Tracer};
use std::{fs, io::Write, path::Path};
use winit::keyboard::KeyCode;

fn main() {
    let mut tracer = Tracer::new(Default::default());

    // trace winit's KeyCode enum
    tracer.trace_simple_type::<KeyCode>().unwrap();

    // codegen for pyclass KeyCode enum
    let registry = tracer.registry().unwrap();
    match registry.get("KeyCode").unwrap() {
        ContainerFormat::Enum(contents) => {
            let mut code = String::new();
            code.push_str("#[pyclass(eq, eq_int)]\n");
            code.push_str("#[derive(Clone, Copy, PartialEq, Eq)]\n");
            code.push_str("pub enum Keys {\n");
            for (_, variant) in contents {
                code.push_str(&format!("\t{},\n", variant.name));
            }
            code.push_str("}\n");

            code.push_str("\nimpl From<winit::keyboard::KeyCode> for Keys {\n");
            code.push_str("\tfn from(key: winit::keyboard::KeyCode) -> Self {\n");
            code.push_str("\t\tmatch key {\n");
            for (_, variant) in contents {
                code.push_str(&format!(
                    "\t\t\twinit::keyboard::KeyCode::{} => Keys::{},\n",
                    variant.name, variant.name
                ));
            }
            code.push_str("\t\t\t_ => panic!(\"unrecognized key code\"),\n");
            code.push_str("\t\t}\n");
            code.push_str("\t}\n");
            code.push_str("}\n");

            let mut out_file = fs::File::create(Path::new("src/codegen/keycode.rs")).unwrap();
            out_file.write_all(code.as_bytes()).unwrap();
        }
        _ => panic!("expected enum"),
    }
}
