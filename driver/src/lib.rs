#![no_std]
#![feature(lang_items)]
#![feature(let_else)]
#![feature(llvm_asm)]
#![feature(decl_macro)]
#![feature(const_deref)]
#![feature(const_mut_refs)]
#![feature(const_ptr_as_ref)]
#![feature(const_trait_impl)]
#![feature(alloc_error_handler)]
#![feature(new_uninit)]
#![feature(allocator_api)]
#![feature(box_syntax)]
#![allow(clippy::new_ret_no_self)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;

#[macro_use] extern crate static_assertions;

use crate::{
    debug::dbg_break,
    hook::{handlers, testing, Hook, HookType},
    svm::Hypervisor,
    utils::{
        logger::KernelLogger,
        nt::{KeBugCheck, MANUALLY_INITIATED_CRASH},
        physmem_descriptor::PhysicalMemoryDescriptor,
    },
};
use alloc::{vec, vec::Vec};
use log::LevelFilter;
use winapi::{
    km::wdm::DRIVER_OBJECT,
    shared::{
        ntdef::{NTSTATUS, PVOID},
        ntstatus::*,
    },
};

pub mod debug;
pub mod hook;
pub mod lang;
pub mod support;
pub mod svm;
pub mod utils;
pub mod vm_test;

#[global_allocator]
static GLOBAL: utils::alloc::KernelAlloc = utils::alloc::KernelAlloc;
static LOGGER: KernelLogger = KernelLogger;

static mut HYPERVISOR: Option<Hypervisor> = None;

fn init_hooks() -> Option<Vec<Hook>> {
    macro save_hook($hook:expr, $global_hook:expr) {
        if let HookType::Function { ref inline_hook } = $hook.hook_type {
            unsafe { $global_hook = Some(core::mem::transmute(inline_hook.trampoline_address())) };
        }
    }

    // ZwQuerySystemInformation
    //
    let zwqsi_hook = Hook::hook_function(
        "ZwQuerySystemInformation",
        handlers::zw_query_system_information as *const (),
    )?;
    save_hook!(zwqsi_hook, handlers::ZWQSI_ORIGINAL);

    // // ExAllocatePoolWithTag
    // //
    // let eapwt_hook = Hook::hook_function(
    //     "ExAllocatePoolWithTag",
    //     handlers::ex_allocate_pool_with_tag as *const (),
    // )?;
    // unsafe {
    //     handlers::EAPWT_ORIGINAL = match eapwt_hook.hook_type {
    //         HookType::Function { ref inline_hook } =>
    // Pointer::new(inline_hook.as_ptr()),         HookType::Page =>
    // unreachable!(),     };
    // }

    // // MmIsAddressValid
    // //
    // let mmiav_hook = Hook::hook_function(
    //     "MmIsAddressValid",
    //     handlers::mm_is_address_valid as *const (),
    // )?;
    // unsafe {
    //     handlers::MMIAV_ORIGINAL = match mmiav_hook.hook_type {
    //         HookType::Function { ref inline_hook } =>
    // Pointer::new(inline_hook.as_ptr()),         HookType::Page =>
    // unreachable!(),     };
    // }
    //
    // let hook = Hook::hook_function_ptr(
    //     unsafe { testing::SHELLCODE_PA.as_ref().unwrap().va() as u64 },
    //     testing::hook_handler as *const (),
    // )?;

    // FIXME: Currently only 1 hook is supported
    // TODO: Check if this is true
    // TODO: Use once_cell for this

    Some(vec![zwqsi_hook])
}

fn virtualize_system() -> Option<()> {
    let hooks = init_hooks()?;
    let Some(mut hv) = Hypervisor::new(hooks) else {
        log::info!("Failed to create processors");
        return None;
    };

    if !hv.virtualize() {
        log::error!("Failed to virtualize processors");
    }

    // Save the processors for later use
    //
    unsafe { HYPERVISOR = Some(hv) };

    Some(())
}

#[cfg(not(feature = "mmap"))]
pub extern "system" fn driver_unload(_driver: &mut DRIVER_OBJECT) {
    // Devirtualize all processors and drop the global struct.
    //
    if let Some(mut hv) = unsafe { HYPERVISOR.take() } {
        hv.devirtualize();

        core::mem::drop(hv);
    }
}

#[no_mangle]
pub extern "system" fn DriverEntry(driver: *mut DRIVER_OBJECT, _path: PVOID) -> NTSTATUS {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info));

    log::info!("Hello from amd_hypervisor!");

    dbg_break!();

    // vm_test::check_all();
    // unsafe { (*driver).DriverUnload = Some(driver_unload) };
    // return STATUS_SUCCESS;

    // Initialize the hook testing
    //

    testing::init();
    testing::print_shellcode();
    testing::call_shellcode();
    testing::print_shellcode();

    // Print physical memory pages
    //

    let desc = PhysicalMemoryDescriptor::new();

    log::info!("Physical memory descriptors: {:x?}", desc);
    log::info!("Found {:#x?} pages", desc.page_count());
    log::info!("Found {}gb of physical memory", desc.total_size_in_gb());

    // Virtualize the system
    //
    cfg_if::cfg_if! {
        if #[cfg(feature = "mmap")] {
            let _ = driver;

            extern "system" fn system_thread(_context: *mut u64) {
                log::info!("System thread started");

                virtualize_system();
            }

            let mut handle = core::mem::MaybeUninit::uninit();
            unsafe {
                crate::utils::nt::PsCreateSystemThread(
                    handle.as_mut_ptr() as _,
                    winapi::km::wdm::GENERIC_ALL,
                    0 as _,
                    0 as _,
                    0 as _,
                    system_thread as *const (),
                    0 as _,
                )
            };

            STATUS_SUCCESS
        } else {
            log::info!("Registering driver unload routine");
            #[allow(clippy::not_unsafe_ptr_arg_deref)]
            unsafe { (*driver).DriverUnload = Some(driver_unload) };

            let status = if virtualize_system().is_some() {
                STATUS_SUCCESS
            } else {
                STATUS_UNSUCCESSFUL
            };

            // Call the hook again after initialization
            //
            testing::print_shellcode();
            testing::call_shellcode();
            testing::print_shellcode();

            handlers::test_hooks();

            vm_test::check_all();

            status
        }
    }
}
