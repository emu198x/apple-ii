//! Hardware-specific primitives; shared shell owns all debug verbs.
use crate::AppleIIRuntime;
use emu198x_shell::DebugPrimitives;
use serde_json::{Value, json};
impl DebugPrimitives for AppleIIRuntime {
    fn dbg_pc(&self) -> u32 {
        u32::from(self.machine.cpu().regs.pc)
    }
    fn dbg_peek(&self, addr: u32) -> u8 {
        u16::try_from(addr).map_or(255, |addr| self.machine.peek(addr))
    }
    fn dbg_poke(&mut self, addr: u32, value: u8) {
        if let Ok(addr) = u16::try_from(addr) {
            self.machine.poke(addr, value);
        }
    }
    fn dbg_cpu_state(&self) -> Value {
        let r = &self.machine.cpu().regs;
        json!({"a":format!("${:02X}",r.a),"x":format!("${:02X}",r.x),"y":format!("${:02X}",r.y),"sp":format!("${:02X}",r.sp),"pc":format!("${:04X}",r.pc),"p":format!("${:02X}",r.p),"halted":self.machine.cpu().halted})
    }
    fn dbg_instruction_boundary_count(&self) -> Option<u64> {
        Some(self.machine.instruction_boundary_count())
    }
    fn dbg_disassemble(&self, addr: u32) -> Option<(String, u8)> {
        let addr = u16::try_from(addr).ok()?;
        emu198x_shell::isa_disasm::decode_one_6502(addr, |a| self.machine.peek(a))
    }
    fn dbg_step(&mut self) -> u64 {
        let ticks = self.dbg_step_no_resync();
        self.update_rgba_framebuffer();
        ticks
    }
    fn dbg_step_no_resync(&mut self) -> u64 {
        if self.machine.cpu().halted {
            return 0;
        }
        let start = self.machine.master_clock();
        let boundary = self.machine.instruction_boundary_count();
        for _ in 0..4096 {
            self.tick();
            if self.machine.instruction_boundary_count() != boundary {
                break;
            }
        }
        self.machine.master_clock() - start
    }
    fn dbg_resync(&mut self) {
        self.update_rgba_framebuffer();
    }
}
