use std::io::Read;

/// Basic VM
const MEM_SIZE: usize = 0xFFFF; // size is u16::MAX
const PC_BASE: u16 = 0x3000; // base address where to load program

/// Instruction opcodes
const BR:  u16 = 0x0;
const ADD: u16 = 0x1;
const LD:  u16 = 0x2;
const ST:  u16 = 0x3;
const JSR: u16 = 0x4;
const AND: u16 = 0x5;
const LDR: u16 = 0x6;
const STR: u16 = 0x7;
// RTI is unimplemented - 0x8;
const NOT: u16 = 0x9;
const LDI: u16 = 0xA;
const STI: u16 = 0xB;
const JMP: u16 = 0xC;
// UNUSED OP 0xD;
const LEA: u16 = 0xE;
const TRAP: u16 = 0xF;

// TRAP codes
const TGETC: u16 = 0x20;
const TOUT: u16 = 0x21;
const TPUTS: u16 = 0x22;
const TIN: u16 = 0x23;
const TPUTSP: u16 = 0x24;
const THALT: u16 = 0x25;
const TINU16: u16 = 0x26;
const TOUTU16: u16 = 0x27;


/// Flags
const FP: u16 = 0x1 << 0;
const FZ: u16 = 0x1 << 1;
const FN: u16 = 0x1 << 2;

/// helper instructions
#[inline]
fn dr(op: u16) -> u16 {
    (op >> 9) & 0x7
}

#[inline]
fn sr1(op: u16) -> u16 {
    (op >> 6) & 0x7
}

#[inline]
fn sr2(op: u16) -> u16 {
    op & 0x7
}

#[inline]
fn fimm(op: u16) -> u16 {
    (op >> 5) & 0x1
}

#[inline]
fn jimm(op: u16) -> u16 {
    (op >> 11) & 0x1
}

#[inline]
fn imm(op: u16, b: u16) -> u16 {
    op & (2_u16.pow(b as u32) - 1)
}

#[inline]
fn sximm(op: u16, b: u16) -> u16 {
    let imm = imm(op, b);
    if ((imm >> (b-1)) & 1) == 1 {
        (0xFFFF << b) | imm
    } else {
        imm
    }
}

struct VM {
    // VM memory
    memory: [u16; MEM_SIZE], 
    cpu: Cpu,
    stop_flag: bool,
}

impl VM {
    fn new() -> Self {
        VM { 
            memory: [0x0; MEM_SIZE], 
            cpu: Cpu::new(PC_BASE),
            stop_flag: false,
        }
    }

    // Update control register
    fn set_flag(&mut self, reg: u16) {
        let v = self.cpu.r[reg as usize];

        if v == 0 {
            self.cpu.rcnd = FZ;
        } else if v > 0x7fff {
            self.cpu.rcnd = FN;
        } else {
            self.cpu.rcnd = FP;
        }
    }

    // Execute AND
    fn and(&mut self, op: u16) {
        print!("-> ");
        if fimm(op) == 1 {
            println!("AND: R{} = R{} & {}", dr(op), sr1(op), sximm(op, 5));
            self.cpu.r[dr(op) as usize] = 
                self.cpu.r[sr1(op) as usize] & sximm(op, 5);
        } else {
            println!("AND: R{} = R{} & R{}", dr(op), sr1(op), sr2(op));          
            self.cpu.r[dr(op) as usize] = 
                self.cpu.r[sr1(op) as usize] & self.cpu.r[sr2(op) as usize];
        }

        // update condition
        self.set_flag(dr(op))
    }

    // Execute ADD
    fn add(&mut self, op: u16) {
        print!("-> ");
        if fimm(op) == 1 {
            println!("ADD: R{} = R{} + {}", dr(op), sr1(op), sximm(op, 5));
            self.cpu.r[dr(op) as usize] = 
                self.cpu.r[sr1(op) as usize] + sximm(op, 5);


        } else {
            println!("ADD: R{} = R{} + R{}", dr(op), sr1(op), sr2(op));          
            self.cpu.r[dr(op) as usize] = 
                self.cpu.r[sr1(op) as usize] + self.cpu.r[sr2(op) as usize];
        }

        // update condition
        self.set_flag(dr(op))
    }

