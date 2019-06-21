extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal, TokenTree};
use quote::quote;

fn get_single_arg(item: TokenStream) -> Ident {
    let item = proc_macro2::TokenStream::from(item);
    let arg = item.into_iter().next();
    let ident = if let Some(TokenTree::Ident(ident)) = arg {
        ident
    } else {
        panic!(
            "k_*_define takes one identifier argument. Got {:?}",
            arg
        );
    };
    ident
}

#[proc_macro]
pub fn k_mutex_define(item: TokenStream) -> TokenStream {
    let ident = get_single_arg(item);
    let section = Literal::string(&format!("._k_mutex.static.{}", ident));
    let ctor = Ident::new(&format!("_rust_mutex_init_{}", ident), ident.span());
    let ctor_ptr = Ident::new(&format!("_ctor_rust_mutex_init_{}", ident), ident.span());
    let expanded = quote! {
        // The static storage for the object, itself
        #[link_section = #section]
        static #ident: zephyr::mutex::global::k_mutex = unsafe { zephyr::mutex::global::k_mutex::uninit() };

        // A constructor function that calls its init
        #[allow(non_snake_case)]
        extern "C" fn #ctor() {
            use zephyr::mutex::RawMutex;
            unsafe { #ident.init::<zephyr::context::Kernel>() }
        }

        // Add a pointer to the constructor to .ctors table
        #[used]
        #[link_section = ".ctors"]
        #[allow(non_upper_case_globals)]
        static #ctor_ptr: extern "C" fn() = #ctor;
    };

    expanded.into()
}

#[proc_macro]
pub fn k_poll_signal_define(item: TokenStream) -> TokenStream {
    let ident = get_single_arg(item);
    // Using mutex section because there is not one for poll_signal. Need to
    // ensure this is in kernel memory.
    let section = Literal::string(&format!("._k_mutex.static.{}", ident));
    let ctor = Ident::new(&format!("_rust_poll_signal_init_{}", ident), ident.span());
    let ctor_ptr = Ident::new(&format!("_ctor_rust_poll_signal_init_{}", ident), ident.span());
    let expanded = quote! {
        // The static storage for the object, itself
        #[link_section = #section]
        static #ident: zephyr::poll::global::k_poll_signal = unsafe { zephyr::poll::global::k_poll_signal::uninit() };

        // A constructor function that calls its init
        #[allow(non_snake_case)]
        extern "C" fn #ctor() {
            use zephyr::poll::*;
            unsafe { #ident.init::<zephyr::context::Kernel>() }
        }

        // Add a pointer to the constructor to .ctors table
        #[used]
        #[link_section = ".ctors"]
        #[allow(non_upper_case_globals)]
        static #ctor_ptr: extern "C" fn() = #ctor;
    };

    expanded.into()
}

fn get_sem_args(item: TokenStream) -> Option<(Ident, Literal, Literal)> {
    let item = proc_macro2::TokenStream::from(item);
    let mut iter = item.into_iter();

    if let (
        Some(TokenTree::Ident(ident)),
        Some(TokenTree::Punct(p1)),
        Some(TokenTree::Literal(l1)),
        Some(TokenTree::Punct(p2)),
        Some(TokenTree::Literal(l2)),
    ) = (
        iter.next(),
        iter.next(),
        iter.next(),
        iter.next(),
        iter.next(),
    ) {
        if p1.as_char() == ',' && p2.as_char() == ',' {
            return Some((ident, l1, l2));
        }
    }
    None
}

#[proc_macro]
pub fn k_sem_define(item: TokenStream) -> TokenStream {
    let (ident, initial, limit) = get_sem_args(item).expect("Expected 3 comma-separated arguments");

    let section = Literal::string(&format!("._k_sem.static.{}", ident));
    let ctor = Ident::new(&format!("_rust_sem_init_{}", ident), ident.span());
    let ctor_ptr = Ident::new(&format!("_ctor_rust_sem_init_{}", ident), ident.span());
    let expanded = quote! {
        // The static storage for the object, itself
        #[link_section = #section]
        static #ident: zephyr::semaphore::global::k_sem = unsafe { zephyr::semaphore::global::k_sem::uninit() };

        // A constructor function that calls its init
        #[allow(non_snake_case)]
        extern "C" fn #ctor() {
            use zephyr::semaphore::*;
            unsafe { #ident.init::<zephyr::context::Kernel>(#initial, #limit) }
        }

        // Add a pointer to the constructor to .ctors table
        #[used]
        #[link_section = ".ctors"]
        #[allow(non_upper_case_globals)]
        static #ctor_ptr: extern "C" fn() = #ctor;
    };

    expanded.into()
}
