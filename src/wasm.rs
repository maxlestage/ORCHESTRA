//! Le compilateur dans le navigateur : `orchestra.wasm`, sans dépendance ni glue
//! générée. Le JavaScript du bac à sable écrit le texte dans la mémoire du module
//! (`alloc`), appelle `render`, puis lit le HTML produit (`out_ptr`, `out_len`).
//!
//!     cargo build --lib --target wasm32-unknown-unknown --profile wasm

use std::cell::RefCell;

thread_local! {
    static OUT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Réserve `len` octets dans la mémoire du module ; `render` les libère.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len].into_boxed_slice()).cast()
}

/// Compile le texte UTF-8 réservé par `alloc(len)` et garde le HTML des résultats.
///
/// # Safety
///
/// `ptr` et `len` doivent venir d'un même appel à [`alloc`], et n'être passés qu'une fois.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn render(ptr: *mut u8, len: usize) {
    // SAFETY : la tranche a été créée par `alloc(len)` et n'est rendue qu'une fois.
    let bytes = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) };
    let source = String::from_utf8_lossy(&bytes);
    let html = crate::render::results(&source, &crate::render::compile(&source));
    OUT.with(|out| *out.borrow_mut() = html);
}

/// L'adresse du dernier HTML produit.
#[unsafe(no_mangle)]
pub extern "C" fn out_ptr() -> *const u8 {
    OUT.with(|out| out.borrow().as_ptr())
}

/// La longueur, en octets, du dernier HTML produit.
#[unsafe(no_mangle)]
pub extern "C" fn out_len() -> usize {
    OUT.with(|out| out.borrow().len())
}
