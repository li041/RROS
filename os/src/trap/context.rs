use core::arch::asm;

use alloc::vec::Vec;
use riscv::{
    asm,
    register::sstatus::{self, Sstatus, SPP},
};

use crate::{config::PAGE_SIZE_BITS, console::print, mm::frame_allocator::frame_alloc};

use crate::mm::page_table::map_temp;
/// Trap Context
/// 内核栈对齐到16字节
#[repr(C)]
#[repr(align(16))]
pub struct TrapContext {
    /// general regs[0..31], x[10]是a0, x[4]是tp, x[1]是ra, x[2]是sp
    pub x: [usize; 32],
    /// CSR sstatus      
    pub sstatus: Sstatus,
    /// CSR sepc
    pub sepc: usize,
}

impl TrapContext {
    /// set stack pointer to x_2 reg (sp)
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    pub fn set_tp(&mut self, tp: usize) {
        self.x[4] = tp;
    }
    /// init app context
    pub fn app_init_trap_context(entry: usize, ustack_top: usize) -> Self {
        let mut sstatus = sstatus::read(); // CSR sstatus
        sstatus.set_spp(SPP::User); //previous privilege mode: user mode
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // entry point of app
        };
        cx.set_sp(ustack_top); // app's user stack pointer
        cx // return initial Trap Context of app
    }
}

// #[cfg(feature = "test")]
#[allow(unused)]
pub fn trap_cx_test() {
    let va: usize = 0xffff_ffff_ffff_f000;
    let frame = frame_alloc().unwrap().ppn;
    let vpn = (va >> PAGE_SIZE_BITS).into();
    let write_addr = va as *mut TrapContext;
    map_temp(vpn, frame);
    let regs = (0..32).collect::<Vec<usize>>().try_into().unwrap();
    let mut sstatus = sstatus::read();
    sstatus.set_spp(SPP::User);
    let cx = TrapContext {
        x: regs,
        sstatus: sstatus,
        sepc: 0x114,
    };
    unsafe {
        // write_addr.write(cx);
        *write_addr = cx;
    }
    let read_addr: u64 = 0xffff_ffff_ffff_f000;
    let mut read_cx: TrapContext = TrapContext::app_init_trap_context(0, 0);
    let mut read_cx_sstatus: usize;
    // 在这里使用汇编代码读取内核栈的值
    unsafe {
        // 把read_addr的值加载进t0
        asm!(
            "mv t0, {0}",
            in(reg) read_addr,
            options(nostack)
        );

        for i in 0..32 {
            // 读取x[0..31]的值
            asm!(
                "ld {0}, (t0)",
                "addi t0, t0, 8",
                out(reg) read_cx.x[i],
                options(nostack)
            );
        }
        asm!(
            "ld {0}, (t0)",
            "addi t0, t0, 8",
            out(reg) read_cx_sstatus,
            options(nostack)
        );
        asm!(
            "ld {0}, (t0)",
            "addi t0, t0, 8",
            out(reg) read_cx.sepc,
            options(nostack)
        );
    }
    assert!(read_cx.x == regs);
    assert!(read_cx.sstatus.bits() == read_cx_sstatus);
    assert!(read_cx.sepc == 0x114);
    println!("trap_cx_test pass!");
}
