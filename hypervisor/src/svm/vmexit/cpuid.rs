use crate::svm::{
    data::{guest::GuestRegisters, processor_data::ProcessorData},
    vmexit::ExitType,
};
use x86::cpuid::cpuid;

pub fn handle_default(_data: &mut ProcessorData, guest_regs: &mut GuestRegisters) -> ExitType {
    let leaf = guest_regs.rax;
    let subleaf = guest_regs.rcx;

    let cpuid = cpuid!(leaf, subleaf);

    guest_regs.rax = cpuid.eax as u64;
    guest_regs.rbx = cpuid.ebx as u64;
    guest_regs.rcx = cpuid.ecx as u64;
    guest_regs.rdx = cpuid.edx as u64;

    ExitType::IncrementRIP
}

pub const CPUID_DEVIRTUALIZE: u32 = 0x4321_1234;
pub(crate) fn handle_devirtualize(_: &mut ProcessorData, _: &mut GuestRegisters) -> ExitType {
    ExitType::ExitHypervisor
}
