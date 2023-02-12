mod assembler;

fn main() {
    let directory = "../projects/06/";
    let file_name = "pong/Pong";
    let asm_file_path = format!("{directory}{file_name}.asm");
    let binary_file_path = format!("{directory}{file_name}.hack");
    assembler::assemble(asm_file_path.as_str(), binary_file_path.as_str());
    println!("finished");
}