    // Execute LD
    fn ld(&mut self, op: u16) {
        println!("-> LD: R{} = 0x{:x}",  dr(op), self.cpu.rpc + sximm(op, 9));
        self.cpu.r[dr(op) as usize] = 
            self.mem_read(self.cpu.rpc + sximm(op, 9));

        // update condition
        self.set_flag(dr(op))
    }

    // Execute LDI
    fn ldi(&mut self, op:u16) {
        println!("-> LDI: R{} = [0x{:x}]",  dr(op), self.cpu.rpc + sximm(op, 9));
        let imm = self.mem_read(self.cpu.rpc + sximm(op, 9));
        self.cpu.r[dr(op) as usize] = self.mem_read(imm);

        // update condition
        self.set_flag(dr(op))
    }

    // Execute LDR
    fn ldr(&mut self, op: u16) {
        println!("-> LDR: R{} = [R{:x}]",  dr(op), sr1(op));
        self.cpu.r[dr(op) as usize] = 
            self.mem_read(self.cpu.r[sr1(op) as usize] + sximm(op, 6));

        // update condition
        self.set_flag(dr(op))
    }

    // Execute LEA
    fn lea(&mut self, op: u16) {
        println!("-> LEA: R{} = RPC + {:x}",  dr(op), sximm(op, 9));
        self.cpu.r[dr(op) as usize] = self.cpu.rpc + sximm(op, 9);

        // update condition
        self.set_flag(dr(op))
    }

    // Execute NOT
    fn not(&mut self, op: u16) {
        println!("-> NOT: R{} = ~R{}",  dr(op), sr1(op));
        self.cpu.r[dr(op) as usize] = !self.cpu.r[sr1(op) as usize];

        // update condition
        self.set_flag(dr(op))
    }

    // Execute ST
    fn st(&mut self, op: u16) {
        println!("-> ST: [RPC + {}] = R{}", sximm(op, 9), dr(op));
        self.mem_write(
            self.cpu.rpc + sximm(op, 9), self.cpu.r[dr(op) as usize]);
    }

    // Execute STR
    fn sti(&mut self, op: u16) {
        println!("-> STI: [RPC + {}] = R{}",  sximm(op, 9), dr(op),);
        self.mem_write(
            self.mem_read(self.cpu.rpc + sximm(op, 9)), 
            self.cpu.r[dr(op) as usize]);
    }

    // Execute STR
    fn str(&mut self, op: u16) {
        println!("-> STR: [{} + {}] = R{}", sr1(op), sximm(op, 6), dr(op));
        self.mem_write(
            self.cpu.r[sr1(op) as usize] + sximm(op, 6), 
            self.cpu.r[dr(op) as usize]);
    }

    // Execute JMP
    fn jmp(&mut self, op: u16) {
        println!("-> JMP: R{}", sr1(op));
        self.cpu.rpc = self.cpu.r[sr1(op) as usize];
    }

    // Execute JSR
    fn jsr(&mut self, op: u16) {
        self.cpu.r[7] = self.cpu.rpc;

        print!("-> ");
        if jimm(op) == 1 {
            println!("JSR: RPC + 0x{:x}", sximm(op, 11));
            self.cpu.rpc += sximm(op, 11);
        } else {
            println!("JSR: R{} ", sr1(op));          
            self.cpu.rpc = self.cpu.r[sr1(op) as usize]
        }
    }

    // Execute BR
    fn br(&mut self, op: u16) {
        if self.cpu.rcnd & dr(op) != 0 {
            println!("-> BR: RPC + {} -> 0x{:x}", sximm(op, 9), self.cpu.rpc + sximm(op, 9));
            self.cpu.rpc += sximm(op, 9);
        }  
    }

