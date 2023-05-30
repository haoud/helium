//! This crate contains the macros used by the kernel. These macros are used to simplify the
//! code, to reduce the boilerplate code and to make the code more readable (except for this
//! file, which is not very readable because macros programming is not very readable).
//!
//! I'm not used to write procedural macros in  Rust (in fact, this is the first time) so the
//! code is probably not the best and may not work in 100% of the cases.
use proc_macro::TokenStream;
use syn::{parse_macro_input, AttributeArgs, ItemFn, ItemStatic};

/// A macro to indicate that a function is only used during the initialization of the kernel.
/// This macro will this attribute are put in a separate .init section. When the kernel has been
/// initialized, this section will be discarded and the memory will be freed, allowing the kernel
/// to reduce its memory footprint.
///
/// # Safety
/// If an function with this attribute is called after the kernel has been initialized, the
/// behavior is undefined.
#[proc_macro_attribute]
pub fn init(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);
    input_fn
        .attrs
        .push(syn::parse_quote!(#[link_section = ".init"]));

    TokenStream::from(quote::quote!(
        #input_fn
    ))
}

/// A macro to indicate that a function is an interrupt handler. This macro will automatically
/// add the necessary code to the function to save the state of the CPU before calling the handler
/// and to restore the state of the CPU after the handler has been called.
///
/// This function takes one argument, which is the data that will be pushed to the stack before
/// saving the state of the CPU (the `code` field of the `x86_64::cpu::State` struct).
#[proc_macro_attribute]
pub fn interrupt(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let mut func = parse_macro_input!(item as ItemFn);

    let func_name = func.sig.ident.clone();
    let handler = syn::Ident::new(&format!("{}_interruption", func_name), func_name.span());

    // Change the function name to a new name so we can create a new function with the same name
    // We make sure that the function ABI is set to C, so we can have an stable ABI
    func.sig.abi = Some(syn::parse_quote!(extern "C"));
    func.sig.ident = handler.clone();

    let data = &args[0];

    quote::quote! {
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn #func_name() {
            core::arch::asm!("
                push {data}
                call interrupt_enter
                call {handler}
                jmp interrupt_exit",
                data = const #data,
                handler = sym #handler,
                options(noreturn)
            );
        }

        #func
    }
    .into()
}

/// This macro work the same as the [`interrupt`] macro, but it does not take any arguments, because
/// it assume that the exception pushed an error code to the stack. This is useful for exceptions
/// that push an error code to the stack, like the page fault exception.
#[proc_macro_attribute]
pub fn exception_err(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);

    let func_name = func.sig.ident.clone();
    let handler = syn::Ident::new(&format!("{}_exception", func_name), func_name.span());

    // Change the function name to a new name so we can create a new function with the same name
    // We make sure that the function ABI is set to C, so we can have an stable ABI
    func.sig.abi = Some(syn::parse_quote!(extern "C"));
    func.sig.ident = handler.clone();

    quote::quote! {
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn #func_name() {
            core::arch::asm!("
                call interrupt_enter
                call {handler}
                jmp interrupt_exit",
                handler = sym #handler,
                options(noreturn)
            );
        }

        #func
    }
    .into()
}

/// This macro work the same as the [`interrupt`] macro, but it does not take any arguments because
/// it automatically pushes 0 to the stack before calling the handler. This is useful for exceptions
/// that does not push any error code to the stack, and in which a custom code would not make sense.
#[proc_macro_attribute]
pub fn exception(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);

    let func_name = func.sig.ident.clone();
    let handler = syn::Ident::new(&format!("{}_exception", func_name), func_name.span());

    // Change the function name to a new name so we can create a new function with the same name
    // We make sure that the function ABI is set to C, so we can have an stable ABI
    func.sig.abi = Some(syn::parse_quote!(extern "C"));
    func.sig.ident = handler.clone();

    quote::quote! {
        #[no_mangle]
        #[naked]
        unsafe extern "C" fn #func_name() {
            core::arch::asm!("
                push 0
                call interrupt_enter
                call {handler}
                jmp interrupt_exit",
                handler = sym #handler,
                options(noreturn)
            );
        }

        #func
    }
    .into()
}

/// This macro is used to signal that a function is an IRQ handler. This macro will automatically
/// create 16 different functions, one for each IRQ, with name `irq_0`, `irq_1`, etc. These
/// functions will call the original function after the usual interrupt handling code (saving the
/// state of the CPU, swapping user GS with kernel GS, etc.)
#[proc_macro_attribute]
pub fn irq_handler(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);
    let handler = func.sig.ident.clone();

    // Change the function name to a new name so we can create a new function with the same name
    // We make sure that the function ABI is set to C, so we can have an stable ABI
    func.sig.abi = Some(syn::parse_quote!(extern "C"));
    let mut functions = Vec::new();

    for irq in 0..16 {
        let func_name = syn::Ident::new(&format!("irq_{}", irq), func.sig.ident.clone().span());

        functions.push(quote::quote! {
            #[no_mangle]
            #[naked]
            unsafe extern "C" fn #func_name() {
                core::arch::asm!("
                    push {irq}
                    call interrupt_enter
                    call {handler}
                    jmp interrupt_exit",
                    irq = const #irq,
                    handler = sym #handler,
                    options(noreturn)
                );
            }
        });
    }

    TokenStream::from(quote::quote!(
        #(#functions)*
        #func
    ))
}

/// A macro that can be used on static variables to make them per-CPU. This macro will wrap the
/// variable in a [`PerCpu`] struct, which will allow each CPU to have its own copy of the
/// variable. For more information, see the [`PerCpu`] documentation.
///
/// # Important
/// To use this macro, you must have the `helium-x86_64` crate in your dependencies, named
/// `x86_64`. If you use this macro directly in the `helium-x86_64` crate, you must add
/// `use crate as x86_64;` at the top of your file in order to make the macro compile.
///
/// # Example
/// ```rust
/// #[per_cpu]
/// pub static COUNTER: AtomicU64 = AtomicU64::new(0);
///
/// fn main() {
///     COUNTER.local().fetch_add(1, Ordering::SeqCst);
/// }
/// ```
#[proc_macro_attribute]
pub fn per_cpu(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut var = parse_macro_input!(item as ItemStatic);

    let old_type = var.ty.clone();
    let old_init = var.expr.clone();
    let new_type = syn::parse_quote!(x86_64::percpu::PerCpu<#old_type>);
    let new_init = syn::parse_quote!(x86_64::percpu::PerCpu::new(#old_init));

    var.ty = Box::new(new_type);
    var.expr = Box::new(new_init);
    var.attrs
        .push(syn::parse_quote!(#[link_section = ".percpu"]));

    TokenStream::from(quote::quote!(#var))
}

// Mark a function as a syscall handler. This macro will change the function name to
// `syscall_handler`, and will make sure that the function is exported with the C ABI.
// This macro should only be used on one function in the entire kernel. Multiple functions
// with this macro will cause the linker to fail with a duplicate symbol error.
// TODO: More check to the function (arguments, return type, etc.)
#[proc_macro_attribute]
pub fn syscall_handler(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut var = parse_macro_input!(item as ItemFn);
    var.attrs.push(syn::parse_quote!(#[no_mangle]));
    var.sig.abi = Some(syn::parse_quote!(extern "C"));
    var.sig.ident = syn::Ident::new("syscall_handler", var.sig.ident.span());

    TokenStream::from(quote::quote!(#var))
}
