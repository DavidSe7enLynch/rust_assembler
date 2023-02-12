use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

pub fn assemble(asm_file_path: &str, binary_file_path: &str) {
    let command_table = *parse_file(asm_file_path);
    let mut symbol_table = *gen_symbol_table(&command_table);
    parse_command_table(&command_table, &mut symbol_table, binary_file_path);
}

struct SymbolTable {
    map: HashMap<String, i16>,
    next_ram_idx: i16,
}

enum CommandType {
    A,
    C,
    L,
}

struct Command {
    #[allow(dead_code)]
    line_orig: String,
    command_type: CommandType,
    rom_idx: i16,
    symbol: Option<String>, // A, L
    comp: Option<String>,   // C
    jump: Option<String>,   // C
    dest: Option<String>,   // C
}

fn gen_symbol_table(command_table: &Vec<Command>) -> Box<SymbolTable> {
    let mut symbol_table = SymbolTable { map: HashMap::new(), next_ram_idx: 16 };
    add_default_symbols(&mut symbol_table);
    for command in command_table {
        match command.command_type {
            CommandType::L => {
                symbol_table.map.insert(command.symbol.as_ref().unwrap().clone(), command.rom_idx);
            }
            _ => {}
        };
    }
    Box::new(symbol_table)
}

fn add_default_symbols(symbol_table: &mut SymbolTable) {
    symbol_table.map.insert("SP".to_string(), 0);
    symbol_table.map.insert("LCL".to_string(), 1);
    symbol_table.map.insert("ARG".to_string(), 2);
    symbol_table.map.insert("THIS".to_string(), 3);
    symbol_table.map.insert("THAT".to_string(), 4);
    symbol_table.map.insert("SCREEN".to_string(), 16384);
    symbol_table.map.insert("KBD".to_string(), 24576);
    for i in 0..16 {
        let symbol = format!("R{i}");
        symbol_table.map.insert(symbol, i);
    }
}

fn parse_file(file_path: &str) -> Box<Vec<Command>> {
    let file = File::open(file_path).expect("open file fail");
    let reader = BufReader::new(file);
    let mut rom_idx = 0;
    let mut command_table = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.expect(&format!("line {} parse fail", idx));
        let line = line.split("//").next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        command_table.push(*parse_line(line, &mut rom_idx));
    }
    Box::new(command_table)
}

fn parse_line(line: &str, rom_idx: &mut i16) -> Box<Command> {
    if line.starts_with("@") {
        let symbol = Some(line[1..].to_string());
        let command = Box::new(Command {
            line_orig: String::from(line),
            command_type: CommandType::A,
            rom_idx: *rom_idx,
            symbol,
            comp: None,
            jump: None,
            dest: None,
        });
        *rom_idx += 1;
        command
    } else if line.starts_with("(") && line.ends_with(")") {
        let symbol = Some(line[1..line.len() - 1].to_string());
        Box::new(Command {
            line_orig: line.to_string(),
            command_type: CommandType::L,
            rom_idx: *rom_idx,
            symbol,
            comp: None,
            jump: None,
            dest: None,
        })
    } else {
        parse_c_line(line, rom_idx)
    }
}

fn parse_c_line(line: &str, rom_idx: &mut i16) -> Box<Command> {
    let split_jump: Vec<&str> = line.split(';').collect();
    let jump = if split_jump.len() > 1 {
        Some(split_jump[1].trim().to_string())
    } else {
        None
    };
    let split_dest: Vec<&str> = split_jump[0].split('=').collect();
    let (dest, comp) = if split_dest.len() > 1 {
        (
            Some(split_dest[0].trim().to_string()),
            Some(split_dest[1].trim().to_string()),
        )
    } else {
        (None, Some(split_dest[0].trim().to_string()))
    };
    let command = Box::new(Command {
        line_orig: line.to_string(),
        command_type: (CommandType::C),
        rom_idx: *rom_idx,
        symbol: None,
        comp,
        jump,
        dest,
    });
    *rom_idx += 1;
    command
}

fn parse_command_table(
    command_table: &Vec<Command>,
    symbol_table: &mut SymbolTable,
    binary_file_path: &str,
) {
    let mut binary_file = File::create(binary_file_path).expect("create binary file fail");
    for command in command_table {
        match command.command_type {
            CommandType::A => parse_a_command(command, &mut binary_file, symbol_table),
            CommandType::C => parse_c_command(command, &mut binary_file),
            _ => {},
        };
    }
}

fn parse_a_command(command: &Command, binary_file: &mut File, symbol_table: &mut SymbolTable) {
    let symbol = command
        .symbol
        .as_ref()
        .expect("A-type command should have symbol");
    let if_number = symbol.parse::<i16>();
    let address;
    match if_number {
        Ok(number) => {
            address = number;
        },
        Err(_) => {
            address = handle_a_symbol(symbol, symbol_table);
        }
    }
    let binary_str = format!("{:0>16b}\n", address);
    binary_file.write_all(binary_str.as_bytes()).expect("write file fail");
}

fn handle_a_symbol(symbol: &String, symbol_table: &mut SymbolTable) -> i16 {
    if symbol_table.map.contains_key(symbol) {
        return *symbol_table.map.get(symbol).unwrap();
    }
    symbol_table.map.insert(symbol.clone(), symbol_table.next_ram_idx);
    symbol_table.next_ram_idx += 1;
    return symbol_table.next_ram_idx - 1;
}

fn parse_c_command(command: &Command, binary_file: &mut File) {
    let comp = parse_comp(command);
    let dest = parse_dest(command);
    let jump = parse_jump(command);
    let binary_str = format!("111{comp}{dest}{jump}\n");
    binary_file
        .write_all(binary_str.as_bytes())
        .expect("write file fail");
}

fn parse_jump(command: &Command) -> &str {
    let jump = command.jump.as_ref();
    if jump.is_none() {
        return "000";
    }
    let jump = jump.unwrap().as_str();
    match jump {
        "JGT" => "001",
        "JEQ" => "010",
        "JGE" => "011",
        "JLT" => "100",
        "JNE" => "101",
        "JLE" => "110",
        "JMP" => "111",
        _ => unreachable!(),
    }
}

fn parse_dest(command: &Command) -> &str {
    let dest = command.dest.as_ref();
    if dest.is_none() {
        return "000";
    }
    let dest = dest.unwrap().as_str();
    match dest {
        "M" => "001",
        "D" => "010",
        "MD" => "011",
        "A" => "100",
        "AM" => "101",
        "AD" => "110",
        "AMD" => "111",
        _ => unreachable!(),
    }
}

fn parse_comp(command: &Command) -> &str {
    let comp_str = command
        .comp
        .as_ref()
        .expect("C-type command should have comp")
        .as_str();
    match comp_str {
        "0" => "0101010",
        "1" => "0111111",
        "-1" => "0111010",
        "D" => "0001100",
        "A" => "0110000",
        "!D" => "0001101",
        "!A" => "0110001",
        "-D" => "0001111",
        "-A" => "0110011",
        "D+1" => "0011111",
        "A+1" => "0110111",
        "D-1" => "0001110",
        "A-1" => "0110010",
        "D+A" => "0000010",
        "D-A" => "0010011",
        "A-D" => "0000111",
        "D&A" => "0000000",
        "D|A" => "0010101",
        "M" => "1110000",
        "!M" => "1110001",
        "-M" => "1110011",
        "M+1" => "1110111",
        "M-1" => "1110010",
        "D+M" => "1000010",
        "D-M" => "1010011",
        "M-D" => "1000111",
        "D&M" => "1000000",
        "D|M" => "1010101",
        _ => unreachable!(),
    }
}