    // Execute TRAP
    fn trap(&mut self, op: u16) {
        let trapvec = sximm(op, 7);
        match trapvec {
            TGETC => {
                let input: Option<u16> = std::io::stdin()
                .bytes() 
                .next()
                .and_then(|result| result.ok())
                .map(|byte| byte as u16);

                self.cpu.r[0] = input.unwrap();
            },
            TOUT => println!("{}", ((self.cpu.r[0] & 0xFF) as u8) as char),
            TPUTS => println!("Not Implemented"),
            TIN => {
                let input: Option<u16> = std::io::stdin()
                .bytes() 
                .next()
                .and_then(|result| result.ok())
                .map(|byte| byte as u16);

                self.cpu.r[0] = input.unwrap();
                println!("{}", ((self.cpu.r[0] & 0xFF) as u8) as char);
            },
            TPUTSP => println!("Not Implemented"),
            THALT => {
                println!("-> THALT");
                self.stop_flag = true;
            },
            TINU16 => {
                let input: Option<u16> = std::io::stdin()
                .bytes() 
                .next()
                .and_then(|result| result.ok())
                .map(|byte| byte as u16);

                self.cpu.r[0] = input.unwrap();
            },
            TOUTU16 => println!("{}", self.cpu.r[0]),
            _ => {
                println!("!> Unimplemented trap vector: 0x{:x}", trapvec);
                self.stop_flag = true;
            }
        }
    }

    fn load_program(&mut self) {
        // for now this is a placeholder function
        self.cpu.r[0] = 0x41;

        self.mem_write(PC_BASE, 0xF020); //TRAP TGETC
        self.mem_write(PC_BASE + 1, 0xF027); // TRAP TOUTU16
        self.mem_write(PC_BASE + 2, 0xF025); // TRAP HALT
    }

    fn start(&mut self) {
        // enter main loop
        while self.stop_flag == false {
            // fetch instruction from memory and update PC
            let op = self.mem_read(self.cpu.rpc);
            self.cpu.rpc += 1;

            // decode instruction
            let instr = op >> 12;

            // execute instruction
            match instr {
                ADD => self.add(op),
                AND => self.and(op),
                LD => self.ld(op),
                LDI => self.ldi(op),
                LDR => self.ldr(op),
                LEA => self.lea(op),
                NOT => self.not(op),
                ST => self.st(op),
                STI => self.sti(op),
                STR => self.str(op),
                JMP => self.jmp(op),
                JSR => self.jsr(op),
                BR => self.br(op),
                TRAP => self.trap(op),
                _ => {
                    println!("!> Unimplemented instruction: 0x{:x}", instr);
                    self.stop_flag = true;
                }
            }
        }
    }

    fn dump_state(&self, mem_address: usize, num_bytes: usize) {
        println!("===========register=========");
        println!("r0 => 0x{:04x}  r4 => 0x{:04x} ", self.cpu.r[0], self.cpu.r[4]);
        println!("r1 => 0x{:04x}  r5 => 0x{:04x} ", self.cpu.r[1], self.cpu.r[5]);
        println!("r2 => 0x{:04x}  r6 => 0x{:04x} ", self.cpu.r[2], self.cpu.r[6]);
        println!("r3 => 0x{:04x}  r7 => 0x{:04x} ", self.cpu.r[3], self.cpu.r[7]);
        println!("==========control===========");
        println!("rpc  => 0x{:04x}", self.cpu.rpc);
        println!("rcnd => {:03b}", self.cpu.rcnd);
        println!("===========memory===========");

        //implement memory dump
        let num_bytes = (num_bytes + 0x8) & !0x8;

        for shift in 0..num_bytes {
            if shift != 0 && shift % 4 == 0 { println!()}
            print!("{:04x} ", self.memory[mem_address + shift]);
        }
        println!();
        println!("============================");
    }

    fn mem_write(&mut self, address: u16, value: u16) {
        self.memory[address as usize] = value;

    }

    fn mem_read(&self, address: u16) -> u16 {
        self.memory[address as usize]
    }
}

impl Cpu {
    fn new(base: u16) -> Self {
        Cpu { 
            r: [0; 8], 
            rpc: base, 
            rcnd: 0 
        }
    }
}

struct Cpu {
    r: [u16; 8],
    rpc: u16,
    rcnd: u16,
}

fn main() {
    let mut vm = VM::new();
    vm.load_program();
    vm.start();

    vm.dump_state(PC_BASE as usize, 0x30);
}
